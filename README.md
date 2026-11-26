# Portfolio — Gideon Sigilai

One Rust binary serving three origins from one database:

```text
:8050  portfolio.html   public site          ─┐
:8051  admin.html       dashboard (session)   ├─ portfolio.exe ──► SQLite
:8052  backend          JSON, contact intake  ─┘
```

No Node.js at runtime, no bundler in production. `build.rs` runs `web/build.ts` through Bun, which inlines each app's stylesheet, client script, and the icon into **two** HTML documents — one per front end — that the server embeds. HTML is rendered on the server, and every mutation is validated and authorised server-side. Each response is gzip/brotli compressed.

Portfolio text and collection records — profile, social links, projects, toolkit groups and tools, principles, milestones, and project filters — live in SQLite and are edited on the dashboard's **Portfolio** tab. Adding a fourth project or a ninth tool is a form, not a rebuild; only bundled assets such as the branded social-preview PNG require rebuilding. The home page ships the project grid and tool *names*; case studies, tool descriptions, and the About lists are available from the JSON API and the tool panel loads its copy on first interaction.

## Public design

The public UI follows `nw_design/gideon-portfolio/dist/`: cream and lime surfaces, DM Sans with Instrument Serif accents, sage/lilac/forest project artwork, a dark About section, a three-column toolkit, and an editorial journal. The existing `/work/{slug}` and `/writing` URLs and database-managed content remain in place; the dashboard retains its own design.

The conversation dialog submits to the existing backend and returns a private reply-thread link, rather than opening an email draft. Journal search and tag filters operate on published posts. Native dialogs support keyboard navigation and Escape, motion respects reduced-motion preferences, and the contact form remains available without JavaScript. Public pages provide sharing controls; private conversations do not.

Fonts and their SIL Open Font Licenses are bundled into the executable and served from `/static/fonts/` on the public origin. No Google Fonts connection is needed. The three WOFF2 files are cached separately from the HTML so navigating between pages does not download them again.

## Layout

