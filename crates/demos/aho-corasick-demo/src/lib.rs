mod automaton;
mod layout;

use automaton::Automaton;
use wasm_bindgen::prelude::*;

/// Text longer than this is silently truncated at construction; the demo
/// page states the cap rather than surfacing a truncation error. Counted in
/// bytes, not chars: `Automaton` only ever matches ASCII letters (see
/// `automaton::fold`) and treats every other byte, including each byte of
/// a multi-byte UTF-8 character, as a word-boundary reset, so a byte cap
/// and a char cap coincide for the ASCII text this demo targets.
const MAX_TEXT_LEN: usize = 200;

/// Converts an automaton match — a `(pattern_index, end)` pair where `end`
/// is the position (in the folded pattern's own units, which is bytes,
/// counted from 1) immediately after the match — into a text byte range
/// `[start, end)`. Pulled out as a pure function because an off-by-one here
/// produces visually wrong highlights that no automaton test would catch:
/// see the folded-length tests below.
fn match_range(end: usize, folded_len: usize) -> (u32, u32) {
    let start = end - folded_len;
    (start as u32, end as u32)
}

/// The automaton, its static layout, and a streaming cursor over `text`,
/// flattened for JS.
///
/// `automaton::Cursor<'a>` borrows the `Automaton` it walks, which cannot
/// live alongside it inside a `#[wasm_bindgen]` struct (no lifetimes at the
/// JS boundary). `Visualizer` instead holds `state`/`pos` as plain fields
/// and drives the automaton itself via `Automaton::advance`, the same
/// failure-hop stepping method `Cursor::step` delegates to — so the logic
/// exists exactly once.
#[wasm_bindgen]
pub struct Visualizer {
    automaton: Automaton,

    xs: Vec<f32>,
    ys: Vec<f32>,
    labels: Vec<u8>,
    parents: Vec<u32>,
    fails: Vec<u32>,
    terminal: Vec<u8>,

    /// Folded length of each pattern, indexed by pattern index — the trie
    /// strips non-alphabetic bytes and lowercases at build time, so this is
    /// *not* the raw input pattern's length. Needed to convert `(pattern
    /// index, end)` matches into text byte ranges.
    pattern_lens: Vec<usize>,

    text: Vec<u8>,
    state: usize,
    pos: usize,

    hops: Vec<u32>,
    match_starts: Vec<u32>,
    match_ends: Vec<u32>,
}

#[wasm_bindgen]
impl Visualizer {
    /// Builds the automaton from comma-separated `patterns` and stores
    /// `text` (truncated to `MAX_TEXT_LEN` bytes) for streaming. Build
    /// errors — too many patterns, a pattern with no letters, a pattern
    /// too long — surface as a JS exception carrying a readable message.
    #[wasm_bindgen(constructor)]
    pub fn new(patterns: &str, text: &str) -> Result<Visualizer, JsValue> {
        let raw: Vec<&str> = patterns.split(',').collect();
        let automaton = Automaton::build(&raw).map_err(|e| JsValue::from_str(&e.to_string()))?;
        let pattern_lens: Vec<usize> = raw.iter().map(|p| automaton::fold(p).len()).collect();

        let positions = layout::layout(&automaton);
        let node_count = automaton.node_count();
        let mut xs = Vec::with_capacity(node_count);
        let mut ys = Vec::with_capacity(node_count);
        let mut labels = Vec::with_capacity(node_count);
        let mut parents = Vec::with_capacity(node_count);
        let mut fails = Vec::with_capacity(node_count);
        let mut terminal = Vec::with_capacity(node_count);
        for (s, &(x, y)) in positions.iter().enumerate() {
            xs.push(x);
            ys.push(y);
            labels.push(automaton.label(s));
            parents.push(automaton.parent(s) as u32);
            fails.push(automaton.fail(s) as u32);
            terminal.push(u8::from(!automaton.outputs(s).is_empty()));
        }

        let mut text: Vec<u8> = text.bytes().collect();
        text.truncate(MAX_TEXT_LEN);

        Ok(Visualizer {
            automaton,
            xs,
            ys,
            labels,
            parents,
            fails,
            terminal,
            pattern_lens,
            text,
            state: 0,
            pos: 0,
            hops: Vec::new(),
            match_starts: Vec::new(),
            match_ends: Vec::new(),
        })
    }

    /// Number of states in the automaton; every `*_ptr` array below is
    /// exactly this long.
    pub fn node_count(&self) -> usize {
        self.automaton.node_count()
    }

    /// Pointer to the per-state x coordinates (unit-free tidy-tree layout).
    ///
    /// The caller MUST rebuild its typed-array view from this pointer on
    /// every read and never cache it: wasm memory growth silently detaches
    /// the backing `ArrayBuffer`, and a retained view then reads garbage or
    /// throws.
    pub fn xs_ptr(&self) -> *const f32 {
        self.xs.as_ptr()
    }

    /// Pointer to the per-state y coordinates. Same rebuild-every-read
    /// contract as [`Self::xs_ptr`].
    pub fn ys_ptr(&self) -> *const f32 {
        self.ys.as_ptr()
    }

    /// Pointer to the per-state trie-edge label byte (0 for the root). Same
    /// rebuild-every-read contract as [`Self::xs_ptr`].
    pub fn labels_ptr(&self) -> *const u8 {
        self.labels.as_ptr()
    }

