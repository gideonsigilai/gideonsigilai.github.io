// Exports a static snapshot of the public portfolio for GitHub Pages hosting.
//
// Usage:
//   bun run web/export-pages.ts [out-dir] [source-origin] [public-origin]
//
//   out-dir        Directory to write into.  default: dist/pages
//   source-origin  Rust server that renders pages.  default: https://217.77.8.87:8052
//   public-origin  GitHub Pages URL used in canonical/sitemap rewrites.
//                  default: https://gideonsigilai.github.io
//
// What it exports:
//   - Every HTML page listed in /sitemap.xml
//   - /writing  (even if no posts exist)
//   - Auxiliary routes: rss.xml, feed.xml, robots.txt, manifest.webmanifest,
//     favicon.ico, and a rendered 404.html
//   - All same-origin binary assets referenced in the HTML
//     (/static/fonts/*, /images/*, /favicon.ico, /static/icon.svg)
//
// What it does NOT export:
//   - Private visitor threads (/messages/*)
//   - Admin or API routes
//   - Anything that requires a live server to serve correctly
//
// The script fails hard on any fetch error so a broken snapshot is never
// published silently.

import { mkdir, writeFile } from "node:fs/promises";
import { join, dirname } from "node:path";

const outDir = process.argv[2] ?? "dist/pages";
const sourceOrigin = (process.argv[3] ?? "https://217.77.8.87:8052").replace(/\/+$/, "");
const publicOrigin = (process.argv[4] ?? "https://gideonsigilai.github.io").replace(/\/+$/, "");

// API_URL in the rendered HTML is the HTTPS backend; we don't rewrite it.
// We only need to rewrite the page-level public origin in canonical/og/sitemap.
if (!sourceOrigin.startsWith("https://")) {
  console.error("source-origin must be HTTPS to prevent exporting plain HTTP API data");
  process.exit(1);
}

// Routes we know are always public. Sitemap adds more.
const STATIC_HTML_ROUTES = ["/", "/writing"];
const STATIC_ASSET_ROUTES = [
  "/rss.xml",
  "/feed.xml",
  "/robots.txt",
  "/manifest.webmanifest",
  "/favicon.ico",
  "/static/icon.svg",
];
const NOT_FOUND_ROUTE = "/this-path-does-not-exist-404";

// Asset URL prefixes allowed to be fetched from source (same-origin only).
const ALLOWED_ASSET_PREFIXES = [
  "/static/fonts/",
  "/images/",
  "/favicon.ico",
  "/static/icon.svg",
];

function routeToPath(route: string): string {
  if (route === "/") return "index.html";
  const clean = route.replace(/^\//, "");
  // Extensionless routes become dir/index.html; routes with extension kept.
  if (!clean.includes(".") || clean.endsWith("/")) {
    return `${clean.replace(/\/$/, "")}/index.html`;
  }
  return clean;
}

async function fetchText(route: string): Promise<string> {
  const url = `${sourceOrigin}${route}`;
  const res = await fetch(url, {
    headers: { accept: "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8" },
  });
  if (!res.ok) throw new Error(`${res.status} ${res.statusText} for ${url}`);
  return res.text();
}

async function fetchBinary(route: string): Promise<{ bytes: ArrayBuffer; contentType: string }> {
  const url = `${sourceOrigin}${route}`;
  const res = await fetch(url);
  if (!res.ok) throw new Error(`${res.status} ${res.statusText} for ${url}`);
  return { bytes: await res.arrayBuffer(), contentType: res.headers.get("content-type") ?? "" };
}

// Replace every origin the server embeds in canonical/og/sitemap URLs with
// the public Pages origin. We detect the server's BASE_URL by extracting the
// canonical from the first page (it may differ from sourceOrigin if the server
// was deployed with an older ecosystem.config.js using a different port/scheme).
let serverBaseUrl: string | null = null;
function rewriteOrigin(html: string): string {
  let out = html;
  // Replace the server's public BASE_URL with the public Pages origin if they
  // differ (e.g. if the server rendered with a local or HTTP origin).
  // CRITICAL: NEVER rewrite sourceOrigin (https://217.77.8.87:8052); that is the
  // backend API origin, which the browser must call from data-api for contact intake.
  if (serverBaseUrl && serverBaseUrl !== publicOrigin && serverBaseUrl !== sourceOrigin) {
    out = out.replaceAll(serverBaseUrl, publicOrigin);
  }
  return out;
}
function detectServerBase(html: string): string | null {
  const m = html.match(/rel="canonical"\s+href="(https?:\/\/[^/"]+(?::\d+)?)(?:\/[^"]*)?"/) ??
            html.match(/rel="canonical" href="(https?:\/\/[^/"]+(?::\d+)?)\/?"/);
  return m ? m[1] : null;
}

