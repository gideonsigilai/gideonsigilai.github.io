//! Read-only JSON for the parts of the site that are fetched on demand.
//!
//! The public pages are still server-rendered; these endpoints exist so the
//! browser (and any tooling) can pull one slice of content instead of shipping
//! every case study and tool description inside the initial HTML.

use std::time::Duration;

use axum::{
    extract::{Path, State},
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde_json::{json, Value};

use crate::analytics::{self, TelemetryPayload};
use crate::portfolio::{self, ProjectRow, Site, ToolRow};
use crate::routes::auth::client_ip;
use crate::state::AppState;

fn cached(payload: Value) -> Response {
    (
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "application/json; charset=utf-8"),
            (header::CACHE_CONTROL, "public, max-age=60"),
        ],
        Json(payload),
    )
        .into_response()
}

fn missing() -> Response {
    (
        StatusCode::NOT_FOUND,
        [(header::CONTENT_TYPE, "application/json; charset=utf-8")],
        r#"{"error":{"code":"not_found","message":"That project does not exist."}}"#,
    )
        .into_response()
}

/// `GET /api/site` — identity, socials, filters, and counts.
pub async fn site(State(state): State<AppState>) -> Response {
    let site = Site::load(&state.db).await;
    cached(json!({
        "profile": {
            "name": site.profile.name,
            "job_title": site.profile.job_title,
            "description": site.profile.description,
            "location": site.profile.location,
            "email": site.profile.email,
            "availability": site.profile.availability,
        },
        "socials": site.socials.iter().map(|social| json!({
            "label": social.label,
            "handle": social.handle,
            "href": social.href,
            "icon": social.icon,
        })).collect::<Vec<_>>(),
        "filters": site.categories.iter().map(|category| json!({
            "id": category.id,
            "label": category.label,
            "count": site.projects.iter().filter(|project| project.category == category.id).count(),
        })).collect::<Vec<_>>(),
        "counts": {
            "projects": site.projects.len(),
            "tools": site.tools.len(),
            "principles": site.principles.len(),
            "milestones": site.milestones.len(),
        }
    }))
}

/// `GET /api/projects` — card summaries, cheapest useful shape for a grid.
pub async fn projects(State(state): State<AppState>) -> Response {
    let projects = portfolio::visible_projects(&state.db).await;
    cached(json!({
        "projects": projects.iter().enumerate().map(|(index, project)| {
            let mut card = project.card();
            let object = card.as_object_mut().expect("card is an object");
            object.insert("stack_line".into(), json!(project.stack().join(" · ")));
            let ia = if project.tagline.trim().is_empty() { format!("{} project cover — {}", project.name, project.summary.trim()) } else { format!("{} project cover — {} {}", project.name, project.tagline.trim(), project.summary.trim()) };
            object.insert("image_alt".into(), json!(ia));
            object.insert("wide".into(), json!(index == 0));
            card
        }).collect::<Vec<_>>()
    }))
}

/// `GET /api/projects/{slug}` — one case study, with siblings for the footer.
pub async fn project(State(state): State<AppState>, Path(slug): Path<String>) -> Response {
    let Some(project) = portfolio::visible_project(&state.db, &slug).await else {
        return missing();
    };
    let others: Vec<Value> = portfolio::visible_projects(&state.db)
        .await
        .iter()
        .filter(|item| item.slug != project.slug)
        .map(ProjectRow::card)
        .collect();
    let image_alt = if project.tagline.trim().is_empty() {
        format!("{} project cover — {}", project.name, project.summary.trim())
    } else {
        format!("{} project cover — {} {}", project.name, project.tagline.trim(), project.summary.trim())
    };
    let mut detail = project.detail();
    if let Some(obj) = detail.as_object_mut() {
        obj.insert("image_alt".into(), json!(image_alt));
    }
    cached(json!({
        "project": detail,
        "others": others,
    }))
}

