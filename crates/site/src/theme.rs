#[derive(Clone, Copy)]
pub struct Palette {
    pub paper: &'static str,
    pub surface: &'static str,
    pub ink: &'static str,
    pub muted: &'static str,
    pub rule: &'static str,
    pub accent: &'static str,
}

pub const LIGHT: Palette = Palette {
    paper: "#FBFAF8",
    surface: "#FFFFFF",
    ink: "#14161A",
    muted: "#6E7076",
    rule: "#E5E2DC",
    accent: "#A8431E",
};

pub const DARK: Palette = Palette {
    paper: "#0E0F11",
    surface: "#16181B",
    ink: "#E9E7E2",
    muted: "#94989F",
    rule: "#25282D",
    accent: "#E0764A",
};

/// Green-phosphor tokens for the Konami-code CRT easter egg
/// (`static/crt.js` toggles `data-crt` on `<html>`). A full palette rather
/// than ad-hoc colours so the mode recolours everything through the same
/// variables light/dark use — and passes through the same WCAG AA test.
pub const CRT: Palette = Palette {
    paper: "#050805",
    surface: "#0B140C",
    ink: "#33FF66",
    muted: "#2EBF5A",
    rule: "#175A2B",
    accent: "#FFB84D",
};

#[cfg(test)]
fn channel_luminance(byte: u8) -> f64 {
    let c = f64::from(byte) / 255.0;
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

#[cfg(test)]
fn relative_luminance(hex: &str) -> f64 {
    let h = hex.trim_start_matches('#');
    let parse = |i: usize| u8::from_str_radix(&h[i..i + 2], 16).expect("valid hex colour");
    0.2126 * channel_luminance(parse(0))
        + 0.7152 * channel_luminance(parse(2))
        + 0.0722 * channel_luminance(parse(4))
}

/// WCAG 2.1 contrast ratio between two hex colours, from 1.0 to 21.0.
///
/// Exists purely to enforce the WCAG invariant in tests; no build-time
/// caller, so it lives only in the test target.
#[cfg(test)]
pub fn contrast_ratio(fg: &str, bg: &str) -> f64 {
    let (a, b) = (relative_luminance(fg), relative_luminance(bg));
    let (hi, lo) = if a > b { (a, b) } else { (b, a) };
    (hi + 0.05) / (lo + 0.05)
}

fn tokens(p: Palette) -> String {
    format!(
        "  --paper: {};\n  --surface: {};\n  --ink: {};\n  --muted: {};\n  --rule: {};\n  --accent: {};\n",
        p.paper, p.surface, p.ink, p.muted, p.rule, p.accent
    )
}

/// The complete stylesheet, emitted from the tokens above so light and dark
/// stay one system rather than two hand-maintained sheets.
pub fn stylesheet() -> String {
    format!(
        r#"@font-face {{
  font-family: "Inter";
  src: url("/fonts/InterVariable.woff2") format("woff2");
  font-weight: 100 900;
  font-display: swap;
}}
@font-face {{
  font-family: "JetBrains Mono";
  src: url("/fonts/JetBrainsMono.woff2") format("woff2");
  font-weight: 400;
  font-display: swap;
}}
:root {{
{light}
  color-scheme: light;
  --space: 8px;
  --measure: 62ch;
  --font-sans: "Inter", "Helvetica Neue", system-ui, sans-serif;
  --font-mono: "JetBrains Mono", ui-monospace, Menlo, monospace;
  --motion: 180ms;
}}

:root[data-theme="dark"] {{
{dark}
  color-scheme: dark;
}}

/* Easter egg: Konami-code CRT mode (static/crt.js toggles data-crt on
   <html>; Escape exits). Equal specificity with the dark block above, so
   this must stay after it — source order is what lets CRT win over an
   explicitly-dark theme. */
:root[data-crt] {{
{crt}
  color-scheme: dark;
}}
:root[data-crt] body {{
  font-family: var(--font-mono);
  text-shadow:
    0 0 2px rgba(51, 255, 102, 0.55),
    1px 0 0 rgba(255, 60, 60, 0.22),
    -1px 0 0 rgba(60, 120, 255, 0.22);
  animation: crt-enter 0.55s steps(2, end), crt-flicker 4s step-end 0.55s infinite;
}}
:root[data-crt] body::after {{
  content: "";
  position: fixed;
  inset: 0;
  z-index: 9999;
  pointer-events: none;
  background:
    repeating-linear-gradient(0deg, rgba(0, 0, 0, 0.25) 0 1px, transparent 1px 3px),
    radial-gradient(ellipse at center, transparent 60%, rgba(0, 0, 0, 0.35) 100%);
}}
@keyframes crt-enter {{
  0% {{ transform: translateX(-6px) skewX(2deg); opacity: 0.4; }}
  25% {{ transform: translateX(5px) skewX(-1.5deg); opacity: 0.9; }}
  50% {{ transform: translateX(-3px); opacity: 0.6; }}
  75% {{ transform: translateX(2px); opacity: 1; }}
  100% {{ transform: none; opacity: 1; }}
}}
@keyframes crt-flicker {{
  0%, 91% {{ opacity: 1; }}
  93% {{ opacity: 0.82; }}
  94% {{ opacity: 1; }}
  97% {{ opacity: 0.93; }}
  98%, 100% {{ opacity: 1; }}
}}

/* Easter egg: space shooter (static/ship.js, launched by clicking #pm-mark).
   Shot words hide via visibility so the layout — and every other word's
   hitbox — stays put; the canvas overlays everything but never eats input,
   the game listens on the document instead. */
.ship-hit {{ visibility: hidden; }}
#pm-mark {{
  cursor: crosshair;
  width: fit-content;
  transition: transform var(--motion) ease;
}}
#pm-mark:hover {{ transform: rotate(-15deg) translateY(-2px); }}
#pm-mark:hover::after {{
  content: "";
  display: inline-block;
  margin-left: 4px;
  border-top: 3px solid transparent;
  border-bottom: 3px solid transparent;
  border-right: 6px solid var(--accent);
}}
#ship-canvas {{
  position: fixed;
  inset: 0;
  width: 100%;
  height: 100%;
  z-index: 10000;
  pointer-events: none;
}}

