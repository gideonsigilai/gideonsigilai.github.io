use std::time::Duration;

use axum::{
    extract::{Path, State},
    http::{header, HeaderMap, StatusCode},
    response::{Html, IntoResponse, Redirect, Response},
    Json,
};
use serde::Deserialize;
use serde_json::json;

use crate::content::{
    add_message, conversation_by_token, create_conversation, hash_token, message_thread,
    ConversationRow, NewConversation,
};
use crate::portfolio::Site;
use crate::routes::auth::{client_ip, random_token};
use crate::state::{ApiError, ApiResult, AppState};
use crate::template::render;
use crate::text::{clean_text, looks_like_email};
use crate::views::{render_page, short_date, Page, MESSAGE};

#[derive(Deserialize)]
pub struct ContactPayload {
    pub name: String,
    pub email: String,
    pub topic: String,
    pub body: String,
    #[serde(default)]
    pub website: String,
}

async fn persist_contact(
    state: &AppState,
    headers: &HeaderMap,
    input: ContactPayload,
) -> ApiResult<Option<String>> {
    let ip = client_ip(headers, state.config.trust_proxy_ip);
    if !state
        .throttle(format!("contact:{ip}"), 5, Duration::from_secs(600))
        .await
    {
        return Err(ApiError::new(
            StatusCode::TOO_MANY_REQUESTS,
            "rate_limited",
            "You have sent several messages already. Try again in a few minutes.",
        ));
    }
    if !input.website.trim().is_empty() {
        // Honeypot: report success to bots without persisting anything.
        return Ok(None);
    }
    let name = clean_text(&input.name, 2, 80, "Name")?;
    let email = clean_text(&input.email, 5, 254, "Email")?;
    if !looks_like_email(&email) {
        return Err(ApiError::bad("That email address does not look right."));
    }
    let topic = clean_text(&input.topic, 2, 60, "Topic")?;
    let body = clean_text(&input.body, 20, 4000, "Message")?;

    let token = random_token(24);
    create_conversation(
        &state.db,
        NewConversation {
            name: &name,
            email: &email,
            topic: &topic,
            token_hash: &hash_token(&token),
            body: &body,
        },
    )
    .await
    .map_err(ApiError::internal)?;
    tracing::info!("conversation created");
    Ok(Some(token))
}

/// `POST /contact` — JSON for the enhanced form, redirect for plain HTML posts.
pub async fn submit_contact(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> Response {
    let wants_json = headers
        .get(header::ACCEPT)
        .and_then(|value| value.to_str().ok())
        .map(|value| value.contains("application/json"))
        .unwrap_or(false);
    let is_json = headers
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .map(|value| value.contains("application/json"))
        .unwrap_or(false);

    let payload = if is_json {
        match serde_json::from_slice::<ContactPayload>(&body) {
            Ok(payload) => payload,
            Err(_) => return ApiError::bad("The request body could not be read.").into_response(),
        }
    } else {
        let form = form_fields(&body);
        ContactPayload {
            name: form.get("name").cloned().unwrap_or_default(),
            email: form.get("email").cloned().unwrap_or_default(),
            topic: form
                .get("topic")
                .cloned()
                .unwrap_or_else(|| "New project".into()),
            body: form.get("body").cloned().unwrap_or_default(),
            website: form.get("website").cloned().unwrap_or_default(),
        }
    };

    match persist_contact(&state, &headers, payload).await {
        Ok(token) => {
            if wants_json {
                let url = token
                    .as_deref()
                    .map(|token| format!("{}/messages/{token}", state.config.threads_url));
                (
                    StatusCode::CREATED,
                    Json(json!({ "token": token, "url": url })),
                )
                    .into_response()
            } else {
                // Private threads stay live on the server even when the public
                // portfolio is a static snapshot hosted by GitHub Pages.
                Redirect::to(&format!(
                    "{}/messages/{}",
                    state.config.threads_url,
                    token.unwrap_or_default()
                ))
                .into_response()
            }
        }
        Err(error) => {
            if wants_json {
                error.into_response()
            } else {
                // Same-origin failure page would be a dead end on the backend,
                // so send the visitor back to the form on the portfolio.
                Redirect::to(&format!("{}/#contact", state.base_url())).into_response()
            }
        }
    }
}

pub fn form_fields(bytes: &[u8]) -> std::collections::HashMap<String, String> {
    let mut map = std::collections::HashMap::new();
    for pair in String::from_utf8_lossy(bytes).split('&') {
        let mut parts = pair.splitn(2, '=');
        let key = parts.next().unwrap_or_default();
        let value = parts.next().unwrap_or_default();
        if key.is_empty() {
            continue;
        }
        map.insert(
            percent_decode(key),
            percent_decode(&value.replace('+', " ")),
        );
    }
    map
}

fn percent_decode(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' && index + 2 < bytes.len() {
            if let Ok(value) = u8::from_str_radix(&input[index + 1..index + 3], 16) {
                out.push(value);
                index += 3;
                continue;
            }
        }
        out.push(bytes[index]);
        index += 1;
    }
    String::from_utf8_lossy(&out).to_string()
}

