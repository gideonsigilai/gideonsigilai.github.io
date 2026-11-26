use std::time::Duration;

use axum::{
    extract::{Path, Query, State},
    http::{header, HeaderMap, StatusCode},
    response::{Html, IntoResponse, Redirect, Response},
    Extension, Json,
};
use serde::Deserialize;
use serde_json::json;

use crate::content::{
    add_message, all_posts, conversation_by_id, conversations, conversation_stats, count_posts,
    delete_post, delete_session, insert_post, insert_session, message_thread, now_iso,
    post_by_id, set_conversation_status, stored_admin_hash, update_post, AdminSessionRow, Draft,
};
use crate::portfolio::{self, Collection, LookupOptions};
use crate::routes::auth::{
    client_ip, csrf_ok, random_token, session_from_headers, AdminSession,
};
use crate::routes::contact::form_fields;
use crate::state::{ApiError, ApiResult, AppState};
use crate::template::render;
use crate::text::{
    clean_text, derive_excerpt, normalise_headings, render_markdown, reading_minutes, slugify,
};
use crate::views::{
    render_admin_page, short_date, Page, ADMIN_ANALYTICS, ADMIN_CONTENT, ADMIN_INBOX, ADMIN_POSTS, LOGIN,
};

// ---------------------------------------------------------------- sign in / out

#[derive(Deserialize)]
struct LoginPayload {
    username: String,
    password: String,
}

pub async fn show_login(State(state): State<AppState>) -> Response {
    let csrf = random_token(16);
    let body = render(LOGIN, &json!({ "csrf": csrf, "error": "" }));
    let page = Page::new("Sign in", "Private area.", "/login").noindex();
    let site = portfolio::Site::load(&state.db).await;
    let mut response = Html(render_admin_page(&site, page, &state, body)).into_response();
    set_csrf_cookie(&mut response, &csrf);
    response
}