*, *::before, *::after {{ box-sizing: border-box; }}

body {{
  margin: 0;
  background: var(--paper);
  color: var(--ink);
  font-family: var(--font-sans);
  font-size: 16px;
  line-height: 1.6;
  font-weight: 350;
  -webkit-font-smoothing: antialiased;
}}

a {{ color: var(--accent); text-decoration: none; }}
a:hover {{ text-decoration: underline; }}

:focus-visible {{
  outline: 2px solid var(--accent);
  outline-offset: 3px;
  border-radius: 2px;
}}

.mono {{
  font-family: var(--font-mono);
  font-size: 10px;
  letter-spacing: 0.14em;
  text-transform: uppercase;
  color: var(--muted);
}}

/* Composition A: persistent left rail, content off-centre. */
.layout {{ display: grid; grid-template-columns: 132px 1fr; min-height: 100vh; }}
.rail {{
  border-right: 1px solid var(--rule);
  padding: calc(var(--space) * 4) calc(var(--space) * 2.5);
  display: flex;
  flex-direction: column;
  gap: calc(var(--space) * 3.5);
}}
.rail nav {{ display: flex; flex-direction: column; gap: 9px; }}
.rail nav a {{ font-size: 13px; color: var(--muted); transition: color var(--motion) ease; }}
.rail nav a[aria-current="page"] {{ color: var(--ink); font-weight: 500; }}
.rail nav a[aria-current="page"]::before {{ content: "— "; color: var(--accent); }}
.rail-foot {{ margin-top: auto; }}
.rail-links {{ display: flex; flex-direction: column; gap: 4px; margin-top: calc(var(--space) * 1.5); }}
.rail-links a {{ font-size: 12px; color: var(--muted); transition: color var(--motion) ease; }}
.rail-links a:hover {{ color: var(--ink); text-decoration: none; }}
.theme-toggle {{
  margin-top: calc(var(--space) * 1.5);
  font-family: var(--font-mono);
  font-size: 10px;
  letter-spacing: 0.14em;
  text-transform: uppercase;
  color: var(--muted);
  background: var(--surface);
  border: 1px solid var(--rule);
  border-radius: 3px;
  padding: 5px 9px;
  cursor: pointer;
  transition: color var(--motion) ease, border-color var(--motion) ease;
}}
.theme-toggle:hover {{ color: var(--ink); border-color: var(--accent); }}

