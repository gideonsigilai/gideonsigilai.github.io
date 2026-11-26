// Builds the two web apps into two self-contained HTML documents.
//
//   portfolio:  web/portfolio.html → templates/layout.html       → $OUT_DIR/portfolio.html
//   dashboard:  web/admin.html     → templates/admin_layout.html → $OUT_DIR/admin.html
//
// Each Bun entrypoint parses its stub, bundles and minifies everything it
// references, and hands the artifacts back in memory. Those artifacts are
// spliced into that app's shell, so the portfolio carries only the portfolio
// CSS and script and the dashboard carries only the dashboard CSS and the
// shared core script. Neither server ever issues a stylesheet, script, or
// favicon request. Public fonts are served separately by the same executable
// so the browser can cache them across pages.
//
// The placeholders ({{title}}, {{body}}, {{#nav}}…) are never touched: each
// file this writes is still a template, just a heavier one.
//
// Run: bun run web/build.ts <out-dir>   (build.rs does this automatically)

import { mkdir } from "node:fs/promises";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const root = dirname(here);
const outDir = process.argv[2] ? resolve(process.argv[2]) : join(here, "dist");

const APPS = [
  {
    name: "portfolio",
    entry: join(here, "portfolio.html"),
    shell: join(root, "templates", "layout.html"),
    output: join(outDir, "portfolio.html"),
  },
  {
    name: "admin",
    entry: join(here, "admin.html"),
    shell: join(root, "templates", "admin_layout.html"),
    output: join(outDir, "admin.html"),
  },
];

const iconPath = join(root, "static", "icon.svg");

// The exact lines build.ts is allowed to rewrite. A template edit that moves or
// renames one of these fails the build instead of silently shipping a document
// that still points at files the server no longer serves.
const MARKERS = {
  stylesheet: '<link rel="stylesheet" href="/static/site.css">',
  adminStylesheet: '<link rel="stylesheet" href="/static/admin.css">',
  script: '<script src="/static/site.js" defer></script>',
  adminScript: '<script src="/static/core.js" defer></script>',
  icon: '<link rel="icon" href="/static/icon.svg" type="image/svg+xml">',
};

// Placeholders each shell must still carry after inlining, per app.
const REQUIRED: Record<string, string[]> = {
  portfolio: ["{{body}}", "{{title}}", "{{#nav}}", "{{head_extra}}"],
  admin: ["{{body}}", "{{title}}", "{{#nav}}", "{{head_extra}}"],
};

const kb = (bytes: number) => `${(bytes / 1024).toFixed(1)} KB`;

const icon = Buffer.from(await Bun.file(iconPath).bytes()).toString("base64");

for (const app of APPS) {
  const result = await Bun.build({
    entrypoints: [app.entry],
    target: "browser",
    format: "iife",
    // Embedded by the Rust server and cached separately from the HTML bundle.
    external: ["/static/fonts/*"],
    minify: true,
  });

  if (!result.success) {
    for (const log of result.logs) console.error(log);
    throw new Error(`bun build failed for ${app.name}`);
  }

  const pick = (mediaType: string) => {
    const found = result.outputs.filter((output) => output.type.startsWith(mediaType));
    if (found.length !== 1) {
      throw new Error(`expected exactly one ${mediaType} output for ${app.name}, got ${found.length}`);
    }
    return found[0];
  };

  const css = await pick("text/css").text();
  const js = await pick("text/javascript").text();

  // An inline <script> or <style> would end at the first closing tag in its
  // body, and each shell is still rendered by the mustache pass in
  // src/template.rs, so a `{{` in either bundle would be read as a placeholder.
  for (const [name, body] of [["script", js], ["style", css]] as const) {
    const closer = `</${name}`;
    if (body.toLowerCase().includes(closer)) {
      throw new Error(`${app.name} ${name} bundle contains ${closer}, so it cannot be inlined`);
    }
    if (body.includes("{{")) {
      throw new Error(`${app.name} ${name} bundle contains "{{", which the template renderer would consume`);
    }
  }

  const inline = {
    stylesheet: `<style>${css}</style>`,
    adminStylesheet: `<style>${css}</style>`,
    script: `<script>${js}</script>`,
    adminScript: `<script>${js}</script>`,
    icon: `<link rel="icon" href="data:image/svg+xml;base64,${icon}" type="image/svg+xml">`,
  };

  let shell = await Bun.file(app.shell).text();

  // Each shell links the bundle that belongs to it: the portfolio links
  // site.css and site.js, the dashboard links admin.css and core.js. Both must
  // carry the icon marker.
  const markers =
    app.name === "admin"
      ? [MARKERS.adminStylesheet, MARKERS.adminScript, MARKERS.icon]
      : [MARKERS.stylesheet, MARKERS.script, MARKERS.icon];

  for (const marker of markers) {
    if (!shell.includes(marker)) {
      throw new Error(`templates that feed ${app.name} no longer contain: ${marker}`);
    }
  }

  for (const marker of markers) {
    // A replacer function keeps `$&` and friends in the bundle literal.
    const replacement =
      marker === MARKERS.stylesheet || marker === MARKERS.adminStylesheet
        ? inline.stylesheet
        : marker === MARKERS.icon
          ? inline.icon
          : inline.script;
    shell = shell.replace(marker, () => replacement);
  }

  // The dashboard shell's script tag carries `defer`; the marker list above
  // matched it exactly, so this is a no-op unless the shell drifts.
  for (const placeholder of REQUIRED[app.name]) {
    if (!shell.includes(placeholder)) {
      throw new Error(`${app.name} shell lost the ${placeholder} placeholder`);
    }
  }

  await mkdir(dirname(app.output), { recursive: true });
  await Bun.write(app.output, shell);

  console.log(
    `  ${app.name}.html  ${kb(shell.length)} inline` +
      ` (${kb(css.length)} css + ${kb(js.length)} js + ${kb(icon.length)} icon),` +
      ` ${kb(Bun.gzipSync(shell).length)} gzipped`,
  );
}
