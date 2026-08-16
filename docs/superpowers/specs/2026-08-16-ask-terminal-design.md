# Ask terminal — design spec

Date: 2026-08-16
Status: approved design, pre-implementation

## Goal

A first-class `/ask/` page where visitors type natural questions about
Paul's work and get real answers instantly, in the browser, with no
server and no LLM. Answers are retrieval-only: every response is text
Paul wrote, ranked by a Rust BM25 searcher compiled to WASM. Truthful by
construction, ~tens of KB of data, zero CSP changes.

## Non-goals

- No generative model, in-browser or API-backed. The terminal never
  composes sentences; it surfaces authored passages.
- No resume-PDF text extraction in v1. The `[[work]]` details and
  authored notes cover that ground, and indexing the PDF would couple
  the build to the private-repo fetch (`scripts/fetch-resume.sh`).
- No embedding model in v1. BM25 + aliases + intents first; a small
  embedding reranker is a possible later phase and changes no UI.
- No search-as-you-type, analytics, or query logging.

## Content schema — `content/ask.toml`

Two entry kinds, both authored:

```toml
[[intent]]                       # the ~10 questions everyone asks
match = ["who are you", "who is paul", "about paul"]
answer = "I'm Paul — a software engineer in DC, currently at P-1 AI..."
source = "/about/"

[[note]]                         # richer Q&A material
q = "What does Paul work on at P-1 AI?"
a = "Longer authored answer — the detail you'd tell a curious colleague."
aliases = ["p1", "archie", "current job", "employer"]
source = "/projects/"
```

- `intent.match` phrases are matched by normalized token overlap; a hit
  short-circuits ranking and returns `answer` directly.
- `note.q` doubles as the answer card's title and is indexed at higher
  weight than the body. `aliases` are hidden bonus index terms.
- `source` is a site-relative path rendered as a link under the answer.

The indexer also auto-derives passages from existing content in
`content/site.toml`, so the terminal is useful with zero notes written:

- each `about` paragraph (source `/about/`),
- each `[[work]]` entry — `title`, `org`, `year`, `summary`, `detail` —
  as one passage (source `/projects/`, title = work title),
- the `lede`/`bio`/`role` lines as one "now" passage (source `/`).

## Shipped artifact — corpus, not index

The site build emits `dist/ask/index.json`:

```json
{
  "passages": [{ "title": "...", "text": "...", "source": "/projects/", "aliases": ["..."], "boost": ["...q tokens..."] }],
  "intents":  [{ "match": ["..."], "answer": "...", "source": "..." }]
}
```

Rationale: at ~50–100 passages the WASM crate can tokenize and build
BM25 structures at page load in well under a millisecond, so shipping
the corpus keeps the JSON format trivial, keeps all tokenization in one
implementation (Rust), and makes a content edit a rebuild of one small
JSON file — never a WASM rebuild. The existing demo crates set this
precedent: WASM binaries are built by `scripts/build-wasm.sh` and
committed under `static/`, while data flows at runtime.

## Search crate — `crates/ask`

New workspace member, compiled to WASM via wasm-bindgen like the demo
crates; also compiled natively for its unit tests.

Pipeline, applied identically to passages (at load) and queries:

1. lowercase, strip punctuation, split on whitespace;
2. drop stopwords (~120-word English list — this is what reduces
   "what did paul build at amazon?" to `{build, amazon}`);
3. light suffix stemming: `ing`, `ed`, `es`, `s` (no external stemmer
   dependency at this corpus size).

Query handling:

1. Intent check: normalized query tokens vs. each `intent.match`
   phrase's tokens; a match (all phrase tokens present, or exact
   normalized equality) returns the canned answer.
2. Otherwise BM25 (k1 = 1.2, b = 0.75) over all passages. A note's `q`
   tokens and `aliases` are indexed with a term-frequency boost so they
   outrank incidental body matches.
3. Return top 5: first is the answer card, the rest are "also
   relevant" links.
4. No passage sharing at least one query term (BM25 score zero): an
   honest miss message that suggests
   real vocabulary sampled from the corpus ("try: amazon, fraud,
   aho-corasick, georgia tech") rather than pretending.

Public WASM surface (wasm-bindgen): `init(index_json: &str)` and
`ask(query: &str) -> String` returning a small JSON result the loader
renders. No DOM access from Rust; rendering is the loader's job,
mirroring how the demo crates split responsibilities.

## Page & UX — `/ask/`

- New `Route::Ask`, path `/ask/`, nav label `Ask`. Joining `Route::ALL`
  gives nav, sitemap, canonical, and build-loop coverage automatically.
- Page follows the demo-page pattern: `noscript` fallback pointing at
  `/about/` and `/projects/`, then
  `<script type="module" src="/ask/loader.js">`.
- `static/ask/loader.js` fetches `/ask/index.json`, instantiates the
  WASM (`static/ask/` artifacts), and renders the terminal.
- Terminal: JetBrains Mono; a scrollback of question/answer cards above
  a prompt row (`>` + a real `<input>`). Answer card = passage text, a
  muted source line ("— /projects/ · Duplicate-Invoice Detection", path
  is a real link), and an "also:" row of runner-up titles. A greeting
  line on load offers two or three example questions.
- Keyboard: Enter submits; ArrowUp recalls query history. Everything
  degrades to a normal form input.
- Styling via `theme.rs` tokens with `.ask-*` classes — inherits light,
  dark, and CRT mode.

## Security / CSP

No changes. The module script and WASM are same-origin (`script-src
'self'` + existing `'wasm-unsafe-eval'`); the `index.json` fetch is
`connect-src 'self'`; no inline scripts or styles are added, so
`security/csp-hashes.txt` is untouched and the drift gate proves it.

## Testing

- `crates/ask` unit tests (native, in CI): tokenizer/stopword/stemmer
  behavior; ranking facts ("what did paul build at amazon" ranks an
  Amazon passage first; "aho corasick" finds the Ampla project); intent
  matching; alias hits; empty-query and no-hit paths.
- Site build tests (existing idiom): `index.json` is emitted and
  parses; every `[[work]]` entry and about paragraph appears in the
  corpus; `/ask/` references the ask loader and only `/ask/` does (same
  guard the demo loaders have); class names cross-checked between
  `theme.rs`, the loader, and the page markup.
- CI ships-check for the WASM artifacts, mirroring the demo crates.
- Live verification before completion: headless-browser session typing
  real questions, asserting the answer card cites the right source,
  exercising the miss path, and reviewing screenshots.

## Phasing

1. **v1 (this spec):** everything above.
2. **Possible later:** small embedding reranker (~25MB, opt-in
   download) for paraphrase matching; resume-PDF text extraction.
   Neither changes the UI or the index.json contract.