main {{ padding: calc(var(--space) * 6.5) calc(var(--space) * 7) calc(var(--space) * 5); }}
h1 {{ font-size: 54px; line-height: 1.02; letter-spacing: -0.035em; font-weight: 600; margin: 0 0 calc(var(--space) * 3.25); }}
.name-line {{ display: block; }}
h2 {{ font-size: 24px; letter-spacing: -0.02em; font-weight: 600; }}
.lede {{ font-size: 17px; line-height: 1.62; max-width: var(--measure); margin: 0 0 calc(var(--space) * 1.75); }}
.prose {{ font-size: 14.5px; line-height: 1.65; max-width: var(--measure); color: var(--muted); }}
.role-line {{ margin-top: 2rem; }}

.section-head {{
  display: flex; justify-content: space-between; align-items: baseline;
  padding-bottom: 11px; border-bottom: 1px solid var(--ink);
  margin: calc(var(--space) * 5.5) 0 0;
}}
.section-head .mono {{ color: var(--ink); }}

.item {{
  display: grid; grid-template-columns: 1fr 74px; gap: calc(var(--space) * 2.25);
  padding: calc(var(--space) * 2.125) 0; border-bottom: 1px solid var(--rule);
  align-items: start;
}}
.item h3 {{ margin: 0 0 5px; font-size: 15px; font-weight: 500; letter-spacing: -0.01em; }}
.item p {{ margin: 0; font-size: 13.5px; line-height: 1.55; color: var(--muted); max-width: var(--measure); }}
.item .year {{ text-align: right; }}
.org {{ color: var(--accent); }}
.work-detail {{ font-size: 13.5px; line-height: 1.6; color: var(--ink); max-width: var(--measure); margin: 6px 0 0; }}

.resume-page {{
  display: block;
  width: 100%;
  max-width: 780px;
  height: auto;
  margin: calc(var(--space) * 3) 0;
  border: 1px solid var(--rule);
  background: #FFFFFF;
}}

.ask-log {{
  font-family: var(--font-mono);
  font-size: 13px;
  line-height: 1.7;
  max-width: 660px;
  display: flex;
  flex-direction: column;
  gap: calc(var(--space) * 2);
  margin: calc(var(--space) * 3) 0;
}}
.ask-card {{ border-left: 2px solid var(--rule); padding-left: calc(var(--space) * 1.5); }}
.ask-q {{ color: var(--muted); }}
.ask-src {{ color: var(--muted); font-size: 11px; }}
.ask-src a {{ color: var(--accent); }}
.ask-also {{ color: var(--muted); font-size: 12px; }}
.ask-form {{
  display: flex;
  gap: var(--space);
  align-items: center;
  max-width: 660px;
}}
.ask-form input {{ flex: 1; }}
.ask-prompt {{ font-family: var(--font-mono); color: var(--accent); }}

