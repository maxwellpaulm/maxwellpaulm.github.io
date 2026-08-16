// Easter egg: the Konami code (↑↑↓↓←→←→BA) flips the site into CRT mode
// by toggling `data-crt` on <html>; Escape or the code again exits. The
// look itself lives under [data-crt] in the generated stylesheet
// (crates/site/src/theme.rs), which cross-checks these names in its tests.
(function () {
  "use strict";
  var CODE = [
    "ArrowUp", "ArrowUp", "ArrowDown", "ArrowDown",
    "ArrowLeft", "ArrowRight", "ArrowLeft", "ArrowRight",
    "b", "a",
  ];
  var progress = 0;
  var root = document.documentElement;
  addEventListener("keydown", function (event) {
    if (event.key === "Escape") {
      root.removeAttribute("data-crt");
      progress = 0;
      return;
    }
    var key = event.key.length === 1 ? event.key.toLowerCase() : event.key;
    if (key === CODE[progress]) {
      progress += 1;
    } else {
      progress = key === CODE[0] ? 1 : 0;
    }
    if (progress === CODE.length) {
      progress = 0;
      root.toggleAttribute("data-crt");
    }
  });
})();
