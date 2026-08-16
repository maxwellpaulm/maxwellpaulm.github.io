// The /ask/ terminal. Fetches the corpus, hands it to the Rust BM25
// searcher (ask_terminal wasm), and renders answers into #ask-log.
// All content shown is authored text from the corpus — this file only
// formats it. Class names are pinned by theme.rs tests.
import init, { Terminal } from "./ask_terminal.js";

const EXAMPLES = [
  "what did paul build at amazon?",
  "aho-corasick",
  "who are you?",
];

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

async function main() {
  const log = document.getElementById("ask-log");
  const form = document.getElementById("ask-form");
  const input = document.getElementById("ask-input");
  if (!log || !form || !input) return;

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
    let result;
    try {
      result = JSON.parse(terminal.ask(query));
    } catch (err) {
      console.error(err);
      children.push(el("div", "", "the search failed — everything it knows is on /about/ and /projects/."));
      card(children);
      return;
    }
    if (result.kind === "answer") {
      if (result.title) children.push(el("div", "", result.title));
      children.push(el("div", "", result.text));
      children.push(sourceLine(result.source, result.title));
      if (result.also.length) {
        children.push(el("div", "ask-also", "also: " + result.also.map((a) => a.title).join(" · ")));
      }
    } else {
      children.push(el("div", "", "no matches — try: " + result.suggest.join(", ")));
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

  try {
    const [, indexJson] = await Promise.all([
      init(),
      fetch("/ask/index.json").then((r) => {
        if (!r.ok) throw new Error("index fetch failed: " + r.status);
        return r.text();
      }),
    ]);
    terminal = new Terminal(indexJson);
  } catch (err) {
    console.error(err);
    card([el("div", "", "the terminal failed to load — everything it knows is on /about/ and /projects/.")]);
    return;
  }

  card([
    el("div", "", "ask about my work — e.g. " + EXAMPLES.map((e) => '"' + e + '"').join(", ")),
  ]);
}

main();
