// Lazy loader for the reaction-diffusion demo. Only this page imports it,
// so no other page on the site downloads any WebAssembly.
import init, { Simulation } from "./reaction-diffusion/reaction_diffusion.js";

const PRESETS = {
  coral:     { feed: 0.0545, kill: 0.0620 },
  mitosis:   { feed: 0.0367, kill: 0.0649 },
  solitons:  { feed: 0.0300, kill: 0.0620 },
  worms:     { feed: 0.0780, kill: 0.0610 },
};

async function main() {
  const canvas = document.getElementById("rd-canvas");
  const status = document.getElementById("rd-status");
  if (!canvas) return;

  const wasm = await init();
  const sim = new Simulation(220, 140);
  const w = sim.width(), h = sim.height();
  canvas.width = w;
  canvas.height = h;

  const ctx = canvas.getContext("2d");
  const image = ctx.createImageData(w, h);

  let preset = PRESETS.coral;
  let running = false;
  let frames = 0, fpsAt = performance.now();

  const isDark = () => document.documentElement.dataset.theme === "dark";

  function seedCentre() {
    sim.reset();
    for (let i = 0; i < 12; i++) {
      const x = Math.floor(w / 2 + (Math.random() - 0.5) * 40);
      const y = Math.floor(h / 2 + (Math.random() - 0.5) * 40);
      sim.seed(x, y, 3);
    }
  }

  function draw() {
    sim.render(isDark());
    const px = new Uint8ClampedArray(wasm.memory.buffer, sim.pixels_ptr(), sim.pixels_len());
    image.data.set(px);
    ctx.putImageData(image, 0, 0);
  }

  function frame() {
    if (!running) return;
    sim.step(preset.feed, preset.kill, 8);
    draw();
    frames++;
    const now = performance.now();
    if (now - fpsAt >= 1000) {
      status.textContent = frames + " fps";
      frames = 0;
      fpsAt = now;
    }
    requestAnimationFrame(frame);
  }

  function start() {
    if (running) return;
    running = true;
    status.textContent = "running";
    requestAnimationFrame(frame);
  }

  function stop() {
    running = false;
    status.textContent = "paused";
  }

  canvas.addEventListener("pointerdown", (e) => {
    const r = canvas.getBoundingClientRect();
    const x = Math.floor((e.clientX - r.left) / r.width * w);
    const y = Math.floor((e.clientY - r.top) / r.height * h);
    sim.seed(x, y, 4);
    if (!running) draw();
  });

  document.getElementById("rd-toggle").addEventListener("click", () => {
    running ? stop() : start();
  });
  document.getElementById("rd-reset").addEventListener("click", () => {
    seedCentre();
    draw();
  });
  document.querySelectorAll("[data-preset]").forEach((b) => {
    b.addEventListener("click", () => {
      preset = PRESETS[b.dataset.preset];
      seedCentre();
      if (!running) draw();
    });
  });

  seedCentre();
  draw();

  // Respect a reduced-motion preference: render one frame and wait to be
  // asked rather than animating unbidden.
  if (window.matchMedia("(prefers-reduced-motion: reduce)").matches) {
    status.textContent = "paused — reduced motion";
  } else {
    start();
  }
}

main();