pub async fn login(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> ApiResult<Response> {
    let ip = client_ip(&headers, state.config.trust_proxy_ip);
    if !state.throttle(format!("login:{ip}"), 8, Duration::from_secs(900)).await {
        return Err(ApiError::new(
            StatusCode::TOO_MANY_REQUESTS,
            "rate_limited",
            "Too many attempts. Wait 15 minutes and try again.",
        ));
    }

    let (payload, is_json) = if headers
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .map(|value| value.contains("application/json"))
        .unwrap_or(false)
    {
        let parsed: LoginPayload = serde_json::from_slice(&body)
            .map_err(|_| ApiError::bad("The request body could not be read."))?;
        (parsed, true)
    } else {
        let form = form_fields(&body);
        (
            LoginPayload {
                username: form.get("username").cloned().unwrap_or_default(),
                password: form.get("password").cloned().unwrap_or_default(),
            },
            false,
        )
    };

    let username_ok = {
        use subtle::ConstantTimeEq;
        bool::from(state.config.admin_username.as_bytes().ct_eq(payload.username.trim().as_bytes()))
    };
    let hash = admin_hash(&state).await?;
    let password = payload.password.clone();
    let verified = tokio::task::spawn_blocking(move || {
        use argon2::{Argon2, PasswordHash, PasswordVerifier};
        match PasswordHash::new(&hash) {
            Ok(parsed) => Argon2::default().verify_password(password.as_bytes(), &parsed).is_ok(),
            Err(_) => false,
        }
    })
    .await
    .map_err(ApiError::internal)?;

    if !(username_ok && verified) {
        if is_json {
            return Err(ApiError::new(
                StatusCode::UNAUTHORIZED,
                "invalid_credentials",
                "Those credentials were not accepted.",
            ));
        }
        let csrf = random_token(16);
        let body = render(
            LOGIN,
            &json!({ "csrf": csrf, "error": "Those credentials were not accepted." }),
        );
        let page = Page::new("Sign in", "Private area.", "/login").noindex();
        let site = portfolio::Site::load(&state.db).await;
        let mut response = Html(render_admin_page(&site, page, &state, body)).into_response();
        set_csrf_cookie(&mut response, &csrf);
        return Ok(response);
    }

    crate::content::purge_expired_sessions(&state.db).await;
    let session_token = random_token(32);
    let csrf = random_token(24);
    let expires = (chrono::Utc::now() + chrono::Duration::hours(12))
        .to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    insert_session(&state.db, &crate::content::hash_token(&session_token), &csrf, &expires)
        .await
        .map_err(ApiError::internal)?;

    if is_json {
        return Ok(Json(json!({ "session": session_token, "csrf": csrf, "expires_at": expires }))
            .into_response());
    }

    let mut response = Redirect::to("/").into_response();
    set_session_cookies(&mut response, &session_token, &csrf);
    Ok(response)
}

pub async fn logout(State(state): State<AppState>, headers: HeaderMap) -> Response {
    if let Some(session) = session_from_headers(&state, &headers).await {
        delete_session(&state.db, &session.token_hash).await;
    }
    let mut response = Redirect::to("/login").into_response();
    clear_session_cookies(&mut response);
    response
}

async fn admin_hash(state: &AppState) -> ApiResult<String> {
    if let Some(hash) = &state.config.admin_password_hash {
        return Ok(hash.clone());
    }
    stored_admin_hash(&state.db).await.ok_or_else(|| {
        ApiError::new(
            StatusCode::SERVICE_UNAVAILABLE,
            "admin_unavailable",
            "Admin access is not configured yet.",
        )
    })
}

// ---------------------------------------------------------------- dashboard

#[derive(Deserialize, Default)]
pub struct InboxQuery {
    pub conversation: Option<String>,
}

pub async fn inbox(
    State(state): State<AppState>,
    Extension(session): Extension<AdminSession>,
    Query(query): Query<InboxQuery>,
) -> ApiResult<Response> {
    let rows = conversations(&state.db).await.map_err(ApiError::internal)?;
    let (unread, total) = conversation_stats(&state.db).await;
    let (_, published) = count_posts(&state.db).await;

    let selected_id = query
        .conversation
        .or_else(|| rows.first().map(|row| row.id.clone()));

    let list: Vec<serde_json::Value> = rows
        .iter()
        .map(|row| {
            json!({
                "id": row.id,
                "name": row.name,
                "topic": row.topic,
                "when": short_date(&row.last_message_at),
                "message_count": row.message_count,
                "plural": row.message_count != 1,
                "unread": row.status == "new",
                "archived": row.status == "archived",
                "selected": Some(&row.id) == selected_id.as_ref(),
            })
        })
        .collect();

    let mut detail = serde_json::Value::Null;
    if let Some(id) = &selected_id {
        if let Some(conversation) = conversation_by_id(&state.db, id).await.map_err(ApiError::internal)? {
            let thread = message_thread(&state.db, &conversation.id)
                .await
                .map_err(ApiError::internal)?;
            detail = json!({
                "id": conversation.id,
                "name": conversation.name,
                "email": conversation.email,
                "topic": conversation.topic,
                "status": conversation.status,
                "messages": thread.iter().map(|message| json!({
                    "author": if message.author == "owner" { "You".to_string() } else { conversation.name.clone() },
                    "owner": message.author == "owner",
                    "body": message.body,
                    "when": short_date(&message.created_at),
                    "created_at": message.created_at,
                })).collect::<Vec<_>>(),
            });
        }
    }

    let body = render(
        ADMIN_INBOX,
        &json!({
            "stats": { "unread": unread, "conversations": total, "published": published },
            "conversations": list,
            "has_conversations": !list.is_empty(),
            "detail": detail,
            "csrf": session.csrf,
            "csrf_root": session.csrf,
        }),
    );
    let site = portfolio::Site::load(&state.db).await;
    let page = Page::new("Dashboard", "Admin dashboard.", "/").noindex();
    Ok(Html(render_admin_page(&site, page, &state, body)).into_response())
}

#[derive(Deserialize, Default)]
pub struct PostsQuery {
    pub id: Option<String>,
    /// Slug of the post just saved, so the editor confirms the write.
    pub saved: Option<String>,
}

pub async fn posts(
    State(state): State<AppState>,
    Extension(session): Extension<AdminSession>,
    Query(query): Query<PostsQuery>,
) -> ApiResult<Response> {
    let rows = all_posts(&state.db).await.map_err(ApiError::internal)?;
    let (unread, total) = conversation_stats(&state.db).await;
    let (_, published) = count_posts(&state.db).await;

    let editing = match &query.id {
        Some(id) => post_by_id(&state.db, id).await.map_err(ApiError::internal)?,
        None => None,
    };

    let list: Vec<serde_json::Value> = rows
        .iter()
        .map(|row| {
            json!({
                "id": row.id,
                "title": row.title,
                "draft": row.status == "draft",
                "selected": editing.as_ref().map(|post| post.id.clone()) == Some(row.id.clone()),
                "subtitle": if row.status == "published" {
                    row.published_at.as_deref().map(short_date).unwrap_or_default()
                } else {
                    format!("edited {}", short_date(&row.updated_at))
                },
            })
        })
        .collect();

    let editor = build_editor(editing.as_ref());

    let body = render(
        ADMIN_POSTS,
        &json!({
            "stats": { "unread": unread, "conversations": total, "published": published },
            "posts": list,
            "csrf": session.csrf,
            "form_action": editor["action"],
            "id": editor["id"],
            "title": editor["title"],
            "slug": editor["slug"],
            "tags": editor["tags"],
            "cover_note": editor["cover_note"],
            "excerpt": editor["excerpt"],
            "markdown": editor["markdown"],
            "notice": query
                .saved
                .as_deref()
                .map(|slug| format!("Saved. It is live at /writing/{slug}."))
                .unwrap_or_default(),
            "error": "",
        }),
    );
    let site = portfolio::Site::load(&state.db).await;
    let page = Page::new("Writing — Dashboard", "Admin dashboard.", "/posts").noindex();
    Ok(Html(render_admin_page(&site, page, &state, body)).into_response())
}

fn build_editor(existing: Option<&crate::content::PostRow>) -> serde_json::Value {
    match existing {
        Some(post) => json!({
            "action": format!("/posts/{}", post.id),
            "id": post.id,
            "title": post.title,
            "slug": post.slug,
            "tags": post.tags().join(", "),
            "cover_note": post.cover_note,
            "excerpt": post.excerpt,
            "markdown": post.body_markdown,
        }),
        None => json!({
            "action": "/posts",
            "id": "",
            "title": "",
            "slug": "",
            "tags": "",
            "cover_note": "",
            "excerpt": "",
            "markdown": "",
        }),
    }
}

// ---------------------------------------------------------------- portfolio content

#[derive(Deserialize, Default)]
pub struct ContentQuery {
    pub section: Option<String>,
    pub id: Option<String>,
    /// Validation message carried back from a failed save.
    pub error: Option<String>,
    /// Confirmation carried back from a successful save.
    pub saved: Option<String>,
}

pub async fn content(
    State(state): State<AppState>,
    Extension(session): Extension<AdminSession>,
    Query(query): Query<ContentQuery>,
) -> ApiResult<Response> {
    let (unread, total) = conversation_stats(&state.db).await;
    let (_, published) = count_posts(&state.db).await;
    let options = LookupOptions::load(&state.db).await;

    let is_profile = query.section.as_deref() == Some("profile");
    let active = if is_profile {
        None
    } else {
        let requested = query.section.as_deref().unwrap_or("projects");
        Some(
            portfolio::collection(requested)
                .or_else(|| portfolio::collection("projects"))
                .expect("projects is declared"),
        )
    };

    let mut sections = Vec::new();
    for collection in portfolio::COLLECTIONS {
        sections.push(json!({
            "slug": collection.slug,
            "title": collection.title,
            "count": portfolio::admin_rows(&state.db, collection).await.len(),
            "active": active.map(|item| item.slug) == Some(collection.slug),
        }));
    }

    let (items, fields, form_action, is_new, view_href, selected_id) = match active {
        Some(collection) => {
            let rows = portfolio::admin_rows(&state.db, collection).await;
            let editing_id = query.id.as_deref().filter(|id| *id != "new");

            match editing_id {
                Some(id) => {
                    let values = portfolio::row_values(&state.db, collection.table, id)
                        .await
                        .unwrap_or_default();
                    let fields = portfolio::fields_html(collection, &values, &options);
                    let slug = values
                        .iter()
                        .find(|(column, _)| column == "slug")
                        .map(|(_, value)| value.clone())
                        .unwrap_or_default();
                    let view = if collection.slug == "projects" && !slug.is_empty() {
                        format!("/work/{slug}")
                    } else {
                        String::new()
                    };
                    (
                        rows.iter()
                            .map(|row| {
                                json!({
                                    "id": row.id,
                                    "title": row.title,
                                    "detail": row.detail,
                                    "selected": row.id == id,
                                })
                            })
                            .collect::<Vec<_>>(),
                        fields,
                        format!("/content/{}/{}", collection.slug, id),
                        false,
                        view,
                        Some(id.to_string()),
                    )
                }
                None => {
                    // New row: prefill single-choice lookups with their first
                    // option so a fresh form saves without extra clicks.
                    let defaults = default_values(collection, &options);
                    let fields = portfolio::fields_html(collection, &defaults, &options);
                    (
                        rows.iter()
                            .map(|row| {
                                json!({
                                    "id": row.id,
                                    "title": row.title,
                                    "detail": row.detail,
                                    "selected": false,
                                })
                            })
                            .collect::<Vec<_>>(),
                        fields,
                        format!("/content/{}", collection.slug),
                        true,
                        String::new(),
                        None,
                    )
                }
            }
        }
        None => (Vec::new(), String::new(), String::new(), true, String::new(), None),
    };

    let profile_fields = if is_profile {
        let current = match portfolio::profile(&state.db).await {
            Some(profile) => portfolio::profile_values(&profile),
            None => Vec::new(),
        };
        profile_fields_html(&current)
    } else {
        String::new()
    };

    let selected = selected_id.unwrap_or_default();
    let body = render(
        ADMIN_CONTENT,
        &json!({
            "stats": { "unread": unread, "conversations": total, "published": published },
            "csrf": session.csrf,
            "csrf_root": session.csrf,
            "sections": sections,
            "is_profile": is_profile,
            "profile_active": is_profile,
            "profile_fields": profile_fields,
            "items": items,
            "section": active.map(|item| item.slug).unwrap_or("projects"),
            "single": active.map(|item| item.single).unwrap_or("item"),
            "title": active.map(|item| item.title).unwrap_or("Portfolio"),
            "blurb": active.map(|item| item.blurb).unwrap_or_default(),
            "fields": fields,
            "form_action": form_action,
            "is_new": is_new,
            "view_href": view_href,
            "id": selected,
            "error": query.error.clone().unwrap_or_default(),
            "notice": query.saved.clone().unwrap_or_default(),
        }),
    );
    let page = Page::new("Portfolio — Dashboard", "Admin dashboard.", "/content").noindex();
    let site = portfolio::Site::load(&state.db).await;
    Ok(Html(render_admin_page(&site, page, &state, body)).into_response())
}

/// Prefill values for a brand-new row.
fn default_values(
    collection: &Collection,
    options: &LookupOptions,
) -> Vec<(String, String)> {
    collection
        .fields
        .iter()
        .map(|field| {
            let value = match field.kind {
                portfolio::Kind::Lookup(portfolio::Lookup::Categories) => {
                    options.categories.first().map(|(id, _)| id.clone()).unwrap_or_default()
                }
                portfolio::Kind::Lookup(portfolio::Lookup::Groups) => {
                    options.groups.first().map(|(id, _)| id.clone()).unwrap_or_default()
                }
                portfolio::Kind::Choice(allowed) => allowed.first().copied().unwrap_or_default().to_string(),
                portfolio::Kind::Toggle => "1".to_string(),
                _ => String::new(),
            };
            (field.column.to_string(), value)
        })
        .collect()
}

fn profile_fields_html(current: &[(String, String)]) -> String {
    let fields = portfolio::PROFILE_FIELDS;
    let mut out = String::new();
    let mut index = 0;
    while index < fields.len() {
        let field = &fields[index];
        let paired = fields.get(index + 1).filter(|next| {
            matches!(field.kind, portfolio::Kind::Line { .. })
                && matches!(next.kind, portfolio::Kind::Line { .. })
        });
        match paired {
            Some(next) => {
                out.push_str(r#"<div class="editor-row">"#);
                out.push_str(&portfolio::field_html_for(field, current));
                out.push_str(&portfolio::field_html_for(next, current));
                out.push_str("</div>");
                index += 2;
            }
            None => {
                out.push_str(&portfolio::field_html_for(field, current));
                index += 1;
            }
        }
    }
    out
}

pub async fn analytics_view(
    State(state): State<AppState>,
    Extension(_session): Extension<AdminSession>,
) -> ApiResult<Response> {
    let summary = crate::analytics::get_analytics_summary(&state.db)
        .await
        .map_err(ApiError::internal)?;

    let site = portfolio::Site::load(&state.db).await;
    let body = render(ADMIN_ANALYTICS, &json!({
        "total_views": summary.total_views,
        "unique_visitors": summary.unique_visitors,
        "today_views": summary.today_views,
        "today_visitors": summary.today_visitors,
        "mobile_pct": summary.mobile_pct,
        "top_device": summary.top_device,
        "top_os": summary.top_os,
        "top_browser": summary.top_browser,
        "devices": summary.devices,
        "os_list": summary.os_list,
        "browsers": summary.browsers,
        "viewports": summary.viewports,
        "trend": summary.trend,
        "recent": summary.recent,
    }));

    let page = Page::new("Visitor Analytics", "Device and audience telemetry.", "/analytics").noindex();
    Ok(Html(render_admin_page(&site, page, &state, body)).into_response())
}

pub async fn content_save(
    State(state): State<AppState>,
    Extension(session): Extension<AdminSession>,
    Path(section): Path<String>,
    body: axum::body::Bytes,
) -> ApiResult<Response> {
    let form = form_fields(&body);
    require_csrf_headers(&session, &form)?;
    let Some(collection) = portfolio::collection(&section) else {
        return Err(ApiError::not_found().redirect("/content"));
    };
    let options = LookupOptions::load(&state.db).await;
    let values = portfolio::parse_form(collection, &form, &options)
        .map_err(|message| ApiError::bad(message.clone()).redirect(message_url(collection.slug, None, "error", &message)))?;
    match portfolio::insert_row_with_id(&state.db, collection, None, &values).await {
        Ok(id) => Ok(Redirect::to(&message_url(collection.slug, Some(&id), "saved", "Saved.")).into_response()),
        Err(error) if crate::content::unique_violation(&error) => {
            let message = "That identifier is already used by another entry.";
            Err(ApiError::new(StatusCode::CONFLICT, "slug_taken", message)
                .redirect(message_url(collection.slug, None, "error", message)))
        }
        Err(error) => Err(ApiError::internal(error).redirect(section_url(collection.slug, None))),
    }
}

pub async fn content_update(
    State(state): State<AppState>,
    Extension(session): Extension<AdminSession>,
    Path((section, id)): Path<(String, String)>,
    body: axum::body::Bytes,
) -> ApiResult<Response> {
    let form = form_fields(&body);
    require_csrf_headers(&session, &form)?;
    let Some(collection) = portfolio::collection(&section) else {
        return Err(ApiError::not_found().redirect("/content"));
    };
    let options = LookupOptions::load(&state.db).await;
    let values = portfolio::parse_form(collection, &form, &options)
        .map_err(|message| ApiError::bad(message.clone()).redirect(message_url(collection.slug, Some(&id), "error", &message)))?;
    match portfolio::update_row(&state.db, collection, &id, &values).await {
        Ok(true) => Ok(Redirect::to(&message_url(collection.slug, Some(&id), "saved", "Saved.")).into_response()),
        Ok(false) => Err(ApiError::not_found().redirect(section_url(collection.slug, None))),
        Err(error) if crate::content::unique_violation(&error) => {
            let message = "That identifier is already used by another entry.";
            Err(ApiError::new(StatusCode::CONFLICT, "slug_taken", message)
                .redirect(message_url(collection.slug, Some(&id), "error", message)))
        }
        Err(error) => Err(ApiError::internal(error).redirect(section_url(collection.slug, Some(&id)))),
    }
}

pub async fn content_delete(
    State(state): State<AppState>,
    Extension(session): Extension<AdminSession>,
    Path((section, id)): Path<(String, String)>,
    body: axum::body::Bytes,
) -> ApiResult<Response> {
    let form = form_fields(&body);
    require_csrf_headers(&session, &form)?;
    let Some(collection) = portfolio::collection(&section) else {
        return Err(ApiError::not_found().redirect("/content"));
    };
    portfolio::delete_row(&state.db, collection.table, &id)
        .await
        .map_err(|error| ApiError::internal(error).redirect(section_url(collection.slug, None)))?;
    Ok(Redirect::to(&section_url(collection.slug, None)).into_response())
}

pub async fn content_move(
    State(state): State<AppState>,
    Extension(session): Extension<AdminSession>,
    Path((section, id)): Path<(String, String)>,
    body: axum::body::Bytes,
) -> ApiResult<Response> {
    let form = form_fields(&body);
    require_csrf_headers(&session, &form)?;
    let Some(collection) = portfolio::collection(&section) else {
        return Err(ApiError::not_found().redirect("/content"));
    };
    let delta = if form.get("direction").map(String::as_str) == Some("up") { -1 } else { 1 };
    portfolio::move_row(&state.db, collection, &id, delta)
        .await
        .map_err(|error| ApiError::internal(error).redirect(section_url(collection.slug, None)))?;
    Ok(Redirect::to(&section_url(collection.slug, Some(&id))).into_response())
}

pub async fn profile_save(
    State(state): State<AppState>,
    Extension(session): Extension<AdminSession>,
    body: axum::body::Bytes,
) -> ApiResult<Response> {
    let form = form_fields(&body);
    require_csrf_headers(&session, &form)?;
    let profile = portfolio::parse_profile(&form).map_err(|message| {
        ApiError::bad(message.clone())
            .redirect(message_url("profile", None, "error", &message))
    })?;
    portfolio::save_profile(&state.db, &profile)
        .await
        .map_err(|error| ApiError::internal(error).redirect("/content?section=profile"))?;
    Ok(Redirect::to("/content?section=profile&saved=Profile+updated.").into_response())
}

fn section_url(section: &str, id: Option<&str>) -> String {
    match id {
        Some(id) => format!("/content?section={section}&id={id}"),
        None => format!("/content?section={section}"),
    }
}

/// Section URL carrying a human-readable message for the admin page.
fn message_url(section: &str, id: Option<&str>, key: &str, message: &str) -> String {
    let encoded: String = message
        .bytes()
        .flat_map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                vec![byte as char]
            }
            b' ' => vec!['+'],
            other => format!("%{other:02X}").chars().collect(),
        })
        .collect();
    let base = section_url(section, id);
    format!("{base}&{key}={encoded}")
}

