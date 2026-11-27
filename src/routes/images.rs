use std::fmt::Write;

use axum::{
    extract::{Path, State},
    http::{header, StatusCode},
    response::{IntoResponse, Response},
};

use crate::{
    content::published_by_slug,
    portfolio::{self, ProfileRow},
    state::{ApiError, ApiResult, AppState},
    template::escape_html,
};

pub async fn social() -> Response {
    (
        [
            (header::CONTENT_TYPE, "image/png"),
            (header::CACHE_CONTROL, "public, max-age=86400"),
        ],
        include_bytes!("../../static/gideon-sigilai.png").as_slice(),
    )
        .into_response()
}

pub async fn creator() -> Response {
    (
        [
            (header::CONTENT_TYPE, "image/png"),
            (header::CACHE_CONTROL, "public, max-age=86400"),
        ],
        include_bytes!("../../images/creator.png").as_slice(),
    )
        .into_response()
}

pub async fn project(State(state): State<AppState>, Path(filename): Path<String>) -> Response {
    let Some(slug) = filename.strip_suffix(".svg") else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let Some(project) = portfolio::visible_project(&state.db, slug).await else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let profile = portfolio::profile(&state.db)
        .await
        .unwrap_or_else(ProfileRow::fallback);
    svg_response(cover(
        &format!("{} — project artwork by {}", project.name, profile.name),
        &project.summary,
        &project.kicker,
        &project.tagline,
        &project.name,
        &profile.name,
        &project.accent,
    ))
}

pub async fn post(
    State(state): State<AppState>,
    Path(filename): Path<String>,
) -> ApiResult<Response> {
    let Some(slug) = filename.strip_suffix(".svg") else {
        return Ok(StatusCode::NOT_FOUND.into_response());
    };
    let Some(post) = published_by_slug(&state.db, slug)
        .await
        .map_err(ApiError::internal)?
    else {
        return Ok(StatusCode::NOT_FOUND.into_response());
    };
    let profile = portfolio::profile(&state.db)
        .await
        .unwrap_or_else(ProfileRow::fallback);
    let tags = post.tags();
    let accent = match tags.first().map(String::as_str) {
        Some("Flutter") => "lime",
        Some("Machine Learning" | "AI") => "sky",
        _ => "clay",
    };
    Ok(svg_response(cover(
        &format!("Cover artwork for {} by {}", post.title, profile.name),
        &post.excerpt,
        tags.first()
            .map(String::as_str)
            .unwrap_or("Engineering notes"),
        &post.title,
        "Notes from the build",
        &profile.name,
        accent,
    )))
}

fn svg_response(body: String) -> Response {
    (
        [
            (header::CONTENT_TYPE, "image/svg+xml; charset=utf-8"),
            // Stable URLs must reflect publishing, visibility, and content edits.
            (header::CACHE_CONTROL, "no-cache"),
            (
                header::CONTENT_SECURITY_POLICY,
                "default-src 'none'; style-src 'unsafe-inline'; sandbox",
            ),
        ],
        body,
    )
        .into_response()
}

/// Small vector covers, not photographs or product screenshots. All labels come
/// from the same published records as the landing pages; there is no JS fetch.
fn cover(
    title: &str,
    description: &str,
    kicker: &str,
    heading: &str,
    label: &str,
    author: &str,
    accent: &str,
) -> String {
    let (paper, ink, panel) = match accent {
        "lime" => ("#dce8c5", "#20261b", "#edf2e4"),
        "sky" => ("#263f37", "#edf2d9", "#355247"),
        _ => ("#dedcf1", "#26233c", "#eeedf7"),
    };
    let heading = text_lines(heading, 26, 3);
    format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="1200" height="630" viewBox="0 0 1200 630" role="img" aria-labelledby="title description">
<title id="title">{title}</title><desc id="description">{description}</desc>
<rect width="1200" height="630" fill="{paper}"/>
<g fill="{ink}">
<text x="64" y="76" font-family="Arial,sans-serif" font-size="18" letter-spacing="2">{kicker}</text>
<text x="64" y="220" font-family="Georgia,serif" font-style="italic" font-size="64">{heading}</text>
<text x="64" y="563" font-family="Arial,sans-serif" font-size="22">{author}</text>
</g>
<g transform="translate(896 145) rotate(8 100 150)">
<rect width="200" height="300" rx="20" fill="{panel}" stroke="{ink}" stroke-opacity=".45" stroke-width="2"/>
<path d="M70 110h60m-30-30v60" stroke="{ink}" stroke-width="3"/>
<path d="M50 216h100m-76 20h52" stroke="{ink}" stroke-opacity=".35" stroke-width="5" stroke-linecap="round"/>
</g>
<text x="994" y="506" text-anchor="middle" font-family="Arial,sans-serif" font-size="16" fill="{ink}">{label}</text>
<path d="M64 597h1072" stroke="{ink}" stroke-opacity=".2"/>
</svg>"##,
        title = escape_html(title),
        description = escape_html(description),
        kicker = escape_html(prefix(kicker, 70)),
        author = escape_html(author),
        label = escape_html(prefix(label, 28)),
    )
}