/// `GET /api/toolkit` — groups, tools, and the project names they point at.
/// The home page uses this when a chip is selected.
pub async fn toolkit(State(state): State<AppState>) -> Response {
    let site = Site::load(&state.db).await;
    let name_of = |tool: &ToolRow| -> String {
        site.projects
            .iter()
            .find(|project| project.slug == tool.project_slug)
            .map(|project| project.name.clone())
            .unwrap_or_default()
    };
    cached(json!({
        "groups": site.groups.iter().map(|group| json!({
            "id": group.id,
            "number": group.number,
            "title": group.title,
            "subtitle": group.subtitle,
        })).collect::<Vec<_>>(),
        "tools": site.tools.iter().map(|tool| json!({
            "name": tool.name,
            "group": tool.group_id,
            "description": tool.description,
            "project": tool.project_slug,
            "project_name": name_of(tool),
        })).collect::<Vec<_>>(),
    }))
}

/// `GET /api/about` — principles and milestones behind the About section.
pub async fn about(State(state): State<AppState>) -> Response {
    let site = Site::load(&state.db).await;
    cached(json!({
        "intro": site.profile.about_intro,
        "principles": site.principles.iter().map(|principle| json!({
            "title": principle.title,
            "body": principle.body,
        })).collect::<Vec<_>>(),
        "milestones": site.milestones.iter().map(|milestone| json!({
            "marker": milestone.marker,
            "title": milestone.title,
            "detail": milestone.detail,
        })).collect::<Vec<_>>(),
    }))
}

/// `POST /api/telemetry` — lightweight, privacy-preserving device and pageview intake.
pub async fn record_telemetry(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> Response {
    let ip = client_ip(&headers, state.config.trust_proxy_ip);
    if !state
        .throttle(format!("telemetry:{ip}"), 60, Duration::from_secs(60))
        .await
    {
        return StatusCode::TOO_MANY_REQUESTS.into_response();
    }

    let payload: TelemetryPayload = serde_json::from_slice(&body).unwrap_or_default();
    let ua = headers
        .get(header::USER_AGENT)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    let (parsed_device, parsed_os, parsed_browser) = analytics::parse_user_agent(ua);

    let device_type = match payload.device.as_deref() {
        Some("mobile") => "mobile",
        Some("tablet") => "tablet",
        Some("desktop") => "desktop",
        _ => parsed_device,
    };

    let os = match payload.os.as_deref() {
        Some(val) if !val.is_empty() && val.len() <= 32 => val,
        _ => parsed_os,
    };

    let browser = match payload.browser.as_deref() {
        Some(val) if !val.is_empty() && val.len() <= 32 => val,
        _ => parsed_browser,
    };

    let path = payload
        .path
        .as_deref()
        .map(|p| if p.len() > 128 { &p[..128] } else { p })
        .unwrap_or("/");

    let referrer = payload
        .referrer
        .as_deref()
        .map(|r| if r.len() > 128 { &r[..128] } else { r })
        .unwrap_or("");

    let screen = payload
        .screen
        .as_deref()
        .map(|s| if s.len() > 32 { &s[..32] } else { s })
        .unwrap_or("");

    let viewport = payload
        .viewport
        .as_deref()
        .map(|v| if v.len() > 32 { &v[..32] } else { v })
        .unwrap_or("");

    let hash = analytics::visitor_hash(&ip, ua);

    let _ = analytics::record_pageview(
        &state.db,
        path,
        referrer,
        device_type,
        os,
        browser,
        screen,
        viewport,
        &hash,
    )
    .await;

    StatusCode::NO_CONTENT.into_response()
}

/// Answers the browser's preflight for the cross-origin contact post. `OPTIONS`
/// must succeed with no body; any other method that lands here is genuinely not
/// allowed, so it keeps its 405 in the API's own JSON shape.
pub async fn preflight(method: axum::http::Method, request: axum::extract::Request) -> Response {
    if method == axum::http::Method::OPTIONS {
        return StatusCode::NO_CONTENT.into_response();
    }
    let path = request.uri().path().to_string();
    (
        StatusCode::METHOD_NOT_ALLOWED,
        [(header::CONTENT_TYPE, "application/json; charset=utf-8")],
        format!(r#"{{"error":{{"code":"method_not_allowed","message":"That method is not allowed on {path}."}}}}"#),
    )
        .into_response()
}

/// `fallback` for the backend surface: an unknown path is a JSON 404, never an
/// HTML page, so the API always answers in the shape it advertises.
pub async fn not_found() -> Response {
    (
        StatusCode::NOT_FOUND,
        [(header::CONTENT_TYPE, "application/json; charset=utf-8")],
        r#"{"error":{"code":"not_found","message":"That endpoint does not exist."}}"#,
    )
        .into_response()
}