// ---------------------------------------------------------------- mutations

fn require_csrf(session: &AdminSession, form: &std::collections::HashMap<String, String>) -> ApiResult<()> {
    let supplied = form.get("csrf").map(String::as_str).unwrap_or_default();
    if csrf_ok(session, supplied) {
        Ok(())
    } else {
        Err(ApiError::forbidden().redirect("/"))
    }
}

/// Content forms post from a page whose CSRF value lives in a hidden field, so
/// a failure returns to the section rather than the inbox.
fn require_csrf_headers(
    session: &AdminSession,
    form: &std::collections::HashMap<String, String>,
) -> ApiResult<()> {
    let supplied = form.get("csrf").map(String::as_str).unwrap_or_default();
    if csrf_ok(session, supplied) {
        Ok(())
    } else {
        Err(ApiError::forbidden().redirect("/content"))
    }
}

pub async fn reply(
    State(state): State<AppState>,
    Extension(session): Extension<AdminSession>,
    Path(id): Path<String>,
    body: axum::body::Bytes,
) -> ApiResult<Response> {
    let form = form_fields(&body);
    require_csrf(&session, &form)?;
    let text = clean_text(form.get("body").map(String::as_str).unwrap_or_default(), 1, 4000, "Reply")?;
    let close = form.get("close").is_some();
    conversation_by_id(&state.db, &id)
        .await
        .map_err(ApiError::internal)?
        .ok_or_else(ApiError::not_found)?;
    let status = if close { "archived" } else { "read" };
    add_message(&state.db, &id, "owner", &text, status)
        .await
        .map_err(ApiError::internal)?;
    Ok(Redirect::to(&format!("/?conversation={id}")).into_response())
}

