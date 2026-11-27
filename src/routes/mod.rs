use axum::{
    http::{header, HeaderName, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Router,
};
use tower_http::{
    compression::{CompressionLayer, CompressionLevel},
    limit::RequestBodyLimitLayer,
    set_header::SetResponseHeaderLayer,
    timeout::TimeoutLayer,
    trace::TraceLayer,
};

pub mod admin;
pub mod api;
pub mod auth;
pub mod contact;
pub mod images;
pub mod pages;

use crate::state::AppState;

// Styles and scripts are inlined. Fonts are embedded separately so browsers can
// cache them across pages without adding their bytes to every HTML response.
pub const ICON: &str = include_str!("../../static/icon.svg");

const MAX_BODY_BYTES: usize = 96 * 1024;

async fn static_icon() -> Response {
    (
        StatusCode::OK,
        [
            (
                header::CONTENT_TYPE,
                HeaderValue::from_static("image/svg+xml"),
            ),
            // The URL carries no content hash, so a long immutable lifetime
            // would strand returning visitors on a stale icon.
            (
                header::CACHE_CONTROL,
                HeaderValue::from_static("public, max-age=86400"),
            ),
        ],
        ICON,
    )
        .into_response()
}

async fn static_font(axum::extract::Path(name): axum::extract::Path<String>) -> Response {
    let (media_type, bytes): (&str, &'static [u8]) = match name.as_str() {
        "dm-sans-latin.woff2" => (
            "font/woff2",
            include_bytes!("../../static/fonts/dm-sans-latin.woff2"),
        ),
        "instrument-serif-latin.woff2" => (
            "font/woff2",
            include_bytes!("../../static/fonts/instrument-serif-latin.woff2"),
        ),
        "instrument-serif-italic-latin.woff2" => (
            "font/woff2",
            include_bytes!("../../static/fonts/instrument-serif-italic-latin.woff2"),
        ),
        "DM-Sans-OFL.txt" => (
            "text/plain; charset=utf-8",
            include_bytes!("../../static/fonts/DM-Sans-OFL.txt"),
        ),
        "Instrument-Serif-OFL.txt" => (
            "text/plain; charset=utf-8",
            include_bytes!("../../static/fonts/Instrument-Serif-OFL.txt"),
        ),
        _ => return StatusCode::NOT_FOUND.into_response(),
    };
    (
        [
            (header::CONTENT_TYPE, media_type),
            (header::CACHE_CONTROL, "public, max-age=86400"),
        ],
        bytes,
    )
        .into_response()
}

async fn health(axum::extract::State(state): axum::extract::State<AppState>) -> Response {
    match sqlx::query("SELECT 1").execute(&state.db).await {
        Ok(_) => (StatusCode::OK, "{\"status\":\"ok\"}").into_response(),
        Err(_) => (StatusCode::SERVICE_UNAVAILABLE, "{\"status\":\"degraded\"}").into_response(),
    }
}

/// Response hardening every surface shares.
fn hardened<S: Clone + Send + Sync + 'static>(router: Router<S>) -> Router<S> {
    router
        .layer(SetResponseHeaderLayer::overriding(
            header::X_CONTENT_TYPE_OPTIONS,
            HeaderValue::from_static("nosniff"),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            header::X_FRAME_OPTIONS,
            HeaderValue::from_static("DENY"),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            header::REFERRER_POLICY,
            HeaderValue::from_static("strict-origin-when-cross-origin"),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            HeaderName::from_static("permissions-policy"),
            HeaderValue::from_static("camera=(), microphone=(), geolocation=(), payment=()"),
        ))
        .layer(RequestBodyLimitLayer::new(MAX_BODY_BYTES))
        .layer(TimeoutLayer::with_status_code(
            StatusCode::REQUEST_TIMEOUT,
            std::time::Duration::from_secs(20),
        ))
        .layer(TraceLayer::new_for_http())
}

/// Brotli/gzip/deflate for the surfaces that answer HTML. A page is ~150 KB of
/// markup and inline CSS; quality 6 sends it as ~30 KB without burning CPU.
fn compressed<S: Clone + Send + Sync + 'static>(router: Router<S>) -> Router<S> {
    router.layer(CompressionLayer::new().quality(CompressionLevel::Precise(6)))
}

/// Token-bearing conversations and health checks are not search content.
async fn private_page_robots(
    request: axum::extract::Request,
    next: axum::middleware::Next,
) -> Response {
    let path = request.uri().path();
    let private = path == "/messages"
        || path.starts_with("/messages/")
        || path == "/health"
        || path.starts_with("/health/");
    let mut response = next.run(request).await;
    if private {
        response.headers_mut().insert(
            HeaderName::from_static("x-robots-tag"),
            HeaderValue::from_static("noindex, nofollow"),
        );
    }
    response
}