// Extract same-origin asset hrefs: fonts, images, icon.
function extractAssets(html: string): string[] {
  const found = new Set<string>();
  const patterns = [
    /href="(\/static\/fonts\/[^"]+)"/g,
    /href="(\/static\/icon\.svg[^"]*)"/g,
    /href="(\/favicon\.ico[^"]*)"/g,
    /src="(\/images\/[^"]+)"/g,
    /url\((\/static\/fonts\/[^)]+)\)/g,
  ];
  for (const pattern of patterns) {
    for (const match of html.matchAll(pattern)) {
      const path = match[1].split("?")[0].split("#")[0];
      if (ALLOWED_ASSET_PREFIXES.some((prefix) => path.startsWith(prefix))) {
        found.add(path);
      }
    }
  }
  return [...found];
}

// Parse sitemap.xml and return the route portion of each <loc> URL.
function parseSitemapRoutes(xml: string, _baseOrigin: string): string[] {
  const routes: string[] = [];
  for (const match of xml.matchAll(/<loc>([^<]+)<\/loc>/g)) {
    try {
      const url = new URL(match[1].trim());
      const route = url.pathname || "/";
      if (route.startsWith("/messages")) continue;
      routes.push(route);
    } catch {
      // ignore unparseable loc
    }
  }
  return routes;
}

async function ensureDir(path: string): Promise<void> {
  await mkdir(dirname(path), { recursive: true });
}

async function writeOut(relPath: string, content: string | ArrayBuffer): Promise<void> {
  const dest = join(outDir, relPath);
  await ensureDir(dest);
  if (typeof content === "string") {
    await writeFile(dest, content, "utf8");
  } else {
    await writeFile(dest, Buffer.from(content));
  }
  const size =
    typeof content === "string" ? Buffer.byteLength(content, "utf8") : content.byteLength;
  console.log(`  ${relPath.padEnd(52)} ${(size / 1024).toFixed(1)} KB`);
}

// --- main ---

console.log(`source  ${sourceOrigin}`);
console.log(`public  ${publicOrigin}`);
console.log(`output  ${outDir}`);
console.log("");

// Fetch sitemap to discover all public HTML routes.
const sitemapXml = await fetchText("/sitemap.xml");
// Detect the server's BASE_URL from the sitemap <loc> entries before rewriting.
const sitemapBaseDetect = sitemapXml.match(/<loc>(https?:\/\/[^/"]+(?::\d+)?)(?:\/[^<]*)?\/?<\/loc>/);
if (sitemapBaseDetect && sitemapBaseDetect[1] !== sourceOrigin) {
  serverBaseUrl = sitemapBaseDetect[1];
  console.log(`detected server BASE_URL  ${serverBaseUrl} → ${publicOrigin}`);
}
// Parse sitemap locations using whichever origin the server used.
const sitemapParseOrigin = serverBaseUrl ?? sourceOrigin;
const sitemapRoutes = parseSitemapRoutes(sitemapXml, sitemapParseOrigin);

const htmlRoutes = [...new Set([...STATIC_HTML_ROUTES, ...sitemapRoutes])].filter(
  (r) => !r.startsWith("/messages")
);
console.log(`Exporting ${htmlRoutes.length} HTML pages + assets`);
console.log("");

const allAssets = new Set<string>();

// Export HTML pages, detecting server BASE_URL from the first canonical tag.
for (const route of htmlRoutes) {
  const rawHtml = await fetchText(route);
  if (!serverBaseUrl) {
    const detected = detectServerBase(rawHtml);
    if (detected && detected !== sourceOrigin && detected !== publicOrigin) {
      serverBaseUrl = detected;
      console.log(`detected server BASE_URL  ${serverBaseUrl} → ${publicOrigin}`);
    }
  }
  const html = rewriteOrigin(rawHtml);
  for (const asset of extractAssets(html)) allAssets.add(asset);
  await writeOut(routeToPath(route), html);
}

// Export 404 (the server responds with a 404 status; we save the body).
{
  const url = `${sourceOrigin}${NOT_FOUND_ROUTE}`;
  const res = await fetch(url);
  if (res.status !== 404) {
    throw new Error(`Expected 404 from ${url}, got ${res.status}`);
  }
  const html = rewriteOrigin(await res.text());
  for (const asset of extractAssets(html)) allAssets.add(asset);
  await writeOut("404.html", html);
  console.log(`  404.html (from ${NOT_FOUND_ROUTE})`);
}

// Export static assets (text routes).
for (const route of STATIC_ASSET_ROUTES) {
  const { bytes, contentType } = await fetchBinary(route);
  await writeOut(routeToPath(route), bytes);
  void contentType; // used only for logging if needed
}

// Export binary assets found in HTML.
for (const route of allAssets) {
  const { bytes } = await fetchBinary(route);
  await writeOut(routeToPath(route), bytes);
}

// Write .nojekyll so GitHub Pages doesn't skip _prefixed paths.
await writeFile(join(outDir, ".nojekyll"), "", "utf8");
console.log("");
console.log(`Done. ${htmlRoutes.length} pages + ${allAssets.size} assets written to ${outDir}`);