pub async fn set_status(
    State(state): State<AppState>,
    Extension(session): Extension<AdminSession>,
    Path(id): Path<String>,
    body: axum::body::Bytes,
) -> ApiResult<Response> {
    let form = form_fields(&body);
    require_csrf(&session, &form)?;
    let status = form.get("status").map(String::as_str).unwrap_or_default();
    if !matches!(status, "new" | "read" | "archived") {
        return Err(ApiError::bad("Unknown status.").redirect("/"));
    }
    if !set_conversation_status(&state.db, &id, status)
        .await
        .map_err(ApiError::internal)?
    {
        return Err(ApiError::not_found().redirect("/"));
    }
    Ok(Redirect::to(&format!("/?conversation={id}")).into_response())
}

async fn draft_from_form(form: &std::collections::HashMap<String, String>) -> ApiResult<(Draft, String)> {
    let title = clean_text(form.get("title").map(String::as_str).unwrap_or_default(), 3, 140, "Title")
        .map_err(|error| error.redirect("/posts"))?;
    let markdown = clean_text(
        form.get("markdown").map(String::as_str).unwrap_or_default(),
        40,
        60_000,
        "Body",
    )
    .map_err(|error| error.redirect("/posts"))?;

    let slug_input = form.get("slug").cloned().unwrap_or_default();
    let slug = if slug_input.trim().is_empty() {
        slugify(&title)
    } else {
        slugify(&slug_input)
    };
    if slug.len() < 3 || slug.len() > 80 {
        return Err(ApiError::bad("Slug must be 3–80 characters.").redirect("/posts"));
    }

    let status = match form.get("status").map(String::as_str).unwrap_or("draft") {
        "published" => "published".to_string(),
        _ => "draft".to_string(),
    };

    let excerpt = match form.get("excerpt").map(String::as_str).unwrap_or_default().trim() {
        "" => derive_excerpt(&markdown),
        value if value.chars().count() < 10 => {
            return Err(ApiError::bad("Excerpt is too short.").redirect("/posts"))
        }
        value if value.chars().count() > 300 => {
            return Err(ApiError::bad("Excerpt is too long.").redirect("/posts"))
        }
        value => value.to_string(),
    };

    let tags: Vec<String> = form
        .get("tags")
        .map(|value| value.split(',').map(|tag| tag.trim().to_string()).collect::<Vec<_>>())
        .unwrap_or_default()
        .into_iter()
        .filter(|tag| !tag.is_empty() && tag.chars().count() <= 24)
        .take(6)
        .collect();

    let cover_note: String = form
        .get("cover_note")
        .cloned()
        .unwrap_or_default()
        .chars()
        .take(140)
        .collect();
    let cover_image: String = form
        .get("cover_image")
        .cloned()
        .unwrap_or_default()
        .trim()
        .to_string();


    Ok((
        Draft {
            title: title.clone(),
            slug,
            excerpt,
            html: normalise_headings(&render_markdown(&markdown), &title),
            tags_json: serde_json::to_string(&tags).unwrap_or_else(|_| "[]".into()),
            cover_note: cover_note.trim().to_string(),
            cover_image: cover_image.clone(),
            reading_min: reading_minutes(&markdown),
            markdown,
            status: status.clone(),
        },
        status,
    ))
}

