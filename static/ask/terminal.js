// The /ask/ terminal. Fetches the corpus, hands it to the Rust BM25
// searcher (ask_terminal wasm), and renders answers into #ask-log.
// All content shown is authored text from the corpus — this file only
// formats it. Class names are pinned by theme.rs tests.

function el(tag, className, text) {
  const node = document.createElement(tag);
  if (className) node.className = className;
  if (text) node.textContent = text;
  return node;
}

function sourceLine(source, title) {
  const src = el("div", "ask-src");
  src.appendChild(document.createTextNode("— "));
  const link = el("a", "", source);
  link.href = source;
  src.appendChild(link);
  if (title) src.appendChild(document.createTextNode(" · " + title));
  return src;
}

// Runner-ups are already filtered to a relevance floor by the engine, and
// every one carries a `source` — link each, rather than listing bare text.
function alsoLine(also) {
  const div = el("div", "ask-also");
  div.appendChild(document.createTextNode("also: "));
  also.forEach((a, i) => {
    if (i > 0) div.appendChild(document.createTextNode(" · "));
    const link = el("a", "", a.title);
    link.href = a.source;
    div.appendChild(link);
  });
  return div;
}

async function main() {
  const log = document.getElementById("ask-log");
  const form = document.getElementById("ask-form");
  const input = document.getElementById("ask-input");
  if (!log || !form || !input) {
    console.error("ask: expected #ask-log, #ask-form, #ask-input");
    return;
  }

  const card = (children) => {
    const c = el("div", "ask-card");
    children.forEach((ch) => c.appendChild(ch));
    log.appendChild(c);
    c.scrollIntoView({ block: "nearest" });
    return c;
  };

  let terminal = null;
  const history = [];
  let historyAt = -1;

  // Wired unconditionally, before the terminal has even loaded: a failed
  // load must still preventDefault() on submit, or a visitor who presses
  // Enter after reading the fallback card gets a native form navigation
  // that wipes it.
  form.addEventListener("submit", (event) => {
    event.preventDefault();
    if (!terminal) return;
    const query = input.value.trim();
    if (!query) return;
    history.push(query);
    historyAt = history.length;
    input.value = "";

    const children = [el("div", "ask-q", "> " + query)];
    // Guards ask()/JSON.parse AND the rendering below: a response-shape
    // drift throwing mid-render must not lose the card, or the echoed
    // "> query" never appends and the page looks frozen.
    try {
      const result = JSON.parse(terminal.ask(query));
      if (result.kind === "answer") {
        if (result.title) children.push(el("div", "", result.title));
        children.push(el("div", "", result.text));
        children.push(sourceLine(result.source, result.title));
        if (result.also.length) {
          children.push(alsoLine(result.also));
        }
      } else {
        children.push(el("div", "", "no matches — try: " + result.suggest.join(", ")));
      }
    } catch (err) {
      console.error(err);
      children.push(el("div", "", "the search failed — everything it knows is on /about/ and /projects/."));
    }
    card(children);
  });

  input.addEventListener("keydown", (event) => {
    if (event.key === "ArrowUp" && historyAt > 0) {
      historyAt -= 1;
      input.value = history[historyAt];
      event.preventDefault();
    } else if (event.key === "ArrowDown" && historyAt < history.length - 1) {
      historyAt += 1;
      input.value = history[historyAt];
      event.preventDefault();
    }
  });

  let examples = [];
  try {
    // Dynamic, not static: a static import at module top level throws
    // before main() ever runs if ask_terminal.js 404s (a partial deploy,
    // build-wasm.sh not run) — no fallback card, no submit handler,
    // nothing. Keeping the import inside this try is what makes the
    // fallback below actually reachable in that case.
    const { default: init, Terminal } = await import("./ask_terminal.js");
    const [, indexJson] = await Promise.all([
      init(),
      fetch("/ask/index.json").then((r) => {
        if (!r.ok) throw new Error("index fetch failed: " + r.status);
        return r.text();
      }),
    ]);
    terminal = new Terminal(indexJson);
    // The greeting's example queries come from the corpus itself (see
    // content/ask.toml's `examples`), not a hardcoded list here, so an
    // edited intent phrase can't silently make the site's own suggested
    // query stop resolving.
    examples = JSON.parse(indexJson).examples || [];
  } catch (err) {
    console.error(err);
    card([el("div", "", "the terminal failed to load — everything it knows is on /about/ and /projects/.")]);
    return;
  }

  card([
    el("div", "", "ask about my work — e.g. " + examples.map((e) => '"' + e + '"').join(", ")),
  ]);
}

main();