    /// Pointer to each state's parent index in the trie. Same
    /// rebuild-every-read contract as [`Self::xs_ptr`].
    pub fn parents_ptr(&self) -> *const u32 {
        self.parents.as_ptr()
    }

    /// Pointer to each state's failure-link target. Same rebuild-every-read
    /// contract as [`Self::xs_ptr`].
    pub fn fails_ptr(&self) -> *const u32 {
        self.fails.as_ptr()
    }

    /// Pointer to each state's terminal flag: 1 if the state has one or
    /// more pattern outputs, 0 otherwise. Same rebuild-every-read contract
    /// as [`Self::xs_ptr`].
    pub fn terminal_ptr(&self) -> *const u8 {
        self.terminal.as_ptr()
    }

    /// Length in bytes of the stored text (after the `MAX_TEXT_LEN` cap).
    pub fn text_len(&self) -> usize {
        self.text.len()
    }

    /// Rewinds the stream to the beginning without rebuilding the
    /// automaton: state and position reset to 0, and the last step's
    /// events are cleared.
    pub fn reset(&mut self) {
        self.state = 0;
        self.pos = 0;
        self.hops.clear();
        self.match_starts.clear();
        self.match_ends.clear();
    }

    /// Advances the cursor by one byte of the stored text. Returns `false`
    /// once the text is exhausted (the cursor does not advance further);
    /// `true` otherwise, with `current_state`, `hops_ptr`/`hops_len`, and
    /// `match_*` refreshed for the step just taken.
    pub fn step(&mut self) -> bool {
        if self.pos >= self.text.len() {
            return false;
        }
        let byte = self.text[self.pos];
        self.pos += 1;
        let (hops, state) = self.automaton.advance(self.state, byte);
        self.state = state;

        self.hops = hops.into_iter().map(|h| h as u32).collect();

        self.match_starts.clear();
        self.match_ends.clear();
        for &pattern_index in self.automaton.outputs(state) {
            let (start, end) = match_range(self.pos, self.pattern_lens[pattern_index]);
            self.match_starts.push(start);
            self.match_ends.push(end);
        }
        true
    }

    /// Bytes of the stored text consumed so far (0 after `reset`).
    pub fn pos(&self) -> usize {
        self.pos
    }

    /// The automaton state the cursor currently sits on.
    pub fn current_state(&self) -> usize {
        self.state
    }

    /// Pointer to the failure hops taken during the most recent `step`
    /// (empty before the first `step` or right after `reset`).
    ///
    /// This buffer is rewritten on every `step` call, so the rebuild-every-
    /// read rule from [`Self::xs_ptr`] applies even more strictly here: a
    /// view taken after one `step` is invalid by the next.
    pub fn hops_ptr(&self) -> *const u32 {
        self.hops.as_ptr()
    }

    /// Length of the buffer at `hops_ptr`.
    pub fn hops_len(&self) -> usize {
        self.hops.len()
    }

    /// Pointer to the inclusive start (in text bytes) of each match found
    /// on the most recent `step`. Same per-step rebuild contract as
    /// [`Self::hops_ptr`].
    pub fn match_starts_ptr(&self) -> *const u32 {
        self.match_starts.as_ptr()
    }

    /// Pointer to the exclusive end (in text bytes) of each match found on
    /// the most recent `step`, parallel to `match_starts_ptr`. Same
    /// per-step rebuild contract as [`Self::hops_ptr`].
    pub fn match_ends_ptr(&self) -> *const u32 {
        self.match_ends.as_ptr()
    }

    /// Length of the buffers at `match_starts_ptr` and `match_ends_ptr`.
    pub fn match_len(&self) -> usize {
        self.match_starts.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn match_range_uses_the_folded_length_not_the_raw_pattern_length() {
        // "h3-e" folds to "he" (folded length 2, not the raw pattern's 4):
        // a match ending at text byte 5 must start at 3, not 1.
        assert_eq!(match_range(5, 2), (3, 5));
    }

    #[test]
    fn visualizer_converts_matches_to_text_ranges_using_folded_lengths() {
        // Textbook patterns, with "he" spelled as "h3-e" to pin the folded-
        // length conversion against a pattern whose raw length (4) differs
        // from its folded length (2).
        let mut v = Visualizer::new("h3-e,she,his,hers", "ushers").unwrap();
        let mut ranges: Vec<(u32, u32)> = Vec::new();
        while v.step() {
            let starts = unsafe { std::slice::from_raw_parts(v.match_starts_ptr(), v.match_len()) };
            let ends = unsafe { std::slice::from_raw_parts(v.match_ends_ptr(), v.match_len()) };
            ranges.extend(starts.iter().copied().zip(ends.iter().copied()));
        }
        // she: text[1..4]; he (folded from "h3-e"): text[2..4]; hers: text[2..6].
        assert_eq!(ranges, vec![(1, 4), (2, 4), (2, 6)]);
    }

    // The constructor's error path (`JsValue::from_str`) calls into
    // wasm-bindgen's JS-describe glue, which panics with "not implemented"
    // outside a wasm32 target — so it can't be exercised from a native
    // `cargo test`. `automaton::tests::build_error_messages_are_readable`
    // covers the same `BuildError::to_string()` text this constructor
    // relays verbatim via `.map_err(|e| JsValue::from_str(&e.to_string()))`.
}