// ------------------------------------------------------------------ portfolio

/// Port 8050. Public pages only: no dashboard routes, no session handling, and
/// no admin template in the embedded document. Contact submissions still leave
/// this process for the backend, so the public surface holds no write path.
pub fn site(state: AppState) -> Router {
    let csp = site_csp(&state.config.api_url);

    compressed(
        hardened(
            Router::new()
                .route("/", get(pages::home))
                .route("/work/{slug}", get(pages::work_detail))
                .route("/writing", get(pages::writing))
                .route("/writing/{slug}", get(pages::post_detail))
                .route(
                    "/messages/{token}",
                    get(contact::show_thread).post(contact::post_thread),
                )
                .route("/rss.xml", get(pages::rss))
                .route("/feed.xml", get(pages::rss))
                .route("/sitemap.xml", get(pages::sitemap))
                .route("/robots.txt", get(pages::robots))
                .route("/manifest.webmanifest", get(pages::manifest))
                .route("/images/gideon-sigilai.png", get(images::social))
                .route("/images/creator.webp", get(images::creator))
                .route("/images/flutter-logo.png", get(images::flutter_logo))
                .route("/images/rust-logo.png", get(images::rust_logo))
                .route("/images/rust-logo.svg", get(images::rust_logo_svg))
                .route("/images/work/{filename}", get(images::project))
                .route("/images/plant_doc.jpg", get(images::plant_doc))
                .route("/images/meioufei.png", get(images::meioufei))
                .route("/images/onekitty.jpg", get(images::onekitty))
                .route(
                    "/images/plant_doc_showcase.png",
                    get(images::plant_showcase),
                )
                .route("/images/onechurch.jpg", get(images::onechurch))
                .route(
                    "/images/onechurch_showcase.jpg",
                    get(images::onechurch_showcase),
                )
                .route("/images/rust-cover.png", get(images::rust_cover))
                .route("/images/flutter-cover.png", get(images::flutter_cover))
                .route("/images/mindpocket.jpg", get(images::mindpocket))
                .route(
                    "/images/mindpocket_showcase.svg",
                    get(images::mindpocket_showcase),
                )
                .route("/images/writing/{filename}", get(images::post))
                .route("/health", get(health))
                .route("/favicon.ico", get(static_icon))
                .route("/static/icon.svg", get(static_icon))
                .route("/static/fonts/{name}", get(static_font))
                .fallback(pages::not_found_handler),
        )
        .layer(SetResponseHeaderLayer::if_not_present(
            header::CONTENT_SECURITY_POLICY,
            HeaderValue::from_str(&csp).expect("CSP is ASCII"),
        )),
    )
    .layer(axum::middleware::from_fn(private_page_robots))
    .with_state(state)
}

/// The portfolio's Content-Security-Policy. The backend origin has to appear in
/// both `connect-src` (the tool-chips fetch) and `form-action` (the contact
/// post), or the browser blocks the split before the request is made.
fn site_csp(api_url: &str) -> String {
    format!(
        "default-src 'self'; script-src 'self' 'unsafe-inline'; style-src 'self' 'unsafe-inline'; \
         img-src 'self' data:; font-src 'self'; connect-src 'self' {api_url}; \
         form-action 'self' {api_url}; base-uri 'self'; frame-ancestors 'none'; object-src 'none'"
    )
}

// ------------------------------------------------------------------ dashboard

/// Port 8051. Sign-in, inbox, writing, and portfolio content editing.
///
/// A cookie is scoped by host, not by port, so a browser does send the dashboard
/// session to 8050 and 8052 on the same host. Neither of those surfaces has a
/// route that reads it: they resolve no sessions and expose no signed-in state,
/// so a stolen cookie is worthless there. On a real deployment the dashboard
/// gets its own host name, and the cookie stops crossing origins entirely.
pub fn admin(state: AppState) -> Router {
    let dashboard = Router::new()
        .route("/", get(admin::inbox))
        .route("/logout", post(admin::logout))
        .route("/posts", get(admin::posts).post(admin::create_post))
        .route("/posts/{id}", post(admin::update_existing_post))
        .route("/posts/{id}/delete", post(admin::destroy_post))
        .route("/content", get(admin::content))
        .route("/content/profile", post(admin::profile_save))
        .route("/content/{section}", post(admin::content_save))
        .route("/content/{section}/{id}", post(admin::content_update))
        .route(
            "/content/{section}/{id}/delete",
            post(admin::content_delete),
        )
        .route("/content/{section}/{id}/move", post(admin::content_move))
        .route("/conversations/{id}/reply", post(admin::reply))
        .route("/conversations/{id}/status", post(admin::set_status))
        .route("/session", get(admin::status_check))
        .route("/analytics", get(admin::analytics_view))
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            auth::admin_gate,
        ));

    compressed(
        hardened(
            Router::new()
                .route("/login", get(admin::show_login).post(admin::login))
                .merge(dashboard)
                .route("/health", get(health))
                .route("/favicon.ico", get(static_icon))
                .route("/static/icon.svg", get(static_icon))
                .fallback(pages::not_found_handler),
        )
        .layer(SetResponseHeaderLayer::overriding(
            header::CONTENT_SECURITY_POLICY,
            HeaderValue::from_static(
                "default-src 'self'; script-src 'self' 'unsafe-inline'; \
                 style-src 'self' 'unsafe-inline'; img-src 'self' data:; font-src 'self'; \
                 connect-src 'self'; form-action 'self'; base-uri 'self'; \
                 frame-ancestors 'none'; object-src 'none'",
            ),
        )),
    )
    .layer(SetResponseHeaderLayer::overriding(
        HeaderName::from_static("x-robots-tag"),
        HeaderValue::from_static("noindex, nofollow"),
    ))
    .with_state(state)
}

