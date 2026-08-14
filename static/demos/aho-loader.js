// Lazy loader for the Aho-Corasick visualizer demo. Only this page imports
// it, so no other page on the site downloads this WebAssembly module.
import init, { Visualizer } from "./aho-corasick/aho_corasick_demo.js";

const MAX_TEXT_LEN = 200; // must match Visualizer's own byte cap
const STEP_MS = 1000 / 3; // ~3 steps/second

// The wasm's 200 cap is a *byte* cap that only equals a char cap for ASCII
// text (see the crate's MAX_TEXT_LEN doc comment). Stripping non-ASCII here
// — mirroring what the Rust side folds away from patterns anyway — keeps
// byte offsets, JS string indices, and the DOM spans built from this same
// string all in agreement, so the match ranges the wasm reports (byte
// offsets into the *stored* text) can index straight into those spans.
function stripNonAscii(s) {
  return s.replace(/[^\x00-\x7F]/g, "");
}

function sanitizeText(s) {
  return stripNonAscii(s).slice(0, MAX_TEXT_LEN);
}

async function main() {
  const canvas = document.getElementById("ac-canvas");
  const status = document.getElementById("ac-status");
  const patternsInput = document.getElementById("ac-patterns");
  const textInput = document.getElementById("ac-text");
  const scan = document.getElementById("ac-scan");
  const rebuildBtn = document.getElementById("ac-rebuild");
  const playBtn = document.getElementById("ac-play");
  const stepBtn = document.getElementById("ac-step");
  const resetBtn = document.getElementById("ac-reset");
  if (
    !canvas || !status || !patternsInput || !textInput || !scan ||
    !rebuildBtn || !playBtn || !stepBtn || !resetBtn
  ) return;

  const wasm = await init();
  const ctx = canvas.getContext("2d");

  let viz = null;
  let text = "";
  let running = false;
  let generation = 0;
  let lastStepAt = 0;
  let matched = []; // accumulated [start, end) text ranges; cleared on reset/rebuild
  let hopStates = new Set(); // states landed on by the most recent step's failure hops

  const cssVar = (name) => getComputedStyle(document.documentElement).getPropertyValue(name).trim();

  function buildScan() {
    scan.textContent = "";
    for (const ch of text) {
      const span = document.createElement("span");
      span.textContent = ch;
      scan.appendChild(span);
    }
  }

  function refreshScanClasses() {
    const spans = scan.children;
    const pos = viz ? viz.pos() : 0;
    for (let i = 0; i < spans.length; i++) {
      spans[i].className = i < pos ? "consumed" : "";
    }
    if (viz && pos < spans.length) {
      spans[pos].className = "current";
    }
    for (const [start, end] of matched) {
      for (let i = start; i < end && i < spans.length; i++) {
        spans[i].classList.add("matched");
      }
    }
  }

  // Rebuilds every typed-array view from the live pointer on every read:
  // wasm memory growth silently detaches the backing ArrayBuffer, and a
  // retained view then reads garbage or throws (see Visualizer's *_ptr doc
  // comments in the wasm crate).
  function readNodeArrays() {
    const n = viz.node_count();
    return {
      xs: new Float32Array(wasm.memory.buffer, viz.xs_ptr(), n),
      ys: new Float32Array(wasm.memory.buffer, viz.ys_ptr(), n),
      labels: new Uint8Array(wasm.memory.buffer, viz.labels_ptr(), n),
      parents: new Uint32Array(wasm.memory.buffer, viz.parents_ptr(), n),
      fails: new Uint32Array(wasm.memory.buffer, viz.fails_ptr(), n),
      terminal: new Uint8Array(wasm.memory.buffer, viz.terminal_ptr(), n),
    };
  }

  function readHops() {
    const len = viz.hops_len();
    return len === 0 ? [] : Array.from(new Uint32Array(wasm.memory.buffer, viz.hops_ptr(), len));
  }

  function readMatches() {
    const len = viz.match_len();
    if (len === 0) return [];
    const starts = new Uint32Array(wasm.memory.buffer, viz.match_starts_ptr(), len);
    const ends = new Uint32Array(wasm.memory.buffer, viz.match_ends_ptr(), len);
    const out = [];
    for (let i = 0; i < len; i++) out.push([starts[i], ends[i]]);
    return out;
  }

  function draw() {
    if (!viz) return;

    const dpr = window.devicePixelRatio || 1;
    const rect = canvas.getBoundingClientRect();
    const cw = Math.max(1, Math.round(rect.width * dpr));
    const ch = Math.max(1, Math.round(rect.height * dpr));
    if (canvas.width !== cw || canvas.height !== ch) {
      canvas.width = cw;
      canvas.height = ch;
    }
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    const w = rect.width, h = rect.height;
    ctx.clearRect(0, 0, w, h);

    const { xs, ys, labels, parents, fails, terminal } = readNodeArrays();
    const n = xs.length;

    let minX = Infinity, maxX = -Infinity, maxY = 0;
    for (let i = 0; i < n; i++) {
      if (xs[i] < minX) minX = xs[i];
      if (xs[i] > maxX) maxX = xs[i];
      if (ys[i] > maxY) maxY = ys[i];
    }
    const pad = 26;
    const spanX = Math.max(1e-6, maxX - minX);
    const spanY = Math.max(1e-6, maxY);
    const px = (x) => (spanX === 0 ? w / 2 : pad + ((x - minX) / spanX) * (w - 2 * pad));
    const py = (y) => pad + (y / spanY) * (h - 2 * pad);

    const rule = cssVar("--rule");
    const ink = cssVar("--ink");
    const muted = cssVar("--muted");
    const accent = cssVar("--accent");
    const surface = cssVar("--surface");

    const current = viz.current_state();
    const radius = Math.max(8, Math.min(14, w / (n + 4)));

    // Trie edges: solid line, parent -> child.
    ctx.strokeStyle = rule;
    ctx.lineWidth = 1.5;
    ctx.setLineDash([]);
    for (let i = 1; i < n; i++) {
      const p = parents[i];
      ctx.beginPath();
      ctx.moveTo(px(xs[p]), py(ys[p]));
      ctx.lineTo(px(xs[i]), py(ys[i]));
      ctx.stroke();
    }

    // Failure links: dashed quadratic arcs. Arcs whose target is the root
    // are skipped — nearly every shallow state fails to root, so drawing
    // all of those is pure clutter that would bury the failure links that
    // actually matter (the deeper, more interesting ones).
    ctx.strokeStyle = muted;
    ctx.setLineDash([4, 3]);
    for (let i = 1; i < n; i++) {
      const f = fails[i];
      if (f === 0) continue;
      const x0 = px(xs[i]), y0 = py(ys[i]);
      const x1 = px(xs[f]), y1 = py(ys[f]);
      const mx = (x0 + x1) / 2;
      const my = (y0 + y1) / 2 - 20;
      ctx.beginPath();
      ctx.moveTo(x0, y0);
      ctx.quadraticCurveTo(mx, my, x1, y1);
      ctx.stroke();
    }
    ctx.setLineDash([]);

    // Nodes: circles with letter labels. Terminal states are filled with
    // the accent; the current state gets an extra ring; states the last
    // step's failure hops passed through are flashed with an accent
    // outline.
    ctx.textAlign = "center";
    ctx.textBaseline = "middle";
    ctx.font = `${Math.max(9, radius)}px var(--font-mono), ui-monospace, monospace`;
    for (let i = 0; i < n; i++) {
      const x = px(xs[i]), y = py(ys[i]);
      const isTerminal = terminal[i] === 1;
      const isCurrent = i === current;
      const isHop = hopStates.has(i);

      ctx.beginPath();
      ctx.arc(x, y, radius, 0, Math.PI * 2);
      ctx.fillStyle = isTerminal ? accent : surface;
      ctx.fill();
      ctx.lineWidth = 1.5;
      ctx.strokeStyle = ink;
      ctx.stroke();

      // The hop flash is drawn as an outer ring rather than recolouring the
      // node's own stroke: a terminal state is already filled with
      // `accent`, so an accent-coloured inner stroke would sit flush
      // against a same-coloured fill and be invisible right when the flash
      // matters most (a hop landing on a pattern-ending state). An outer
      // ring drawn against the page background stays visible regardless of
      // the node's own fill colour; dashed, so it reads as distinct from
      // the current-state ring below even on the rare node that is both.
      if (isHop) {
        ctx.beginPath();
        ctx.setLineDash([2, 2]);
        ctx.arc(x, y, radius + 4, 0, Math.PI * 2);
        ctx.strokeStyle = accent;
        ctx.lineWidth = 2;
        ctx.stroke();
        ctx.setLineDash([]);
      }

      if (isCurrent) {
        ctx.beginPath();
        ctx.arc(x, y, radius + (isHop ? 8 : 4), 0, Math.PI * 2);
        ctx.strokeStyle = accent;
        ctx.lineWidth = 2;
        ctx.stroke();
      }

      ctx.fillStyle = isTerminal ? surface : ink;
      ctx.fillText(i === 0 ? "•" : String.fromCharCode(labels[i]), x, y);
    }
  }

  function rebuild() {
    const patterns = stripNonAscii(patternsInput.value);
    const nextText = sanitizeText(textInput.value);
    try {
      const next = new Visualizer(patterns, nextText);
      stop();
      viz = next;
      text = nextText;
      matched = [];
      hopStates = new Set();
      buildScan();
      refreshScanClasses();
      draw();
      status.textContent = `${viz.node_count()} states`;
    } catch (err) {
      // A bad rebuild must not brick the demo the user was already looking
      // at: the previous automaton (if any) stays exactly as it was.
      status.textContent = err && err.message ? err.message : String(err);
    }
  }

  function doStep() {
    const advanced = viz.step();
    hopStates = advanced ? new Set(readHops()) : new Set();
    if (advanced) matched.push(...readMatches());
    refreshScanClasses();
    draw();
    status.textContent = advanced ? `pos ${viz.pos()}/${text.length}` : "done";
    return advanced;
  }

  // Generation-token play loop, copied from /demos/loader.js: a `gen`
  // captured when `start()` runs is compared against the live `generation`
  // on every tick, so a rapid pause/play double-click can't leave two
  // chains of `requestAnimationFrame` calls racing each other and doubling
  // the effective step rate.
  function frame(gen) {
    if (!running || gen !== generation) return;
    requestAnimationFrame((ts) => {
      if (!running || gen !== generation) return;
      if (ts - lastStepAt >= STEP_MS) {
        lastStepAt = ts;
        if (!doStep()) {
          stop({ quiet: true }); // doStep already set status to "done"
          return;
        }
      }
      frame(gen);
    });
  }

  function start() {
    if (running || !viz || viz.pos() >= text.length) return;
    running = true;
    generation++;
    const gen = generation;
    lastStepAt = performance.now();
    playBtn.textContent = "Pause";
    status.textContent = "playing";
    frame(gen);
  }

  function stop({ quiet = false } = {}) {
    if (!running) return;
    running = false;
    generation++; // orphans any pending frame from the current chain
    playBtn.textContent = "Play";
    // A manual pause leaves "playing" as the last status text forever
    // unless something replaces it; every other caller (rebuild, step,
    // reset) immediately sets its own status right after calling `stop`,
    // so this only actually shows for the pause button itself.
    if (!quiet && viz) status.textContent = `pos ${viz.pos()}/${text.length}`;
  }

  // The site's theme toggle mutates data-theme but dispatches no event. A
  // playing automaton repaints on its own next tick, but a paused one
  // would otherwise sit rendered in the wrong palette until the next step.
  new MutationObserver(() => {
    if (!running) draw();
  }).observe(document.documentElement, { attributes: true, attributeFilter: ["data-theme"] });

  rebuildBtn.addEventListener("click", rebuild);
  playBtn.addEventListener("click", () => {
    running ? stop() : start();
  });
  stepBtn.addEventListener("click", () => {
    if (!viz) return;
    stop();
    doStep();
  });
  resetBtn.addEventListener("click", () => {
    if (!viz) return;
    stop();
    viz.reset();
    matched = [];
    hopStates = new Set();
    refreshScanClasses();
    draw();
    status.textContent = `${viz.node_count()} states`;
  });

  rebuild();

  // Respect a reduced-motion preference: build and draw, but never
  // auto-play or animate unbidden.
  if (window.matchMedia("(prefers-reduced-motion: reduce)").matches) {
    status.textContent = "paused — reduced motion";
  }
}

main().catch((err) => {
  const status = document.getElementById("ac-status");
  if (status) status.textContent = "failed to load";
  console.error("aho-corasick demo failed to load:", err);
});
