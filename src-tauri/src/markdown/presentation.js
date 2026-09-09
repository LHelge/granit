// Granit presentation page script.
//
// Canvas scaling and cursor hiding. The canvas size is read from the base
// stylesheet's custom properties so it is defined in exactly one place.
(function () {
  "use strict";

  var root = document.documentElement;

  function canvasSize(name, fallback) {
    var value = parseFloat(getComputedStyle(root).getPropertyValue(name));
    return isNaN(value) ? fallback : value;
  }

  function fitCanvas() {
    var width = canvasSize("--slide-width", 1280);
    var height = canvasSize("--slide-height", 720);
    var scale = Math.min(window.innerWidth / width, window.innerHeight / height);
    root.style.setProperty("--slide-scale", String(scale));
  }

  window.addEventListener("resize", fitCanvas);
  fitCanvas();

  // Hide the cursor after a few seconds without mouse movement.
  var cursorTimer = null;
  function showCursor() {
    document.body.classList.remove("cursor-hidden");
    if (cursorTimer !== null) clearTimeout(cursorTimer);
    cursorTimer = setTimeout(function () {
      document.body.classList.add("cursor-hidden");
    }, 3000);
  }
  window.addEventListener("mousemove", showCursor);
  showCursor();
})();
