use axum::{
    extract::{Request, State},
    http::{header, HeaderMap, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};

use crate::content::{hash_token, session_by_token_hash, touch_session};
use crate::state::{ApiError, AppState};

pub const SESSION_COOKIE: &str = "gs_session";
pub const CSRF_COOKIE: &str = "gs_csrf";
const SESSION_TTL_HOURS: i64 = 12;

#[derive(Clone, Debug)]
pub struct AdminSession {
    pub token_hash: String,
    pub csrf: String,
}

fn cookie<'a>(headers: &'a HeaderMap, name: &str) -> Option<&'a str> {
    for value in headers.get_all(header::COOKIE) {
        let Ok(text) = value.to_str() else { continue };
        for pair in text.split(';') {
            let pair = pair.trim();
            if let Some(rest) = pair.strip_prefix(name) {
                if let Some(value) = rest.strip_prefix('=') {
                    return Some(value);
                }
            }
        }
    }
    None
}

pub async fn session_from_headers(state: &AppState, headers: &HeaderMap) -> Option<AdminSession> {
    let token = cookie(headers, SESSION_COOKIE)?;
    let csrf_cookie = cookie(headers, CSRF_COOKIE)?;
    let token_hash = hash_token(token);
    let row = session_by_token_hash(&state.db, &token_hash).await.ok()??;
    let expiry = chrono::DateTime::parse_from_rfc3339(&row.expires_at).ok()?;
    if expiry.with_timezone(&chrono::Utc) <= chrono::Utc::now() {
        crate::content::delete_session(&state.db, &token_hash).await;
        return None;
    }
    if row.csrf != csrf_cookie {
        return None;
    }
    let expires = (chrono::Utc::now() + chrono::Duration::hours(SESSION_TTL_HOURS))
        .to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    touch_session(&state.db, &token_hash, &expires).await;
    Some(AdminSession {
        token_hash,
        csrf: row.csrf,
    })
}

pub fn csrf_ok(session: &AdminSession, supplied: &str) -> bool {
    use subtle::ConstantTimeEq;
    bool::from(session.csrf.as_bytes().ct_eq(supplied.as_bytes()))
}

pub fn client_ip(headers: &HeaderMap, trust_proxy: bool) -> String {
    if trust_proxy {
        if let Some(forwarded) = headers
            .get("x-forwarded-for")
            .and_then(|value| value.to_str().ok())
        {
            if let Some(first) = forwarded.split(',').next() {
                let candidate = first.trim();
                if !candidate.is_empty() {
                    return candidate.to_string();
                }
            }
        }
    }
    headers
        .get("x-real-ip")
        .and_then(|value| value.to_str().ok())
        .unwrap_or("local")
        .to_string()
}

/// Attaches the resolved session to every admin request so handlers stay small.
pub async fn admin_gate(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Response {
    match session_from_headers(&state, request.headers()).await {
        Some(session) => {
            request.extensions_mut().insert(session);
            next.run(request).await
        }
        None => {
            let accepts_html = request
                .headers()
                .get(header::ACCEPT)
                .and_then(|value| value.to_str().ok())
                .map(|value| value.contains("text/html"))
                .unwrap_or(true);
            if accepts_html {
                (StatusCode::SEE_OTHER, [(header::LOCATION, "/login")]).into_response()
            } else {
                ApiError::unauthorized().into_response()
            }
        }
    }
}

/// Random hex token of `bytes` entropy (2 hex characters per byte).
pub fn random_token(bytes: usize) -> String {
    let mut raw = [0u8; 32];
    rand::RngCore::fill_bytes(&mut rand::thread_rng(), &mut raw[..bytes.min(32)]);
    hex::encode(&raw[..bytes.min(32)])
}