/// `GET /messages/{token}` — the visitor's reply thread.
pub async fn show_thread(
    State(state): State<AppState>,
    Path(token): Path<String>,
) -> ApiResult<Response> {
    let conversation = load_conversation(&state, &token).await?;
    reply_page(&state, &token, &conversation, None).await
}

/// `POST /messages/{token}` — visitor follow-up.
pub async fn post_thread(
    State(state): State<AppState>,
    Path(token): Path<String>,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> ApiResult<Response> {
    let form = form_fields(&body);
    let conversation = load_conversation(&state, &token).await?;

    // Bound to the thread: the CSRF value is the hash of the link token, which
    // only someone holding this private URL can read.
    if form.get("csrf").map(String::as_str) != Some(conversation.token_hash.as_str()) {
        return Err(ApiError::forbidden());
    }
    let ip = client_ip(&headers, state.config.trust_proxy_ip);
    if !state
        .throttle(format!("reply:{ip}"), 30, Duration::from_secs(600))
        .await
    {
        return Err(ApiError::new(
            StatusCode::TOO_MANY_REQUESTS,
            "rate_limited",
            "Slow down a moment, then try again.",
        ));
    }
    let text = form.get("body").cloned().unwrap_or_default();
    match clean_text(&text, 2, 4000, "Message") {
        Ok(value) => {
            add_message(&state.db, &conversation.id, "visitor", &value, "new")
                .await
                .map_err(ApiError::internal)?;
            Ok(Redirect::to(&format!("/messages/{token}")).into_response())
        }
        Err(error) => reply_page(&state, &token, &conversation, Some(&error.message)).await,
    }
}

async fn load_conversation(state: &AppState, token: &str) -> ApiResult<ConversationRow> {
    conversation_by_token(&state.db, token)
        .await
        .map_err(ApiError::internal)?
        .ok_or_else(ApiError::not_found)
}

async fn reply_page(
    state: &AppState,
    token: &str,
    conversation: &ConversationRow,
    error: Option<&str>,
) -> ApiResult<Response> {
    let site = Site::load(&state.db).await;
    let rows = message_thread(&state.db, &conversation.id)
        .await
        .map_err(ApiError::internal)?;
    let owner_name = site.profile.name.clone();
    let rendered: Vec<serde_json::Value> = rows
        .iter()
        .map(|message| {
            json!({
                "author": if message.author == "owner" { owner_name.clone() } else { conversation.name.clone() },
                "owner": message.author == "owner",
                "body": message.body.clone(),
                "when": short_date(&message.created_at),
                "created_at": message.created_at,
            })
        })
        .collect();

    let body = render(
        MESSAGE,
        &json!({
            "token": token,
            "csrf": conversation.token_hash,
            "conversation": { "topic": conversation.topic },
            "owner": site.profile.name.clone(),
            "messages": rendered,
            "email": site.profile.email.clone(),
            "error": error.unwrap_or_default(),
        }),
    );

    let page = Page::new(
        format!("{} — Your conversation", conversation.topic),
        format!("Your conversation with {}.", site.profile.name),
        format!("/messages/{token}"),
    )
    .noindex();
    Ok(Html(render_page(&site, page, &state, body)).into_response())
}