// -------------------------------------------------------------------- backend

/// Port 8052. Read-only JSON, contact intake, and visitor reply threads. No
/// dashboard routes and no session cookies: this surface cannot mutate content.
pub fn api(state: AppState) -> Router {
    hardened(
        Router::new()
            .route("/api/site", get(api::site))
            .route("/api/projects", get(api::projects))
            .route("/api/projects/{slug}", get(api::project))
            .route("/api/toolkit", get(api::toolkit))
            .route("/api/about", get(api::about))
            .route("/api/posts", get(pages::json_posts))
            .route("/api/posts/{slug}", get(pages::json_post))
            .route("/contact", post(contact::submit_contact))
            .route("/api/telemetry", post(api::record_telemetry))
            .route("/health", get(health))
            .route("/favicon.ico", get(static_icon))
            .route("/static/icon.svg", get(static_icon))
            .route("/images/gideon-sigilai.png", get(images::social))
            .route("/images/creator.webp", get(images::creator))
            .route("/images/flutter-logo.png", get(images::flutter_logo))
            .route("/images/rust-logo.png", get(images::rust_logo))
            .route("/images/rust-logo.svg", get(images::rust_logo_svg))
            .route("/images/work/{filename}", get(images::project))
            .route("/images/plant_doc.jpg", get(images::plant_doc))
            .route("/images/meioufei.png", get(images::meioufei))
            .route("/images/onekitty.jpg", get(images::onekitty))
            .route("/images/plant_doc_showcase.png", get(images::plant_showcase))
            .route("/images/onechurch.jpg", get(images::onechurch))
            .route("/images/onechurch_showcase.jpg", get(images::onechurch_showcase))
            .route("/images/rust-cover.png", get(images::rust_cover))
            .route("/images/flutter-cover.png", get(images::flutter_cover))
            .route("/images/mindpocket.jpg", get(images::mindpocket))
            .route("/images/mindpocket_showcase.svg", get(images::mindpocket_showcase))
            .route("/images/writing/{filename}", get(images::post))
            .fallback(api::not_found)
            // Browsers preflight a cross-origin JSON post. Axum answers a known
            // path with 405 for an unregistered method, so OPTIONS is lifted to
            // a no-content reply here as well as in the fallback.
            .method_not_allowed_fallback(api::preflight),
    )
    .layer(SetResponseHeaderLayer::overriding(
        header::ACCESS_CONTROL_ALLOW_ORIGIN,
        HeaderValue::from_static("*"),
    ))
    .layer(SetResponseHeaderLayer::overriding(
        header::ACCESS_CONTROL_ALLOW_METHODS,
        HeaderValue::from_static("GET, POST, OPTIONS"),
    ))
    .layer(SetResponseHeaderLayer::overriding(
        header::ACCESS_CONTROL_ALLOW_HEADERS,
        HeaderValue::from_static("content-type, accept"),
    ))
    .layer(SetResponseHeaderLayer::overriding(
        HeaderName::from_static("x-robots-tag"),
        HeaderValue::from_static("noindex, nofollow"),
    ))
    .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn portfolio_csp_names_the_backend_origin_for_both_directives() {
        let csp = site_csp("https://api.example.test");
        assert!(
            csp.contains("connect-src 'self' https://api.example.test"),
            "{csp}"
        );
        assert!(
            csp.contains("form-action 'self' https://api.example.test"),
            "{csp}"
        );
        // The old single-origin policy would silently break the contact form.
        assert!(csp.contains("frame-ancestors 'none'"), "{csp}");
    }
}