pub async fn create_post(
    State(state): State<AppState>,
    Extension(session): Extension<AdminSession>,
    body: axum::body::Bytes,
) -> ApiResult<Response> {
    let form = form_fields(&body);
    require_csrf(&session, &form)?;
    let (draft, _) = draft_from_form(&form).await?;
    match insert_post(&state.db, draft).await {
        Ok(id) => Ok(Redirect::to(&format!("/posts?id={id}")).into_response()),
        Err(error) if crate::content::unique_violation(&error) => {
            Err(ApiError::new(StatusCode::CONFLICT, "slug_taken", "That slug is already used by another post.")
                .redirect("/posts"))
        }
        Err(error) => Err(ApiError::internal(error).redirect("/posts")),
    }
}

pub async fn update_existing_post(
    State(state): State<AppState>,
    Extension(session): Extension<AdminSession>,
    Path(id): Path<String>,
    body: axum::body::Bytes,
) -> ApiResult<Response> {
    let form = form_fields(&body);
    require_csrf(&session, &form)?;
    let (draft, _) = draft_from_form(&form).await?;
    match update_post(&state.db, &id, draft).await {
        Ok(Some(slug)) => Ok(Redirect::to(&format!("/posts?id={id}&saved={slug}")).into_response()),
        Ok(None) => Err(ApiError::not_found().redirect("/posts")),
        Err(error) if crate::content::unique_violation(&error) => {
            Err(ApiError::new(StatusCode::CONFLICT, "slug_taken", "That slug is already used by another post.")
                .redirect("/posts"))
        }
        Err(error) => Err(ApiError::internal(error).redirect("/posts")),
    }
}