```
src/
  main.rs            entry point: config, database, seed, three listeners
  state.rs           config, app state, rate limiter, error type
  db.rs              connection and idempotent schema migration
  content.rs         data access: posts, conversations, sessions
  text.rs            markdown → sanitised HTML, slugs, validation
  template.rs        small mustache-style renderer
  seed.rs            admin password bootstrap + starter essays
  seed_content.rs    first-run portfolio content (only on an empty database)
  portfolio.rs       content rows, ordering, field specs, generic CRUD
  views/             layout, page builders, icons, JSON-LD
  routes/
    mod.rs           the three routers, compression, icon/fonts, security headers
    pages.rs         home, work, writing, post, feeds
    api.rs           /api content endpoints
    contact.rs       contact form (backend) and visitor threads (portfolio)
    admin.rs         sign in, inbox, post editor, portfolio CRUD
    auth.rs          cookie sessions, CSRF, rate-limit identity
templates/           HTML views (embedded at compile time)
                     layout.html = portfolio shell, admin_layout.html = dashboard shell
static/              site.css + detail.css + site.js (portfolio),
                     base.css + admin.css (dashboard), core.js (shared), icon.svg,
                     fonts/ (self-hosted WOFF2 fonts and their licenses)
migrations/          SQLite schema
web/
  portfolio.html     Bun entry: portfolio stylesheet + core.js + site.js
  admin.html         Bun entry: dashboard stylesheet + core.js
  build.ts           inlines each app into its own HTML document
build.rs             runs web/build.ts into $OUT_DIR/{portfolio,admin}.html
build.ps1            builds dist\portfolio.exe
deploy.ps1           builds the Linux binary and deploys it over SSH, under pm2

## Build

Requires Bun: `cargo build` invokes `web/build.ts`, which produces one inlined HTML document per web app.

```powershell
.\build.ps1                 # tests, release build, stage in dist\
.\build.ps1 -Run            # ...then start the server
.\build.ps1 -SkipTests      # faster iteration
```

`dist\portfolio.exe` is self-contained: copy it anywhere and run it. It binds:

| Port | Document | Serves |
| --- | --- | --- |
| 8050 | `portfolio.html` | Home, work, writing, posts, feeds, and the `/messages/{token}` visitor thread. No dashboard routes at all. |
| 8051 | `admin.html` | Sign-in, inbox, writing, portfolio content. Nothing public links here. |
| 8052 | — | `/api/*` JSON, `POST /contact`, health. |

The database is created next to the executable.

The split is not cosmetic. Neither front end carries the other's CSS or script — the portfolio page contains no `.admin-*` rule at all — and the public listener cannot answer a dashboard request. A cookie is scoped by host, not port, so a browser does send the dashboard cookie to 8050 and 8052 on the same host — but neither surface has a route that reads it: they resolve no sessions and render no signed-in state. Give the dashboard its own host name in production and the cookie stops crossing origins at all.

## Run in development

```bash
export ADMIN_PASSWORD=a-long-password     # hashed on first boot, then ignored
cargo run
# portfolio  http://127.0.0.1:8050
# dashboard  http://127.0.0.1:8051
# backend    http://127.0.0.1:8052
```

Environment:

| Variable | Purpose |
| --- | --- |
| `SITE_BIND` | Portfolio listen address, default `127.0.0.1:8050`. |
| `ADMIN_BIND` | Dashboard listen address, default `127.0.0.1:8051`. |
| `API_BIND` | Backend listen address, default `127.0.0.1:8052`. |
| `BIND` | Legacy alias for `SITE_BIND`. |
| `API_URL` | Backend origin the browser calls, default `http://<API_BIND>`; named in the portfolio's CSP. |
| `ADMIN_URL` | Dashboard origin used for its canonical links, default `http://<ADMIN_BIND>`. |
| `BASE_URL` | Public URL used in feeds, sitemaps, canonicals. |
| `DATABASE_PATH` | SQLite file, default `data/portfolio.db` beside the executable. |
| `ADMIN_USERNAME` | Dashboard user, default `gideon`. |
| `ADMIN_PASSWORD` | Read once, on first boot, then hashed with argon2. |
| `ADMIN_PASSWORD_HASH` | Pre-computed hash; skips `ADMIN_PASSWORD`. |
| `TRUST_PROXY_IP` | Honour `X-Forwarded-For`; only behind a trusted proxy. |

## Security model

- **No mutation API.** Every write is an authenticated, CSRF-checked form post on 8051; the JSON endpoints on 8052 are read-only and contain only published, public content.
- **The public origin holds no dashboard code.** 8050 has no dashboard routes, no session handling, and no admin template in its document; `robots.txt` no longer needs to disallow a path that does not exist there.
- **Separate origins for the session.** The dashboard runs on its own port, and on its own host name in production. Neither the site nor the backend resolves sessions, so the dashboard cookie grants nothing if it arrives there.
- **Session tokens never reach JavaScript.** They live in an httpOnly, SameSite=Strict cookie. Only the CSRF value is readable, so forms can echo it back.
- **Double-submit CSRF** on every admin mutation, compared in constant time.
- **Argon2id password hashing**, constant-time username comparison, tighter rate limit on sign-in than on the contact form.
- **Untrusted text is sanitised** with `pulldown-cmark` + `ammonia` before it is stored, plus CSP and `X-Frame-Options: DENY` on every response.
- **Rate limiting** keyed by client address, with a honeypot and a 96 KB body cap on the contact form.

## Content

Everything is managed at the dashboard origin (`http://127.0.0.1:8051`):

| Tab | What it edits |
| --- | --- |
| Inbox | Visitor conversations and replies. |
| Writing | Blog posts, drafts, slugs, tags. |
| Portfolio | Profile, projects, toolkit, principles, milestones, filters, social links. |

The **Portfolio** tab lists a collection on the left and an editor on the right. Add as many rows as you like with **New**; reorder with the arrow buttons (tools move inside their group); delete from the editor. Each save is validated server-side and returns to the same row.

Three essays and the original portfolio content ship as seed data on an empty database. Seeding runs once: deleting a row or editing it never gets overwritten on restart.

Publishing a post or saving a project updates `/`, `/writing`, `/work/{slug}`, the RSS feed, and the sitemap on the next request: the three listeners share one process and one database.

## Content API

Read-only JSON on the backend origin (8052), sent with `Access-Control-Allow-Origin: *` and no credentials:

| Endpoint | Returns |
| --- | --- |
| `GET /api/site` | Profile, social links, project filters with counts, content counts. |
| `GET /api/projects` | Project cards (no long-form copy). |
| `GET /api/projects/{slug}` | One case study plus sibling cards. |
| `GET /api/toolkit` | Groups and tools, including the project each tool points at. |
| `GET /api/about` | Principles and milestones. |
| `GET /api/posts` | Published posts; `?tag=` filters. |

These exist so the browser can fetch one slice instead of shipping every case study in the initial HTML. The home page uses `/api/toolkit` when a visitor selects a tool chip, which keeps ~1.5 KB of descriptions out of the first response and off every other page. The origin comes from `<html data-api>`, so the same script works whichever port the backend ends up on.

## Search and image indexing

Public content and search metadata are rendered on the server, without requiring JavaScript:

- The home title and main heading identify the profile owner by name. Project and article bylines link back to the profile.
- JSON-LD connects the `Person`, `WebSite`, home `ProfilePage`, writing `CollectionPage`, project `CreativeWork`, and article `BlogPosting` records. Direct visits to a case study or article include the named author and public social profiles.
- Each visible project and published article has a lightweight SVG cover at `/images/work/{slug}.svg` or `/images/writing/{slug}.svg`. The pages use actual image elements with descriptive alt text, explicit dimensions, and lazy loading. Hidden projects and drafts return 404 from both page and image URLs.
- `/sitemap.xml` includes public pages and their cover images. Modification dates come from stored timestamps, not the current date; unknown dates are omitted. `/robots.txt` advertises the sitemap and permits public content and assets.
- Dashboard/API responses and private conversation paths send `X-Robots-Tag: noindex, nofollow`. Private pages also omit canonical/share metadata; unknown public pages return a real 404.
- Open Graph and Twitter cards use the embedded 1200 × 630 PNG at `/images/gideon-sigilai.png`. Its editable source is `static/gideon-sigilai.svg`; regenerate the PNG and rebuild when the branding changes. These covers are artwork, not portraits or product screenshots.

### Production search setup

1. Serve one preferred public HTTPS origin and set `BASE_URL` to it, without a trailing slash. Canonical, structured-data, feed, image-metadata, and sitemap URLs all use this setting. Redirect other public domain/scheme variants to that origin at the reverse proxy.
2. With the deployment script, pass `.\deploy.ps1 -BaseUrl https://your-domain.example`. The default remains the direct HTTP server address. This option sets canonical URLs only: configure DNS, TLS, redirects, and the HTTPS `API_URL` separately.
3. Verify ownership in Google Search Console and Bing Webmaster Tools. Submit `https://your-domain.example/sitemap.xml` and request indexing of the homepage and important pages. These account-level actions are not performed by the build.
4. Keep the full name and site URL consistent on public profiles, and link those profiles back to the site. Publish original case studies and writing with useful titles rather than repeating name variants as keywords.

These changes improve crawlability and identity signals, not a guaranteed ranking. “Gideon Sigilai” is more specific than “Gideon”; search engines control indexing, snippets, image selection, and positions.

## Rendering

Each response inlines its app's stylesheet, client script, and icon at build time, so a page needs no follow-up CSS, JS, or favicon request. The portfolio additionally loads its self-hosted fonts from the same executable, with a one-day browser cache; two fonts are preloaded for the initial hero. The dashboard does not load these fonts. `CompressionLayer` negotiates gzip, brotli, or deflate for HTML and sets `Vary: accept-encoding`.

## Deploying

Bind each port to `0.0.0.0`, give the dashboard its own host name, set `BASE_URL`, `API_URL`, and `ADMIN_URL` to the matching HTTPS URLs, and put a TLS-terminating reverse proxy in front. `data/` beside the executable holds the SQLite database — keep it on a persistent volume. Vercel cannot run this binary; use a host that runs long-lived processes (Fly.io, Railway, Render, or a VPS).

`deploy.ps1` does that for one specific host: it cross-builds for
`x86_64-unknown-linux-musl` (static, so no glibc version to match), pushes the
binary and an `ecosystem.config.js` to `gidi@217.77.8.87:~/portfolio/` with
`sshpass`, starts it under pm2, opens 8050 in ufw, and verifies the result.

```powershell
.\deploy.ps1                 # build for Linux, deploy, open 8050, verify
.\deploy.ps1 -OpenApiPort    # also open 8052 so the contact form works
.\deploy.ps1 -SkipBuild      # redeploy the binary already in dist\linux
```

The cross-build needs `cargo-zigbuild` and `zig` (`pip install ziglang` puts it
in `%LOCALAPPDATA%\zig`); zig supplies the musl C toolchain the bundled SQLite
is compiled with. `rustup target add x86_64-unknown-linux-musl` supplies std, and
the script adds it when it is missing.

Only 8050 is public. The dashboard (8051) and the backend (8052) bind to
loopback, so reach them through a tunnel:

```bash
ssh -L 8051:127.0.0.1:8051 -L 8052:127.0.0.1:8052 gidi@217.77.8.87
```

The dashboard password is `ADMIN_PASSWORD` in the pm2 environment; the app hashes
it on first boot and ignores the plaintext afterwards. With 8052 closed the
portfolio's contact form and tool panel cannot reach the backend, and the form
reports that the message could not be sent. Pass `-OpenApiPort` when that has to
work for visitors.
