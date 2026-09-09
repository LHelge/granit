// Granit presentation page script: canvas scaling, cursor hiding and
// navigation. Runs in the presentation window with no app code around it.
//
// Keys: arrows, space, PageUp/PageDown, Home/End move between slides;
// F or F11 toggles fullscreen; Escape leaves fullscreen, then closes the
// window. A click or tap on the right half advances, on the left half goes
// back. The current slide index lives in the URL hash so a reload keeps
// its position.
(function () {
  "use strict";

  var root = document.documentElement;
  var slides = Array.prototype.slice.call(document.querySelectorAll("section.slide"));
  var current = 0;

  // ── Canvas scaling ──────────────────────────────────────────────
  // The canvas size is read from the base stylesheet's custom properties
  // so it is defined in exactly one place.

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

  // ── Cursor hiding ───────────────────────────────────────────────

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

  // ── Slides ──────────────────────────────────────────────────────

  function show(index) {
    if (slides.length === 0) return;
    current = Math.max(0, Math.min(slides.length - 1, index));
    slides.forEach(function (slide, i) {
      slide.classList.toggle("active", i === current);
    });
    history.replaceState(null, "", "#" + current);
  }

  function indexFromHash() {
    var parsed = parseInt(location.hash.slice(1), 10);
    return isNaN(parsed) ? 0 : parsed;
  }

  show(indexFromHash());
  window.addEventListener("hashchange", function () {
    if (indexFromHash() !== current) show(indexFromHash());
  });

  // ── Window control ──────────────────────────────────────────────
  // Inside Granit the Tauri window API is available and drives the native
  // fullscreen mode and close; elsewhere fall back to the browser APIs.

  var tauriWindow =
    window.__TAURI__ && window.__TAURI__.window && window.__TAURI__.window.getCurrentWindow
      ? window.__TAURI__.window.getCurrentWindow()
      : null;

  function toggleFullscreen() {
    if (tauriWindow) {
      tauriWindow.isFullscreen().then(function (full) {
        return tauriWindow.setFullscreen(!full);
      });
    } else if (document.fullscreenElement) {
      document.exitFullscreen();
    } else if (root.requestFullscreen) {
      root.requestFullscreen();
    }
  }

  function escape() {
    if (tauriWindow) {
      tauriWindow.isFullscreen().then(function (full) {
        return full ? tauriWindow.setFullscreen(false) : tauriWindow.close();
      });
    } else if (document.fullscreenElement) {
      document.exitFullscreen();
    } else {
      window.close();
    }
  }

  // ── Input ───────────────────────────────────────────────────────

  window.addEventListener("keydown", function (event) {
    if (event.metaKey || event.ctrlKey || event.altKey) return;
    switch (event.key) {
      case "ArrowRight":
      case "ArrowDown":
      case "PageDown":
      case " ":
        show(current + 1);
        break;
      case "ArrowLeft":
      case "ArrowUp":
      case "PageUp":
        show(current - 1);
        break;
      case "Home":
        show(0);
        break;
      case "End":
        show(slides.length - 1);
        break;
      case "f":
      case "F":
      case "F11":
        toggleFullscreen();
        break;
      case "Escape":
        escape();
        break;
      default:
        return;
    }
    // Space and the page keys would otherwise scroll the webview.
    event.preventDefault();
  });

  // Clicks navigate; links are inert so a slide can never navigate the
  // presentation window away from the presentation.
  window.addEventListener("click", function (event) {
    event.preventDefault();
    if (event.clientX >= window.innerWidth / 2) {
      show(current + 1);
    } else {
      show(current - 1);
    }
  });
})();