pub async fn destroy_post(
    State(state): State<AppState>,
    Extension(session): Extension<AdminSession>,
    Path(id): Path<String>,
    body: axum::body::Bytes,
) -> ApiResult<Response> {
    let form = form_fields(&body);
    require_csrf(&session, &form)?;
    if !delete_post(&state.db, &id).await.map_err(ApiError::internal)? {
        return Err(ApiError::not_found().redirect("/posts"));
    }
    Ok(Redirect::to("/posts").into_response())
}

pub async fn status_check(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Response {
    match session_from_headers(&state, &headers).await {
        Some(_) => Json(json!({ "authenticated": true, "at": now_iso() })).into_response(),
        None => (
            StatusCode::UNAUTHORIZED,
            Json(json!({ "authenticated": false })),
        )
            .into_response(),
    }
}

// ---------------------------------------------------------------- cookies

fn cookie_flags() -> &'static str {
    "Path=/; HttpOnly; SameSite=Strict"
}

fn set_session_cookies(response: &mut Response, token: &str, csrf: &str) {
    let token_cookie = format!("gs_session={token}; {flags}; Max-Age=43200", flags = cookie_flags());
    // The CSRF value must be readable so the form can echo it back.
    let csrf_cookie = format!("gs_csrf={csrf}; Path=/; SameSite=Strict; Max-Age=43200");
    if let Ok(value) = header::HeaderValue::from_str(&token_cookie) {
        response.headers_mut().append(header::SET_COOKIE, value);
    }
    if let Ok(value) = header::HeaderValue::from_str(&csrf_cookie) {
        response.headers_mut().append(header::SET_COOKIE, value);
    }
}

fn set_csrf_cookie(response: &mut Response, csrf: &str) {
    let csrf_cookie = format!("gs_csrf={csrf}; Path=/; SameSite=Strict; Max-Age=43200");
    if let Ok(value) = header::HeaderValue::from_str(&csrf_cookie) {
        response.headers_mut().append(header::SET_COOKIE, value);
    }
}

fn clear_session_cookies(response: &mut Response) {
    for name in ["gs_session", "gs_csrf"] {
        if let Ok(value) = header::HeaderValue::from_str(&format!("{name}=; Path=/; Max-Age=0")) {
            response.headers_mut().append(header::SET_COOKIE, value);
        }
    }
}

#[allow(dead_code)]
fn unused_admin_row(_: AdminSessionRow) {}