/* The rail becomes a top bar before the grid gets cramped. */
@media (max-width: 640px) {{
  .layout {{ grid-template-columns: 1fr; }}
  .rail {{
    border-right: 0; border-bottom: 1px solid var(--rule);
    flex-direction: row; flex-wrap: wrap; align-items: center; justify-content: space-between;
    gap: calc(var(--space) * 2); padding: calc(var(--space) * 2);
  }}
  .rail nav {{ flex-direction: row; gap: calc(var(--space) * 2); flex-wrap: wrap; }}
  .rail-foot {{ margin-top: 0; min-width: 0; }}
  .rail-links {{ flex-direction: row; flex-wrap: wrap; gap: calc(var(--space) * 1.5); margin-top: 0; }}
  main {{ padding: calc(var(--space) * 4) calc(var(--space) * 2.5); }}
  h1 {{ font-size: 38px; }}
  .item {{ grid-template-columns: 1fr; gap: 4px; }}
  .item .year {{ text-align: left; }}
}}

@media (prefers-reduced-motion: reduce) {{
  *, *::before, *::after {{
    animation-duration: 0.01ms !important;
    transition-duration: 0.01ms !important;
  }}
}}

.rd-canvas {{
  width: 100%;
  max-width: 660px;
  aspect-ratio: 220 / 140;
  display: block;
  border: 1px solid var(--rule);
  image-rendering: pixelated;
  cursor: crosshair;
  touch-action: none;
  margin: calc(var(--space) * 3) 0;
}}
.rd-controls {{
  display: flex;
  flex-wrap: wrap;
  gap: var(--space);
  align-items: center;
  max-width: 660px;
}}

