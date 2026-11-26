(function () {
  "use strict";

  var root = document.documentElement;
  var reduceMotion = window.matchMedia("(prefers-reduced-motion: reduce)");

  /* ------------------------------------------------------------ icon markup */

  var PATHS = {
    sun: "M12 4V2m0 20v-2m8-8h2M2 12h2m13.7-5.7 1.4-1.4M4.9 19.1l1.4-1.4m11.4 0 1.4 1.4M4.9 4.9l1.4 1.4M16 12a4 4 0 1 1-8 0 4 4 0 0 1 8 0Z",
    moon: "M20.5 14.5A8.5 8.5 0 0 1 9.5 3.5 8.5 8.5 0 1 0 20.5 14.5Z",
    menu: "M4 7h16M4 12h16M4 17h16",
    close: "M6 6l12 12M18 6 6 18",
    check: "M4 12.5 9.5 18 20 6.5",
    send: "M4 12l16-8-6 16-2.5-6.5L4 12Z",
    spark: "M12 2v6m0 8v6M2 12h6m8 0h6M5 5l4 4m6 6 4 4M19 5l-4 4m-6 6-4 4"
  };

  function icon(name) {
    var svg = document.createElementNS("http://www.w3.org/2000/svg", "svg");
    svg.setAttribute("aria-hidden", "true");
    svg.setAttribute("focusable", "false");
    svg.setAttribute("class", "icon");
    svg.setAttribute("viewBox", "0 0 24 24");
    var path = document.createElementNS("http://www.w3.org/2000/svg", "path");
    path.setAttribute("d", PATHS[name] || "");
    svg.appendChild(path);
    return svg;
  }

  function setIcon(button, name) {
    button.replaceChildren(icon(name));
  }

  /* --------------------------------------------------------------- theme */

  var toggle = document.getElementById("theme-toggle");
  var themeMeta = document.querySelector('meta[name="theme-color"]');

  function currentTheme() {
    return root.dataset.theme === "dark" ? "dark" : "light";
  }

  function paintTheme() {
    var dark = currentTheme() === "dark";
    if (toggle) {
      setIcon(toggle, dark ? "sun" : "moon");
      toggle.setAttribute("aria-pressed", String(dark));
      toggle.setAttribute("aria-label", dark ? "Switch to light mode" : "Switch to dark mode");
    }
    if (themeMeta) themeMeta.setAttribute("content", dark ? "#0d0f0b" : "#f6f7f1");
  }

  if (toggle) {
    paintTheme();
    toggle.addEventListener("click", function () {
      var next = currentTheme() === "dark" ? "light" : "dark";
      root.dataset.theme = next;
      root.style.colorScheme = next;
      try {
        localStorage.setItem("theme", next);
      } catch (error) {
        /* storage can be blocked; the visual change still applies */
      }
      paintTheme();
    });

    var systemTheme = window.matchMedia("(prefers-color-scheme: dark)");
    systemTheme.addEventListener("change", function () {
      if (localStorage.getItem("theme")) return;
      root.dataset.theme = systemTheme.matches ? "dark" : "light";
      root.style.colorScheme = root.dataset.theme;
      paintTheme();
    });
  }

  /* ------------------------------------------------------- header + progress */

  var header = document.getElementById("site-header");
  var bar = document.getElementById("progress");
  var frame = 0;

  function paintScroll() {
    frame = 0;
    var scrolled = window.scrollY;
    if (header) header.dataset.scrolled = String(scrolled > 8);
    if (!bar) return;
    // Sections below the fold are contained (see site.css), so the document
    // height only settles as each one renders; it is re-read per frame for
    // that reason. Scrolling happens after layout is clean, so this measures
    // rather than forces.
    var max = document.documentElement.scrollHeight - window.innerHeight;
    bar.style.setProperty("--progress", max > 4 ? Math.min(1, scrolled / max) : 0);
  }

  function onScroll() {
    if (!frame) frame = requestAnimationFrame(paintScroll);
  }

  // Painting is deferred to the visitor's first scroll (or a restore) instead
  // of running during parse, where the same read forced a full-document layout
  // before first paint.
  window.addEventListener("scroll", onScroll, { passive: true });
  window.addEventListener("pageshow", onScroll);
  if (window.scrollY > 0) onScroll();

  /* ------------------------------------------------------------------ menu */

  var sheet = document.getElementById("menu-sheet");
  var openButton = document.getElementById("menu-open");
  var closeButton = document.getElementById("menu-close");

  if (openButton) setIcon(openButton, "menu");
  if (closeButton) setIcon(closeButton, "close");

  if (sheet && openButton) {
    openButton.addEventListener("click", function () {
      if (typeof sheet.showModal === "function") sheet.showModal();
      else sheet.setAttribute("open", "");
      openButton.setAttribute("aria-expanded", "true");
    });
    var closeSheet = function () {
      if (typeof sheet.close === "function") sheet.close();
      else sheet.removeAttribute("open");
      openButton.setAttribute("aria-expanded", "false");
    };
    if (closeButton) closeButton.addEventListener("click", closeSheet);
    sheet.addEventListener("close", function () {
      openButton.setAttribute("aria-expanded", "false");
    });
    sheet.addEventListener("click", function (event) {
      if (event.target === sheet) closeSheet();
    });
  }

  /* --------------------------------------------------------- reveal on scroll */

  var reveal = Array.prototype.slice.call(document.querySelectorAll("[data-reveal]"));
  if (reveal.length) {
    if (reduceMotion.matches || !("IntersectionObserver" in window)) {
      reveal.forEach(function (element) {
        element.classList.add("is-in");
      });
    } else {
      var observer = new IntersectionObserver(
        function (entries) {
          entries.forEach(function (entry) {
            if (!entry.isIntersecting) return;
            entry.target.classList.add("is-in");
            observer.unobserve(entry.target);
          });
        },
        { threshold: 0.12, rootMargin: "0px 0px -40px 0px" }
      );
      reveal.forEach(function (element) {
        observer.observe(element);
      });
    }
  }

  // The portfolio bundle builds on this: it needs the same icon helper the
  // contact button is rebuilt with after a send.
  window.portfolioUI = { icon: icon, setIcon: setIcon, onScroll: onScroll };
})();
