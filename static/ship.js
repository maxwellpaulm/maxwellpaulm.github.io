// Easter egg: clicking the PM mark (#pm-mark) launches a tiny asteroids-style
// shooter over the live page. ←/→ rotate, ↑ thrusts, space shoots the page's
// words away; touching a surviving word is death. Escape (or dying) restores
// every word exactly as it was. Styles (#ship-canvas, .ship-hit) live in the
// generated stylesheet (crates/site/src/theme.rs), whose tests pin the names
// used here.
(function () {
  "use strict";

  var TURN = 0.075; // radians per frame at 60fps
  var THRUST = 0.16; // px/frame^2
  var FRICTION = 0.99;
  var MAX_SPEED = 7;
  var BULLET_SPEED = 10;
  var BULLET_LIFE = 70; // frames
  var COOLDOWN = 140; // ms between shots
  var SHIP_RADIUS = 9;
  var GRACE_MS = 2000; // spawn invulnerability
  var SCROLL_BAND = 130; // px from viewport top/bottom before the camera follows

  var running = false;
  var canvas = null;
  var ctx = null;
  var splits = []; // {node, inserted} for perfect DOM restore
  var words = []; // .ship-word spans still alive
  var keys = {};
  var ship, bullets, particles, score, startedAt, lastShot, dead, deadAt, rafId;

  function colors() {
    var s = getComputedStyle(document.documentElement);
    return {
      ink: s.getPropertyValue("--ink").trim() || "#14161A",
      accent: s.getPropertyValue("--accent").trim() || "#A8431E",
      muted: s.getPropertyValue("--muted").trim() || "#6E7076",
    };
  }

  function splitWords() {
    var walker = document.createTreeWalker(document.body, NodeFilter.SHOW_TEXT, {
      acceptNode: function (node) {
        if (!/\S/.test(node.data)) return NodeFilter.FILTER_REJECT;
        var name = node.parentNode && node.parentNode.nodeName;
        if (name === "SCRIPT" || name === "STYLE" || name === "NOSCRIPT") {
          return NodeFilter.FILTER_REJECT;
        }
        return NodeFilter.FILTER_ACCEPT;
      },
    });
    var nodes = [];
    while (walker.nextNode()) nodes.push(walker.currentNode);
    nodes.forEach(function (node) {
      var frag = document.createDocumentFragment();
      var inserted = [];
      node.data.split(/(\s+)/).forEach(function (part) {
        if (!part) return;
        var child;
        if (/\s/.test(part)) {
          child = document.createTextNode(part);
        } else {
          child = document.createElement("span");
          child.className = "ship-word";
          child.textContent = part;
          words.push(child);
        }
        inserted.push(child);
        frag.appendChild(child);
      });
      node.parentNode.replaceChild(frag, node);
      splits.push({ node: node, inserted: inserted });
    });
  }

  function restoreWords() {
    splits.forEach(function (split) {
      var first = split.inserted[0];
      if (!first || !first.parentNode) return;
      first.parentNode.insertBefore(split.node, first);
      split.inserted.forEach(function (child) {
        if (child.parentNode) child.parentNode.removeChild(child);
      });
    });
    splits = [];
    words = [];
  }

  function start() {
    if (running) return;
    running = true;
    splitWords();
    canvas = document.createElement("canvas");
    canvas.id = "ship-canvas";
    document.body.appendChild(canvas);
    ctx = canvas.getContext("2d");
    resize();
    ship = {
      x: innerWidth / 2,
      y: innerHeight / 2,
      vx: 0,
      vy: 0,
      angle: -Math.PI / 2,
    };
    bullets = [];
    particles = [];
    keys = {};
    score = 0;
    dead = false;
    startedAt = performance.now();
    lastShot = 0;
    addEventListener("keydown", onKeyDown, true);
    addEventListener("keyup", onKeyUp, true);
    addEventListener("resize", resize);
    rafId = requestAnimationFrame(frame);
  }

  function end() {
    if (!running) return;
    running = false;
    cancelAnimationFrame(rafId);
    removeEventListener("keydown", onKeyDown, true);
    removeEventListener("keyup", onKeyUp, true);
    removeEventListener("resize", resize);
    if (canvas && canvas.parentNode) canvas.parentNode.removeChild(canvas);
    canvas = ctx = null;
    restoreWords();
  }

  function resize() {
    if (!canvas) return;
    var dpr = devicePixelRatio || 1;
    canvas.width = innerWidth * dpr;
    canvas.height = innerHeight * dpr;
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
  }

  var GAME_KEYS = ["ArrowLeft", "ArrowRight", "ArrowUp", "ArrowDown", " "];

  function onKeyDown(event) {
    if (event.key === "Escape") {
      end();
      return;
    }
    if (GAME_KEYS.indexOf(event.key) !== -1) {
      keys[event.key] = true;
      event.preventDefault();
    }
  }

  function onKeyUp(event) {
    keys[event.key] = false;
  }

  function shoot(now) {
    if (now - lastShot < COOLDOWN) return;
    lastShot = now;
    bullets.push({
      x: ship.x + Math.cos(ship.angle) * 12,
      y: ship.y + Math.sin(ship.angle) * 12,
      vx: Math.cos(ship.angle) * BULLET_SPEED + ship.vx,
      vy: Math.sin(ship.angle) * BULLET_SPEED + ship.vy,
      life: BULLET_LIFE,
    });
  }

  function burst(x, y, color, count) {
    for (var i = 0; i < count; i++) {
      var angle = Math.random() * Math.PI * 2;
      var speed = 0.5 + Math.random() * 3;
      particles.push({
        x: x,
        y: y,
        vx: Math.cos(angle) * speed,
        vy: Math.sin(angle) * speed,
        life: 30 + Math.random() * 25,
        color: color,
      });
    }
  }

  // Viewport-relative rects of words still in play, skipping anything
  // hidden or off-screen (rects are re-read every frame, so scrolling
  // never leaves stale hitboxes behind).
  function targetRects() {
    var out = [];
    for (var i = 0; i < words.length; i++) {
      var rect = words[i].getBoundingClientRect();
      if (rect.width === 0 || rect.height === 0) continue;
      if (rect.bottom < 0 || rect.top > innerHeight) continue;
      if (rect.right < 0 || rect.left > innerWidth) continue;
      out.push({ rect: rect, span: words[i], index: i });
    }
    return out;
  }

  function killWord(target, c) {
    target.span.classList.add("ship-hit");
    words.splice(words.indexOf(target.span), 1);
    score++;
    burst(
      target.rect.left + target.rect.width / 2,
      target.rect.top + target.rect.height / 2,
      c.accent,
      10
    );
  }

  function followCamera() {
    var overshoot = 0;
    if (ship.y < SCROLL_BAND) overshoot = ship.y - SCROLL_BAND;
    else if (ship.y > innerHeight - SCROLL_BAND) {
      overshoot = ship.y - (innerHeight - SCROLL_BAND);
    }
    if (overshoot !== 0) {
      var before = scrollY;
      scrollBy(0, overshoot);
      ship.y -= scrollY - before; // camera absorbed what it could
    }
    if (ship.y < SHIP_RADIUS) {
      ship.y = SHIP_RADIUS;
      ship.vy = Math.abs(ship.vy) * 0.4;
    }
    if (ship.y > innerHeight - SHIP_RADIUS) {
      ship.y = innerHeight - SHIP_RADIUS;
      ship.vy = -Math.abs(ship.vy) * 0.4;
    }
    ship.x = (ship.x + innerWidth) % innerWidth;
  }

  function frame(now) {
    var c = colors();
    ctx.clearRect(0, 0, innerWidth, innerHeight);

    if (!dead) {
      if (keys.ArrowLeft) ship.angle -= TURN;
      if (keys.ArrowRight) ship.angle += TURN;
      if (keys.ArrowUp) {
        ship.vx += Math.cos(ship.angle) * THRUST;
        ship.vy += Math.sin(ship.angle) * THRUST;
      }
      var speed = Math.hypot(ship.vx, ship.vy);
      if (speed > MAX_SPEED) {
        ship.vx *= MAX_SPEED / speed;
        ship.vy *= MAX_SPEED / speed;
      }
      ship.vx *= FRICTION;
      ship.vy *= FRICTION;
      ship.x += ship.vx;
      ship.y += ship.vy;
      followCamera();
      if (keys[" "]) shoot(now);
    }

    var targets = targetRects();

    for (var i = bullets.length - 1; i >= 0; i--) {
      var b = bullets[i];
      b.x += b.vx;
      b.y += b.vy;
      b.life--;
      var gone = b.life <= 0 || b.x < 0 || b.x > innerWidth || b.y < 0 || b.y > innerHeight;
      if (!gone) {
        for (var t = 0; t < targets.length; t++) {
          var r = targets[t].rect;
          if (b.x >= r.left && b.x <= r.right && b.y >= r.top && b.y <= r.bottom) {
            killWord(targets[t], c);
            targets.splice(t, 1);
            gone = true;
            break;
          }
        }
      }
      if (gone) bullets.splice(i, 1);
    }

    var grace = now - startedAt < GRACE_MS;
    if (!dead && !grace) {
      for (var w = 0; w < targets.length; w++) {
        var rect = targets[w].rect;
        var nx = Math.max(rect.left, Math.min(ship.x, rect.right));
        var ny = Math.max(rect.top, Math.min(ship.y, rect.bottom));
        if (Math.hypot(ship.x - nx, ship.y - ny) < SHIP_RADIUS) {
          dead = true;
          deadAt = now;
          burst(ship.x, ship.y, c.ink, 26);
          break;
        }
      }
    }

    for (var p = particles.length - 1; p >= 0; p--) {
      var pt = particles[p];
      pt.x += pt.vx;
      pt.y += pt.vy;
      pt.vx *= 0.97;
      pt.vy *= 0.97;
      pt.life--;
      if (pt.life <= 0) particles.splice(p, 1);
    }

    draw(c, now, grace);

    if (dead && now - deadAt > 2400) {
      end();
      return;
    }
    rafId = requestAnimationFrame(frame);
  }

  function draw(c, now, grace) {
    ctx.font = '11px "JetBrains Mono", monospace';

    particles.forEach(function (pt) {
      ctx.globalAlpha = Math.max(pt.life / 40, 0);
      ctx.fillStyle = pt.color;
      ctx.fillRect(pt.x - 1.5, pt.y - 1.5, 3, 3);
    });
    ctx.globalAlpha = 1;

    ctx.fillStyle = c.accent;
    bullets.forEach(function (b) {
      ctx.fillRect(b.x - 1.5, b.y - 1.5, 3, 3);
    });

    if (!dead && (!grace || Math.floor(now / 150) % 2 === 0)) {
      ctx.save();
      ctx.translate(ship.x, ship.y);
      ctx.rotate(ship.angle);
      ctx.strokeStyle = c.ink;
      ctx.lineWidth = 1.5;
      ctx.beginPath();
      ctx.moveTo(12, 0);
      ctx.lineTo(-8, -7);
      ctx.lineTo(-4, 0);
      ctx.lineTo(-8, 7);
      ctx.closePath();
      ctx.stroke();
      if (keys.ArrowUp) {
        ctx.strokeStyle = c.accent;
        ctx.beginPath();
        ctx.moveTo(-6, -3);
        ctx.lineTo(-12 - Math.random() * 5, 0);
        ctx.lineTo(-6, 3);
        ctx.stroke();
      }
      ctx.restore();
    }

    // HUD, top centre: running word count and the way out.
    ctx.fillStyle = c.muted;
    ctx.textAlign = "center";
    ctx.fillText(score + " WORDS · ESC RESTORES", innerWidth / 2, 18);
    if (now - startedAt < 4000 && !dead) {
      ctx.fillText("← → ROTATE · ↑ THRUST · SPACE FIRE", innerWidth / 2, 34);
    }

    if (dead) {
      ctx.fillStyle = c.ink;
      ctx.font = '20px "JetBrains Mono", monospace';
      ctx.fillText("GAME OVER", innerWidth / 2, innerHeight / 2 - 12);
      ctx.font = '12px "JetBrains Mono", monospace';
      ctx.fillText(score + " WORDS DOWN", innerWidth / 2, innerHeight / 2 + 10);
    }
    ctx.textAlign = "left";
  }

  var mark = document.getElementById("pm-mark");
  if (mark) mark.addEventListener("click", start);
})();