/// Bound visible text to the cover, keeping the unabridged text in title/desc.
fn text_lines(text: &str, columns: usize, rows: usize) -> String {
    let mut out = String::new();
    let mut remaining = text.trim();
    for row in 0..rows {
        if remaining.is_empty() {
            break;
        }
        let mut end = prefix(remaining, columns).len();
        if end < remaining.len() {
            end = remaining[..end]
                .rfind(' ')
                .filter(|&index| index > 0)
                .unwrap_or(end);
        }
        let line = &remaining[..end];
        remaining = remaining[end..].trim_start();
        let ellipsis = if row + 1 == rows && !remaining.is_empty() {
            "…"
        } else {
            ""
        };
        let _ = write!(
            out,
            r#"<tspan x="64" dy="{}">{}{ellipsis}</tspan>"#,
            if row == 0 { 0 } else { 78 },
            escape_html(line),
        );
    }
    out
}

pub async fn plant_doc() -> Response {
    const BYTES: &[u8] = include_bytes!("../../images/plant_doc.jpg");
    (
        [
            (header::CONTENT_TYPE, "image/jpeg"),
            (header::CACHE_CONTROL, "public, max-age=31536000, immutable"),
        ],
        BYTES,
    )
        .into_response()
}
pub async fn meioufei() -> Response {
    const BYTES: &[u8] = include_bytes!("../../images/meioufei.png");
    (
        [
            (header::CONTENT_TYPE, "image/png"),
            (header::CACHE_CONTROL, "public, max-age=31536000, immutable"),
        ],
        BYTES,
    )
        .into_response()
}
pub async fn onekitty() -> Response {
    const BYTES: &[u8] = include_bytes!("../../images/onekitty.jpg");
    (
        [
            (header::CONTENT_TYPE, "image/jpeg"),
            (header::CACHE_CONTROL, "public, max-age=31536000, immutable"),
        ],
        BYTES,
    )
        .into_response()
}
pub async fn plant_showcase() -> Response {
    const BYTES: &[u8] = include_bytes!("../../images/plant_doc_showcase.png");
    (
        [
            (header::CONTENT_TYPE, "image/png"),
            (header::CACHE_CONTROL, "public, max-age=31536000, immutable"),
        ],
        BYTES,
    )
        .into_response()
}
pub async fn onechurch() -> Response {
    const BYTES: &[u8] = include_bytes!("../../images/onechurch.jpg");
    (
        [
            (header::CONTENT_TYPE, "image/jpeg"),
            (header::CACHE_CONTROL, "public, max-age=31536000, immutable"),
        ],
        BYTES,
    )
        .into_response()
}
pub async fn onechurch_showcase() -> Response {
    const BYTES: &[u8] = include_bytes!("../../images/onechurch_showcase.jpg");
    (
        [
            (header::CONTENT_TYPE, "image/jpeg"),
            (header::CACHE_CONTROL, "public, max-age=31536000, immutable"),
        ],
        BYTES,
    )
        .into_response()
}
pub async fn rust_cover() -> Response {
    const BYTES: &[u8] = include_bytes!("../../images/rust-cover.png");
    (
        [
            (header::CONTENT_TYPE, "image/png"),
            (header::CACHE_CONTROL, "public, max-age=31536000, immutable"),
        ],
        BYTES,
    )
        .into_response()
}
pub async fn flutter_cover() -> Response {
    const BYTES: &[u8] = include_bytes!("../../images/flutter-cover.png");
    (
        [
            (header::CONTENT_TYPE, "image/png"),
            (header::CACHE_CONTROL, "public, max-age=31536000, immutable"),
        ],
        BYTES,
    )
        .into_response()
}

pub async fn mindpocket() -> Response {
    const BYTES: &[u8] = include_bytes!("../../images/mindpocket.jpg");
    (
        [
            (header::CONTENT_TYPE, "image/jpeg"),
            (header::CACHE_CONTROL, "public, max-age=31536000, immutable"),
        ],
        BYTES,
    )
        .into_response()
}

pub async fn flutter_logo() -> Response {
    const BYTES: &[u8] = include_bytes!("../../images/flutter-logo.png");
    (
        [
            (header::CONTENT_TYPE, "image/png"),
            (header::CACHE_CONTROL, "public, max-age=31536000, immutable"),
        ],
        BYTES,
    )
        .into_response()
}

pub async fn rust_logo() -> Response {
    const BYTES: &[u8] = include_bytes!("../../images/rust-logo.png");
    (
        [
            (header::CONTENT_TYPE, "image/png"),
            (header::CACHE_CONTROL, "public, max-age=31536000, immutable"),
        ],
        BYTES,
    )
        .into_response()
}

pub async fn rust_logo_svg() -> Response {
    const BYTES: &[u8] = include_bytes!("../../images/rust-logo.svg");
    (
        [
            (header::CONTENT_TYPE, "image/svg+xml"),
            (header::CACHE_CONTROL, "public, max-age=31536000, immutable"),
        ],
        BYTES,
    )
        .into_response()
}
pub async fn mindpocket_showcase() -> Response {
    const BYTES: &[u8] = include_bytes!("../../images/mindpocket_showcase.svg");
    (
        [
            (header::CONTENT_TYPE, "image/svg+xml; charset=utf-8"),
            (header::CACHE_CONTROL, "public, max-age=31536000, immutable"),
        ],
        BYTES,
    )
        .into_response()
}

fn prefix(text: &str, chars: usize) -> &str {
    &text[..text
        .char_indices()
        .nth(chars)
        .map_or(text.len(), |(index, _)| index)]
}
