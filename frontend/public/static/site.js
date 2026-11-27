(function () {
  "use strict";

  // The shell-level behaviour (theme, header, menu sheet, reveal, icon helper)
  // lives in core.js, which is inlined ahead of this file. Both documents ship
  // it; only the portfolio needs what follows.
  var ui = window.portfolioUI || {};
  var onScroll = ui.onScroll || function () {};
  var sendIcon =
    ui.icon ||
    function () {
      return document.createTextNode("");
    };

  // JSON and contact intake belong to the backend, which may be separate from
  // a static GitHub Pages portfolio. The contact response supplies its live
  // conversation URL rather than assuming Pages can render a private thread.
  var apiBase = (document.documentElement.getAttribute("data-api") || "").replace(/\/$/, "");
  function api(path) {
    return apiBase + path;
  }

  /* ---------------------------------------------------- device & visit telemetry */

  (function reportVisit() {
    if (!apiBase) return;
    try {
      var width = window.innerWidth || (document.documentElement && document.documentElement.clientWidth) || 0;
      var height = window.innerHeight || (document.documentElement && document.documentElement.clientHeight) || 0;
      var screenW = (window.screen && window.screen.width) || 0;
      var screenH = (window.screen && window.screen.height) || 0;

      var device = "desktop";
      if (width <= 768 || /Mobi|Android|iPhone|iPod/i.test(navigator.userAgent || "")) {
        device = "mobile";
      } else if (width <= 1024 || /iPad|Tablet/i.test(navigator.userAgent || "")) {
        device = "tablet";
      }

      var payload = JSON.stringify({
        path: window.location.pathname || "/",
        referrer: document.referrer ? (function (r) {
          try { return new URL(r).hostname; } catch (e) { return ""; }
        })(document.referrer) : "",
        screen: screenW + "x" + screenH,
        viewport: width + "x" + height,
        device: device
      });

      var endpoint = api("/api/telemetry");
      if (navigator.sendBeacon) {
        navigator.sendBeacon(endpoint, new Blob([payload], { type: "application/json" }));
      } else if (window.fetch) {
        fetch(endpoint, {
          method: "POST",
          headers: { "content-type": "application/json" },
          body: payload,
          keepalive: true
        }).catch(function () {});
      }
    } catch (e) {
      // Telemetry must never interrupt the visitor's page experience.
    }
  })();

  /* ------------------------------------------------------ work filter chips */

  var filterGroup = document.querySelector("[data-filter-group]");
  if (filterGroup) {
    var cards = Array.prototype.slice.call(document.querySelectorAll(".project-card[data-category]"));
    var status = filterGroup.querySelector("[data-filter-status]");
    filterGroup.addEventListener("click", function (event) {
      var button = event.target.closest("[data-filter]");
      if (!button) return;
      var filter = button.getAttribute("data-filter");
      filterGroup.querySelectorAll("[data-filter]").forEach(function (item) {
        item.setAttribute("aria-pressed", String(item === button));
        item.classList.toggle("active", item === button);
      });
      var shown = 0;
      cards.forEach(function (card) {
        var visible = filter === "all" || card.getAttribute("data-category") === filter;
        card.hidden = !visible;
        if (visible) shown += 1;
      });
      if (status) status.textContent = "Showing " + shown + " of " + cards.length + " projects";
      onScroll();
    });
  }

  /* ---------------------------------------------------- journal search + tags */

  var journal = document.querySelector(".journal-grid");
  if (journal) {
    var posts = Array.prototype.slice.call(journal.querySelectorAll("[data-post]"));
    var search = document.querySelector("#journal-search, [data-journal-search]");
    var journalFilters = document.querySelector("[data-journal-filters]");
    var journalCount = document.querySelector("[data-journal-count]");
    var journalEmpty = document.querySelector("[data-journal-empty]");
    var selectedTag = "all";
    var categories = new Map();
    var notes = posts.map(function (post) {
      var tags = Array.prototype.slice.call(post.querySelectorAll("[data-journal-tags] .chip"))
        .map(function (chip) {
          var label = chip.textContent.trim();
          var key = label.toLocaleLowerCase();
          if (key && key !== "all" && !categories.has(key)) categories.set(key, label);
          return key;
        });
      return { element: post, tags: tags, text: post.textContent.toLocaleLowerCase() };
    });

    function filterNotes() {
      var query = search ? search.value.trim().toLocaleLowerCase() : "";
      var shown = 0;
      notes.forEach(function (note) {
        var visible = (selectedTag === "all" || note.tags.indexOf(selectedTag) !== -1) &&
          (!query || note.text.indexOf(query) !== -1);
        note.element.hidden = !visible;
        if (visible) shown += 1;
      });
      if (journalFilters) {
        journalFilters.querySelectorAll("[data-journal-filter]").forEach(function (button) {
          var active = button.getAttribute("data-journal-filter") === selectedTag;
          button.setAttribute("aria-pressed", String(active));
          button.classList.toggle("active", active);
        });
      }
      if (journalCount) journalCount.textContent = "Showing " + shown + " of " + notes.length + " notes";
      if (journalEmpty) journalEmpty.hidden = shown !== 0;
      onScroll();
    }

    if (journalFilters) {
      categories.forEach(function (label, key) {
        var button = document.createElement("button");
        button.type = "button";
        button.className = "chip";
        button.setAttribute("data-journal-filter", key);
        button.setAttribute("aria-pressed", "false");
        button.textContent = label;
        journalFilters.appendChild(button);
      });
      journalFilters.addEventListener("click", function (event) {
        var button = event.target.closest("[data-journal-filter]");
        if (!button || !journalFilters.contains(button)) return;
        selectedTag = button.getAttribute("data-journal-filter");
        filterNotes();
      });
    }
    if (search) search.addEventListener("input", filterNotes);
    document.querySelectorAll("[data-journal-reset]").forEach(function (button) {
      button.addEventListener("click", function (event) {
        event.preventDefault();
        selectedTag = "all";
        if (search) search.value = "";
        filterNotes();
        if (search) search.focus();
      });
    });
    filterNotes();
  }

  /* -------------------------------------------------------- toolkit explorer */

  // Chips ship with their name only. Descriptions and project links come from
  // /api/toolkit the first time a visitor explores, so the page stays small.
  var chips = document.querySelectorAll("[data-tool]");
  var panelTitle = document.getElementById("tool-name");
  var panelCopy = document.getElementById("tool-description");
  var panelLink = document.getElementById("tool-project");
  var panelHint = document.getElementById("tool-hint");
  var activeTool = null;
  var toolIndex = null;
  var toolRequest = null;
  var defaultHint = panelHint ? panelHint.textContent : "";

  function loadTools() {
    if (toolRequest) return toolRequest;
    toolRequest = fetch(api("/api/toolkit"), { headers: { accept: "application/json" } })
      .then(function (response) {
        if (!response.ok) throw new Error("toolkit unavailable");
        return response.json();
      })
      .then(function (payload) {
        toolIndex = {};
        (payload.tools || []).forEach(function (tool) {
          toolIndex[tool.name] = tool;
        });
        return toolIndex;
      })
      .catch(function () {
        // Let a later click retry rather than caching the failure.
        toolRequest = null;
        throw new Error("toolkit unavailable");
      });
    return toolRequest;
  }

  function clearTool() {
    activeTool = null;
    chips.forEach(function (chip) {
      chip.setAttribute("aria-pressed", "false");
    });
    if (panelTitle) panelTitle.textContent = "Every tool has a purpose.";
    if (panelCopy) {
      panelCopy.textContent =
        "Pick a technology to see where it fits in my work — each one earns its place.";
    }
    if (panelLink) panelLink.hidden = true;
    if (panelHint) panelHint.textContent = defaultHint;
  }

  function describe(tool) {
    if (panelCopy) {
      panelCopy.textContent = tool.description || "Used across the projects on this site.";
    }
    if (!panelLink) return;
    if (tool.project && tool.project_name) {
      panelLink.hidden = false;
      panelLink.href = "/work/" + tool.project;
      panelLink.textContent = "See it in " + tool.project_name;
    } else {
      panelLink.hidden = true;
    }
  }

  chips.forEach(function (chip) {
    chip.addEventListener("click", function () {
      var name = chip.getAttribute("data-tool");
      if (activeTool === name) {
        clearTool();
        return;
      }
      activeTool = name;
      chips.forEach(function (item) {
        item.setAttribute("aria-pressed", String(item === chip));
      });
      if (panelTitle) panelTitle.textContent = name;
      if (panelHint) panelHint.textContent = "Selected from the list — press again to clear.";

      if (toolIndex) {
        describe(toolIndex[name] || {});
        return;
      }
      if (panelCopy) panelCopy.textContent = "Loading notes…";
      loadTools()
        .then(function (index) {
          if (activeTool !== name) return;
          describe(index[name] || {});
        })
        .catch(function () {
          if (activeTool !== name || !panelCopy) return;
          panelCopy.textContent =
            "That description could not load. Refresh, or open a project to see the tool in context.";
        });
    });
  });

  /* ------------------------------------------------------------ contact form */

  var form = document.getElementById("contact-form");
  if (form) {
    var submit = form.querySelector('button[type="submit"]');
    var errorBox = form.querySelector("[data-form-error]");
    var success = document.getElementById("contact-success");
    var limits = { name: 80, email: 254, topic: 60, body: 4000 };

    function fieldError(name, message) {
      var field = form.querySelector('[name="' + name + '"]');
      var slot = form.querySelector('[data-error-for="' + name + '"]');
      if (field) {
        if (message) {
          field.setAttribute("aria-invalid", "true");
          field.setAttribute("aria-describedby", "contact-" + name + "-error");
        } else {
          field.removeAttribute("aria-invalid");
          field.removeAttribute("aria-describedby");
        }
      }
      if (slot) slot.textContent = message || "";
      return message;
    }

    function validate(values) {
      var first = null;
      var errors = {
        name: values.name.trim().length < 2 ? "Please tell me your name." : "",
        email: /^[^\s@]+@[^\s@]+\.[^\s@]{2,}$/.test(values.email.trim())
          ? ""
          : "That email address does not look right.",
        topic: values.topic.trim().length < 2 ? "Choose a topic." : "",
        body:
          values.body.trim().length < 20
            ? "A little more context helps — at least 20 characters."
            : ""
      };
      for (var key in errors) {
        if (fieldError(key, errors[key]) && !first) first = key;
      }
      if (first) {
        var target = form.querySelector('[name="' + first + '"]');
        if (target) target.focus();
      }
      return errors;
    }

    form.addEventListener("input", function (event) {
      var field = event.target;
      if (field.getAttribute("aria-invalid") === "true") fieldError(field.name, "");
    });

    form.addEventListener("submit", function (event) {
      event.preventDefault();
      var data = new FormData(form);
      var values = {
        name: String(data.get("name") || ""),
        email: String(data.get("email") || ""),
        topic: String(data.get("topic") || ""),
        body: String(data.get("body") || "")
      };
      if (values.name.length > limits.name) values.name = values.name.slice(0, limits.name);
      if (values.email.length > limits.email) values.email = values.email.slice(0, limits.email);
      if (values.body.length > limits.body) values.body = values.body.slice(0, limits.body);

      var errors = validate(values);
      if (errors.name || errors.email || errors.topic || errors.body) return;

      if (submit) {
        submit.disabled = true;
        submit.textContent = "Sending…";
      }
      if (errorBox) errorBox.hidden = true;

      fetch(api("/contact"), {
        method: "POST",
        headers: { "content-type": "application/json", accept: "application/json" },
        body: JSON.stringify({
          name: values.name,
          email: values.email,
          topic: values.topic,
          body: values.body,
          website: String(data.get("website") || "")
        })
      })
        .then(function (response) {
          return response.json().catch(function () {
            return {};
          }).then(function (payload) {
            if (!response.ok) throw new Error((payload.error && payload.error.message) || "The message could not be sent.");
            return payload;
          });
        })
        .then(function (payload) {
          var link = document.getElementById("contact-thread-link");
          if (link) {
            link.hidden = !payload.url;
            if (payload.url) link.href = payload.url;
          }
          if (success) {
            success.hidden = false;
            form.hidden = true;
            success.focus();
          }
        })
        .catch(function (error) {
          if (errorBox) {
            errorBox.textContent = error.message;
            errorBox.hidden = false;
          }
        })
        .then(function () {
          if (submit) {
            submit.disabled = false;
            submit.replaceChildren(document.createTextNode("Send message"));
            submit.appendChild(sendIcon("send"));
          }
        });
    });
  }

  /* --------------------------------------------------- clipboard and sharing */

  var toastTimer = 0;
  function toast(message) {
    var box = document.getElementById("toast");
    if (!box) return;
    clearTimeout(toastTimer);
    box.textContent = message;
    box.hidden = false;
    toastTimer = setTimeout(function () { box.hidden = true; }, 4000);
  }

  function manualCopy(button, text) {
    var dialog = button.closest("dialog");
    var fallback = dialog && dialog.querySelector(".copy-fallback");
    if (!fallback) {
      fallback = button.parentElement.querySelector("[data-manual-copy]");
      if (!fallback) {
        fallback = document.createElement("label");
        fallback.className = "copy-fallback";
        fallback.setAttribute("data-manual-copy", "");
        fallback.appendChild(document.createTextNode("Select and copy: "));
        var field = document.createElement("input");
        field.type = "text";
        field.readOnly = true;
        fallback.appendChild(field);
        button.insertAdjacentElement("afterend", fallback);
      }
    }
    var input = fallback.querySelector("input");
    if (!input) return;
    fallback.hidden = false;
    input.value = text;
    input.focus();
    input.select();
  }

  function copyText(button, text, message) {
    if (!text) {
      toast("There is nothing available to copy.");
      return;
    }
    var copied = function () { toast(message); };
    var failed = function () {
      manualCopy(button, text);
      toast("Automatic copying is unavailable. Select and copy the text instead.");
    };
    if (navigator.clipboard && navigator.clipboard.writeText && window.isSecureContext) {
      navigator.clipboard.writeText(text).then(copied, failed);
    } else {
      failed();
    }
  }

  var canonical = document.querySelector('link[rel="canonical"]');
  var pageUrl = new URL(window.location.href);
  if (canonical) {
    try {
      var publicUrl = new URL(canonical.href);
      if (publicUrl.protocol === "https:" || publicUrl.protocol === "http:") pageUrl = publicUrl;
    } catch (error) { /* Keep the actual page URL if the canonical is malformed. */ }
  }
  pageUrl.hash = "";
  var encodedUrl = encodeURIComponent(pageUrl.href);
  var encodedTitle = encodeURIComponent(document.title);
  var shareUrls = {
    linkedin: "https://www.linkedin.com/sharing/share-offsite/?url=" + encodedUrl,
    x: "https://twitter.com/intent/tweet?url=" + encodedUrl + "&text=" + encodedTitle,
    whatsapp: "https://wa.me/?text=" + encodedTitle + "%20" + encodedUrl,
    facebook: "https://www.facebook.com/sharer/sharer.php?u=" + encodedUrl,
    telegram: "https://t.me/share/url?url=" + encodedUrl + "&text=" + encodedTitle,
    email: "mailto:?subject=" + encodedTitle + "&body=" + encodedUrl
  };
  document.querySelectorAll("[data-share-platform]").forEach(function (link) {
    var destination = shareUrls[link.getAttribute("data-share-platform")];
    if (destination) link.href = destination;
  });
  document.querySelectorAll("[data-copy-page]").forEach(function (button) {
    button.addEventListener("click", function () {
      copyText(button, pageUrl.href, "Page link copied.");
    });
  });
  document.querySelectorAll("[data-copy-email]").forEach(function (button) {
    button.addEventListener("click", function () {
      var scope = button.closest("#contact-dialog, #contact") || document;
      var mail = scope.querySelector('a[href^="mailto:"]');
      var email = mail ? mail.getAttribute("href").slice(7).split("?")[0] : "";
      try { email = decodeURIComponent(email); } catch (error) { /* Keep readable raw text. */ }
      copyText(button, email, "Email address copied.");
    });
  });
  var copyButton = document.getElementById("thread-copy");
  if (copyButton) {
    copyButton.addEventListener("click", function () {
      // A private thread must copy its actual token URL, not the public canonical.
      copyText(copyButton, window.location.href, "Thread link copied.");
    });
  }

  /* ----------------------------------------------------------- native dialogs */

  function bindDialog(id, selector) {
    var dialog = document.getElementById(id);
    if (!dialog || typeof dialog.showModal !== "function") return;
    document.querySelectorAll(selector).forEach(function (opener) {
      opener.addEventListener("click", function (event) {
        event.preventDefault();
        closeMenu();
        if (!dialog.open) dialog.showModal();
      });
    });
    dialog.querySelectorAll("[data-close]").forEach(function (button) {
      button.addEventListener("click", function () { dialog.close(); });
    });
    dialog.addEventListener("click", function (event) {
      if (event.target !== dialog) return;
      var rect = dialog.getBoundingClientRect();
      if (event.clientX < rect.left || event.clientX > rect.right ||
          event.clientY < rect.top || event.clientY > rect.bottom) dialog.close();
    });
  }

  var menu = document.getElementById("menu-sheet");
  var menuButton = document.getElementById("menu-open");
  function closeMenu() {
    if (menu && menu.open) {
      if (typeof menu.close === "function") menu.close();
      else menu.removeAttribute("open");
    }
    if (menuButton) menuButton.setAttribute("aria-expanded", "false");
  }
  if (menu) {
    menu.addEventListener("click", function (event) {
      if (event.target.closest("a[href]")) closeMenu();
    });
    var desktop = window.matchMedia("(min-width: 821px)");
    desktop.addEventListener("change", function (event) { if (event.matches) closeMenu(); });
    if (desktop.matches) closeMenu();
  }
  bindDialog("contact-dialog", "[data-contact]");
  bindDialog("share-dialog", "[data-share]");

  /* ------------------------------------------------------ local time + motion */

  var clock = document.getElementById("nairobi-time");
  if (clock) {
    var clockFormat = new Intl.DateTimeFormat("en-GB", {
      timeZone: "Africa/Nairobi", hour: "2-digit", minute: "2-digit", hour12: false
    });
    var updateClock = function () {
      clock.textContent = clockFormat.format(new Date()) + " EAT";
    };
    updateClock();
    setInterval(function () { if (!document.hidden) updateClock(); }, 60000);
    document.addEventListener("visibilitychange", function () {
      if (!document.hidden) updateClock();
    });
  }
  document.querySelectorAll("[data-year]").forEach(function (element) {
    element.textContent = new Date().getFullYear();
  });

  var reducedMotion = window.matchMedia("(prefers-reduced-motion: reduce)");
  var compact = window.matchMedia("(max-width: 820px)");
  var spinAnimations = new Map();
  document.querySelectorAll(".spin-trigger").forEach(function (button) {
    button.addEventListener("click", function () {
      if (reducedMotion.matches || typeof button.animate !== "function") {
        toast("A little spark of curiosity.");
        return;
      }
      var previous = spinAnimations.get(button);
      if (previous) previous.cancel();
      spinAnimations.set(button, button.animate(
        [{ transform: "rotate(0deg)" }, { transform: "rotate(360deg)" }],
        { duration: 700, easing: "cubic-bezier(.2,.8,.2,1)" }
      ));
    });
  });

  var layers = Array.prototype.slice.call(document.querySelectorAll("[data-depth]"))
    .map(function (element) {
      return {
        element: element,
        anchor: element.closest(".project-art, .hero, .about, .contact, section"),
        speed: Number(element.getAttribute("data-depth"))
      };
    }).filter(function (layer) { return layer.anchor && Number.isFinite(layer.speed); });
  layers.forEach(function (layer) { layer.element.classList.add("parallax-layer"); });

  var homeSections = [];
  var navLinks = [];
  if (document.getElementById("work") && document.getElementById("about")) {
    homeSections = ["home", "work", "about", "toolkit", "writing", "contact"]
      .map(function (id) { return document.getElementById(id); })
      .filter(function (section) { return section; });
    navLinks = Array.prototype.slice.call(
      document.querySelectorAll("#site-header a[href], #menu-sheet a[href]")
    ).filter(function (link) {
      var href = link.getAttribute("href") || "";
      var isHome = (href === "/" || href === window.location.pathname) && !link.hash;
      return link.origin === window.location.origin && (link.hash || isHome) &&
        (link.pathname === "/" || link.pathname === window.location.pathname);
    });
  }

  var motionFrame = 0;
  function paintMotion() {
    motionFrame = 0;
    var viewport = window.innerHeight;
    var scrolled = window.scrollY;
    var rects = new Map();
    var active = "";
    // Measure all anchors before writing styles; frames only run after events.
    if (!reducedMotion.matches) {
      layers.forEach(function (layer) {
        if (!rects.has(layer.anchor)) rects.set(layer.anchor, layer.anchor.getBoundingClientRect());
      });
    }
    homeSections.forEach(function (section) {
      var rect = rects.get(section);
      if (!rect) rect = section.getBoundingClientRect();
      if (rect.top <= 155) active = section.id;
    });
    navLinks.forEach(function (link) {
      var linkSection = link.hash ? link.hash.slice(1) : "home";
      var current = linkSection === active;
      link.classList.toggle("active", current);
      if (current) link.setAttribute("aria-current", "location");
      else link.removeAttribute("aria-current");
    });
    layers.forEach(function (layer) {
      if (reducedMotion.matches) {
        layer.element.style.removeProperty("--parallax-y");
        return;
      }
      var rect = rects.get(layer.anchor);
      if (!rect.height || rect.bottom < -60 || rect.top > viewport + 60) return;
      var distance = layer.anchor.classList.contains("hero") ? Math.max(0, scrolled) :
        viewport / 2 - rect.top - rect.height / 2;
      var limit = layer.element.classList.contains("asterisk") ? 36 : 22;
      var offset = Math.max(-limit, Math.min(limit, distance * layer.speed)) *
        (compact.matches ? 0.35 : 1);
      layer.element.style.setProperty("--parallax-y", offset.toFixed(2) + "px");
    });
  }
  function scheduleMotion() {
    if (!motionFrame) motionFrame = requestAnimationFrame(paintMotion);
  }
  reducedMotion.addEventListener("change", function () {
    if (reducedMotion.matches) spinAnimations.forEach(function (animation) { animation.cancel(); });
    scheduleMotion();
  });
  if (layers.length || homeSections.length) {
    window.addEventListener("scroll", scheduleMotion, { passive: true });
    window.addEventListener("resize", scheduleMotion);
    window.addEventListener("pageshow", scheduleMotion);
    compact.addEventListener("change", scheduleMotion);
    var main = document.querySelector("main");
    if (main && "ResizeObserver" in window) new ResizeObserver(scheduleMotion).observe(main);
    if (document.fonts) document.fonts.ready.then(scheduleMotion);
    scheduleMotion();
  }

  // core.js owns theme switching; only the public shell's browser color differs.
  var themeMeta = document.querySelector('meta[name="theme-color"]');
  if (themeMeta) {
    var syncThemeColor = function () {
      var paper = getComputedStyle(document.documentElement).getPropertyValue("--paper").trim();
      if (paper) themeMeta.setAttribute("content", paper);
    };
    new MutationObserver(syncThemeColor).observe(document.documentElement, {
      attributes: true, attributeFilter: ["data-theme"]
    });
    requestAnimationFrame(syncThemeColor);
  }
})();