.ac-canvas {{
  width: 100%;
  max-width: 720px;
  aspect-ratio: 16 / 9;
  display: block;
  border: 1px solid var(--rule);
  background: var(--surface);
  margin: calc(var(--space) * 3) 0;
}}
.ac-controls {{
  display: flex;
  flex-wrap: wrap;
  gap: var(--space);
  align-items: center;
  max-width: 720px;
  margin-bottom: calc(var(--space) * 2);
}}
.demo-input {{
  font-family: var(--font-mono);
  font-size: 13px;
  color: var(--ink);
  background: var(--surface);
  border: 1px solid var(--rule);
  border-radius: 3px;
  padding: 5px 9px;
}}
#ac-scan {{
  font-family: var(--font-mono);
  font-size: 14px;
  line-height: 1.9;
  max-width: 720px;
  margin: calc(var(--space) * 2.5) 0;
  white-space: pre-wrap;
  word-break: break-word;
}}
#ac-scan .consumed {{ color: var(--muted); }}
#ac-scan .current {{ background: var(--accent); color: var(--surface); border-radius: 2px; }}
#ac-scan .matched {{ background: var(--rule); color: var(--ink); border-bottom: 2px solid var(--accent); }}
.demo-status {{
  font-family: var(--font-mono);
  font-size: 13px;
  color: var(--ink);
}}
"#,
        light = tokens(LIGHT),
        dark = tokens(DARK),
        crt = tokens(CRT),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contrast_ratio_matches_known_values() {
        // Black on white is the reference maximum, 21:1.
        assert!((contrast_ratio("#000000", "#FFFFFF") - 21.0).abs() < 0.01);
        assert!((contrast_ratio("#FFFFFF", "#FFFFFF") - 1.0).abs() < 0.01);
    }

    #[test]
    fn every_token_pair_meets_wcag_aa() {
        for (name, p) in [("light", LIGHT), ("dark", DARK), ("crt", CRT)] {
            for (bg_label, bg) in [("paper", p.paper), ("surface", p.surface)] {
                for (label, fg) in [("ink", p.ink), ("muted", p.muted), ("accent", p.accent)] {
                    let ratio = contrast_ratio(fg, bg);
                    assert!(
                        ratio >= 4.5,
                        "{name}/{label} on {bg_label} is {ratio:.2}:1, below WCAG AA 4.5:1"
                    );
                }
            }
        }
    }

    #[test]
    fn resume_page_class_used_in_markup_is_defined_here() {
        // pages/resume.rs writes `.resume-page` with nothing tying it to
        // this stylesheet.
        let css = stylesheet();
        assert!(css.contains(".resume-page {"), "stylesheet missing .resume-page: {css}");
    }

    #[test]
    fn role_line_class_used_in_markup_is_defined_here() {
        // pages/index.rs writes `.role-line` with nothing tying it to this
        // stylesheet — it replaces a `style="margin-top:2rem"` attribute so
        // the CSP can drop `style-src-attr 'unsafe-inline'`.
        let css = stylesheet();
        assert!(css.contains(".role-line {"), "stylesheet missing .role-line: {css}");
    }

    #[test]
    fn work_detail_class_used_in_markup_is_defined_here() {
        // components/work.rs's work_list_detailed writes `.work-detail` with
        // nothing tying it to this stylesheet.
        let css = stylesheet();
        assert!(css.contains(".work-detail {"), "stylesheet missing .work-detail: {css}");
    }

    #[test]
    fn rail_links_class_used_in_markup_is_defined_here() {
        // components/rail.rs writes `.rail-links` with nothing tying it to
        // this stylesheet, including the collapsed-top-bar row layout.
        let css = stylesheet();
        assert!(css.contains(".rail-links {"), "stylesheet missing .rail-links: {css}");
        assert!(
            css.contains(".rail-links { flex-direction: row;"),
            "stylesheet missing the collapsed top-bar .rail-links row variant: {css}"
        );
    }

    #[test]
    fn collapsed_rail_wraps_as_a_group_instead_of_overflowing() {
        // Below 640px the rail is a flex row (PM mark, nav, .rail-foot) laid
        // out with `justify-content: space-between`. If that row can't wrap
        // as a group, long-enough content in any one child pushes
        // `.rail-foot` past the right edge and the page scrolls
        // horizontally, which the site must never do. `.rail` needs
        // `flex-wrap: wrap` so the group can drop `.rail-foot` to its own
        // line, and `.rail-foot` needs `min-width: 0` so it isn't held to
        // its content's intrinsic width when the group doesn't wrap.
        let css = stylesheet();
        assert!(
            css.contains("flex-direction: row; flex-wrap: wrap; align-items: center; justify-content: space-between;"),
            "stylesheet missing flex-wrap on the collapsed .rail so it can wrap as a group: {css}"
        );
        assert!(
            css.contains(".rail-foot { margin-top: 0; min-width: 0; }"),
            "stylesheet missing min-width: 0 on the collapsed .rail-foot: {css}"
        );
    }

    #[test]
    fn theme_toggle_uses_the_surface_token_as_its_background() {
        let css = stylesheet();
        assert!(
            css.contains("background: var(--surface);"),
            "the --surface token has no consumer: {css}"
        );
    }

    #[test]
    fn stylesheet_defines_both_themes_and_respects_reduced_motion() {
        let css = stylesheet();
        assert!(css.contains("--paper: #FBFAF8"), "light tokens missing");
        assert!(css.contains("--paper: #0E0F11"), "dark tokens missing");
        assert!(css.contains("prefers-reduced-motion: reduce"));
        assert!(css.contains(":focus-visible"), "focus ring missing");
    }

    #[test]
    fn ac_canvas_class_used_in_markup_is_defined_here() {
        // pages/demo_aho_corasick.rs writes `.ac-canvas` with nothing tying
        // it to this stylesheet.
        let css = stylesheet();
        assert!(css.contains(".ac-canvas {"), "stylesheet missing .ac-canvas: {css}");
        assert!(
            css.contains("aspect-ratio: 16 / 9;"),
            "stylesheet missing the responsive aspect-ratio on .ac-canvas: {css}"
        );
    }

    #[test]
    fn demo_input_class_used_in_markup_is_defined_here() {
        // pages/demo_aho_corasick.rs writes `.demo-input` with nothing
        // tying it to this stylesheet.
        let css = stylesheet();
        assert!(css.contains(".demo-input {"), "stylesheet missing .demo-input: {css}");
        assert!(
            css.contains(".demo-input {\n  font-family: var(--font-mono);"),
            "demo inputs should be set in the mono typeface: {css}"
        );
    }

    #[test]
    fn ac_scan_span_classes_used_in_markup_are_defined_here() {
        // static/demos/aho-loader.js writes `.consumed`/`.current`/`.matched`
        // spans inside `#ac-scan` with nothing tying them to this stylesheet.
        let css = stylesheet();
        for class in ["#ac-scan .consumed {", "#ac-scan .current {", "#ac-scan .matched {"] {
            assert!(css.contains(class), "stylesheet missing {class}: {css}");
        }
    }

    #[test]
    fn demo_status_class_used_in_markup_is_defined_here() {
        // pages/demo_aho_corasick.rs writes `#ac-status` with `.demo-status`
        // (10px uppercase `.mono` defeats "loud errors" for the status
        // line that also carries build-error text) with nothing tying it
        // to this stylesheet.
        let css = stylesheet();
        assert!(css.contains(".demo-status {"), "stylesheet missing .demo-status: {css}");
        assert!(
            !css.contains(".demo-status {\n  font-family: var(--font-mono);\n  font-size: 10px;"),
            "status text must not be shrunk back to .mono's 10px uppercase treatment: {css}"
        );
    }

    #[test]
    fn matched_scan_text_meets_wcag_aa_against_its_own_background() {
        // #ac-scan .matched sets an explicit `color: var(--ink)` on
        // `background: var(--rule)` rather than inheriting `.consumed`'s
        // `color: var(--muted)` — a matched character is normally also
        // `consumed` (equal-specificity selectors, source order decides),
        // and muted-on-rule is only 3.83:1, below AA. Pin both the text
        // colour and the accent underline against that same background.
        for (name, p) in [("light", LIGHT), ("dark", DARK)] {
            let ink_ratio = contrast_ratio(p.ink, p.rule);
            assert!(ink_ratio >= 4.5, "{name} ink-on-rule is {ink_ratio:.2}:1, below WCAG AA 4.5:1");
            let accent_ratio = contrast_ratio(p.accent, p.rule);
            assert!(
                accent_ratio >= 4.5,
                "{name} accent-on-rule is {accent_ratio:.2}:1, below WCAG AA 4.5:1"
            );
        }
    }

    #[test]
    fn ac_scan_and_canvas_class_names_agree_with_the_loader_that_writes_them() {
        // The stylesheet-tying tests above only prove these classes exist
        // *somewhere* in this file — they don't prove the loader that
        // actually writes/reads them still spells them the same way.
        // Renaming a class in either place alone should break the build.
        let css = stylesheet();
        const LOADER_JS: &str = include_str!("../../../static/demos/aho-loader.js");
        for class in ["consumed", "current", "matched", "ac-canvas"] {
            assert!(css.contains(class), "stylesheet no longer mentions {class}: {css}");
            assert!(
                LOADER_JS.contains(class),
                "static/demos/aho-loader.js no longer mentions {class}"
            );
        }
    }

    #[test]
    fn crt_mode_styles_are_defined_here() {
        // static/crt.js toggles `data-crt` on <html> (the Konami-code
        // easter egg) with nothing tying it to this stylesheet. The mode
        // is its own token override block — like dark mode — plus the
        // scanline overlay and the flicker/entry animations. It must sit
        // *after* the dark block in the emitted sheet: the selectors have
        // equal specificity, so source order is what lets CRT win over an
        // explicitly-dark theme.
        let css = stylesheet();
        assert!(css.contains(":root[data-crt] {"), "stylesheet missing CRT token block: {css}");
        assert!(
            css.contains(":root[data-crt] body::after {"),
            "stylesheet missing the CRT scanline overlay: {css}"
        );
        assert!(css.contains("@keyframes crt-flicker"), "stylesheet missing CRT flicker: {css}");
        assert!(css.contains("@keyframes crt-enter"), "stylesheet missing CRT entry glitch: {css}");
        let dark = css.find(r#":root[data-theme="dark"]"#).expect("dark block present");
        let crt = css.find(":root[data-crt] {").expect("CRT block present");
        assert!(crt > dark, "CRT token block must come after the dark block to win on source order");
    }

    #[test]
    fn crt_attribute_names_agree_with_the_script_that_toggles_them() {
        // Same cross-file tie as the aho loader test above: the stylesheet
        // styles `[data-crt]` and static/crt.js sets it; renaming either
        // side alone should break the build.
        let css = stylesheet();
        const CRT_JS: &str = include_str!("../../../static/crt.js");
        assert!(css.contains("data-crt"), "stylesheet no longer mentions data-crt: {css}");
        assert!(CRT_JS.contains("data-crt"), "static/crt.js no longer toggles data-crt");
        assert!(CRT_JS.contains("Escape"), "static/crt.js no longer exits CRT mode on Escape");
    }

    #[test]
    fn ship_game_class_names_agree_with_the_script_that_writes_them() {
        // static/ship.js wraps page text in `.ship-word` spans, hides shot
        // words with `.ship-hit`, overlays `#ship-canvas`, and launches from
        // `#pm-mark` (see rail.rs). The stylesheet must define what the
        // script toggles, and renaming either side alone should break the
        // build. `.ship-hit` must hide via visibility, not display: pulling
        // a word out of flow would reflow every remaining target mid-game.
        let css = stylesheet();
        const SHIP_JS: &str = include_str!("../../../static/ship.js");
        assert!(
            css.contains(".ship-hit { visibility: hidden; }"),
            "stylesheet missing the layout-stable .ship-hit rule: {css}"
        );
        assert!(css.contains("#ship-canvas {"), "stylesheet missing #ship-canvas overlay: {css}");
        for name in ["ship-word", "ship-hit", "ship-canvas", "pm-mark"] {
            assert!(SHIP_JS.contains(name), "static/ship.js no longer mentions {name}");
        }
        assert!(SHIP_JS.contains("Escape"), "static/ship.js no longer quits on Escape");
    }

    #[test]
    fn pm_mark_hints_at_the_hidden_game_on_hover() {
        // The discoverability gag: hovering the PM monogram turns the cursor
        // into a crosshair and tips the mark into a launch pose with a tiny
        // accent exhaust flame (rail.rs adds the deadpan tooltip).
        let css = stylesheet();
        assert!(
            css.contains("#pm-mark {\n  cursor: crosshair;"),
            "stylesheet missing the crosshair cursor on #pm-mark: {css}"
        );
        assert!(css.contains("#pm-mark:hover {"), "stylesheet missing the #pm-mark launch pose: {css}");
        assert!(
            css.contains("#pm-mark:hover::after {"),
            "stylesheet missing the #pm-mark exhaust flame: {css}"
        );
    }

    #[test]
    fn ask_terminal_class_names_agree_with_the_script_that_writes_them() {
        // static/ask/terminal.js renders .ask-card/.ask-q/.ask-src/.ask-also
        // into #ask-log; renaming either side alone should break the build.
        let css = stylesheet();
        const TERMINAL_JS: &str = include_str!("../../../static/ask/terminal.js");
        for class in [".ask-log {", ".ask-card {", ".ask-src {", ".ask-also {", ".ask-form {"] {
            assert!(css.contains(class), "stylesheet missing {class}: {css}");
        }
        for name in ["ask-log", "ask-card", "ask-q", "ask-src", "ask-also", "ask-input"] {
            assert!(TERMINAL_JS.contains(name), "static/ask/terminal.js no longer mentions {name}");
        }
    }

    #[test]
    fn theme_is_explicit_not_os_driven() {
        let css = stylesheet();
        assert!(
            css.contains(r#"[data-theme="dark"]"#),
            "missing explicit dark theme selector"
        );
        assert!(
            !css.contains("prefers-color-scheme"),
            "OS-driven theming must not select the theme; prefers-color-scheme should be gone"
        );
        assert!(css.contains("color-scheme: light"), "missing light color-scheme");
        assert!(css.contains("color-scheme: dark"), "missing dark color-scheme");
    }
}
