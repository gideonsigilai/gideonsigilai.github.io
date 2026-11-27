use std::{
    net::SocketAddr,
    sync::Arc,
    time::{Duration, Instant},
};

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use sqlx::SqlitePool;

// ---------------------------------------------------------------- configuration

// Ports are fixed by default so the three surfaces are always found where the
// README and build script say they are: 8050 the portfolio, 8051 the dashboard,
// 8052 the content API and contact intake.
pub const DEFAULT_SITE_PORT: u16 = 8050;
pub const DEFAULT_ADMIN_PORT: u16 = 8051;
pub const DEFAULT_API_PORT: u16 = 8052;

#[derive(Clone)]
pub struct Config {
    /// Portfolio pages: the public site.
    pub site_bind: SocketAddr,
    /// Admin dashboard: sign-in, writing, inbox, portfolio content editing.
    pub admin_bind: SocketAddr,
    /// Backend: read-only JSON, contact intake, visitor threads.
    pub api_bind: SocketAddr,
    /// Absolute origin of the backend as the browser reaches it. Contact posts
    /// go cross-origin to it, so it is part of the portfolio's CSP.
    pub api_url: String,
    /// Origin serving live visitor conversations; may differ from static Pages.
    pub threads_url: String,
    /// Absolute origin of the dashboard. Nothing public links to it.
    pub admin_url: String,
    pub database_path: String,
    pub admin_username: String,
    pub admin_password: Option<String>,
    pub admin_password_hash: Option<String>,
    pub trust_proxy_ip: bool,
    /// Absolute base URL used in feeds, sitemaps, and canonical links.
    pub base_url: String,
}

impl Config {
    pub fn from_env() -> Result<Self, String> {
        let _ = dotenv_load();
        let site_bind = bind("SITE_BIND", "BIND", DEFAULT_SITE_PORT)?;
        let admin_bind = bind("ADMIN_BIND", "ADMIN_BIND", DEFAULT_ADMIN_PORT)?;
        let api_bind = bind("API_BIND", "API_BIND", DEFAULT_API_PORT)?;
        let api_url = std::env::var("API_URL")
            .unwrap_or_else(|_| format!("http://{api_bind}"))
            .trim_end_matches('/')
            .to_string();
        let admin_url = std::env::var("ADMIN_URL")
            .unwrap_or_else(|_| format!("http://{admin_bind}"))
            .trim_end_matches('/')
            .to_string();
        let base_url = std::env::var("BASE_URL")
            .unwrap_or_else(|_| format!("http://{site_bind}"))
            .trim_end_matches('/')
            .to_string();
        let threads_url = std::env::var("THREADS_URL")
            .unwrap_or_else(|_| base_url.clone())
            .trim_end_matches('/')
            .to_string();
        Ok(Self {
            site_bind,
            admin_bind,
            api_bind,
            api_url,
            threads_url,
            admin_url,
            database_path: std::env::var("DATABASE_PATH").unwrap_or_else(|_| {
                // Keep state beside the executable so a copied .exe behaves the
                // same no matter which directory it is started from.
                let base = std::env::current_exe()
                    .ok()
                    .and_then(|path| path.parent().map(std::path::Path::to_path_buf))
                    .unwrap_or_else(|| std::path::PathBuf::from("."));
                base.join("data")
                    .join("portfolio.db")
                    .to_string_lossy()
                    .into_owned()
            }),
            admin_username: std::env::var("ADMIN_USERNAME").unwrap_or_else(|_| "gideon".into()),
            admin_password: std::env::var("ADMIN_PASSWORD").ok(),
            admin_password_hash: std::env::var("ADMIN_PASSWORD_HASH").ok(),
            trust_proxy_ip: std::env::var("TRUST_PROXY_IP").is_ok(),
            base_url,
        })
    }
}

/// Reads `<specific>` first, then `legacy` (the portfolio keeps honouring the
/// old single-server `BIND`), then the fixed loopback default.
fn bind(specific: &str, legacy: &str, port: u16) -> Result<SocketAddr, String> {
    let raw = std::env::var(specific)
        .or_else(|_| std::env::var(legacy))
        .unwrap_or_else(|_| format!("127.0.0.1:{port}"));
    raw.parse()
        .map_err(|_| format!("invalid {specific}: {raw}"))
}

fn dotenv_load() {
    if let Ok(contents) = std::fs::read_to_string(".env") {
        for line in contents.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if let Some((key, value)) = line.split_once('=') {
                let key = key.trim();
                if std::env::var_os(key).is_some() {
                    continue;
                }
                let value = value.trim().trim_matches('"').trim_matches('\'');
                std::env::set_var(key, value);
            }
        }
    }
}

// ---------------------------------------------------------------- rate limiting

pub struct RateLimiter {
    hits: std::collections::HashMap<String, Vec<Instant>>,
}

impl RateLimiter {
    pub fn new() -> Self {
        Self {
            hits: std::collections::HashMap::new(),
        }
    }

    /// Sliding-window limiter. Returns false when the caller must back off.
    pub fn allow(&mut self, key: &str, max: usize, window: Duration) -> bool {
        let now = Instant::now();
        self.hits.retain(|_, stamps| {
            stamps.retain(|at| now.duration_since(*at) < window);
            !stamps.is_empty()
        });
        if self.hits.len() > 20_000 {
            self.hits.clear();
        }
        let entry = self.hits.entry(key.to_string()).or_default();
        if entry.len() >= max {
            return false;
        }
        entry.push(now);
        true
    }
}

// ---------------------------------------------------------------- app state

#[derive(Clone)]
pub struct AppState {
    pub db: SqlitePool,
    pub config: Arc<Config>,
    pub limiter: Arc<tokio::sync::Mutex<RateLimiter>>,
}

impl AppState {
    pub async fn throttle(&self, key: String, max: usize, window: Duration) -> bool {
        self.limiter.lock().await.allow(&key, max, window)
    }

    /// Absolute origin of the content API, injected into each page.
    pub fn api_url(&self) -> &str {
        &self.config.api_url
    }

    pub fn base_url(&self) -> &str {
        &self.config.base_url
    }
}

// ---------------------------------------------------------------- errors

pub struct ApiError {
    pub status: StatusCode,
    pub code: &'static str,
    pub message: String,
    /// Redirect target for form failures handled by the HTML layer.
    pub location: Option<String>,
}

impl ApiError {
    pub fn new(status: StatusCode, code: &'static str, message: impl Into<String>) -> Self {
        Self {
            status,
            code,
            message: message.into(),
            location: None,
        }
    }

    pub fn bad(message: impl Into<String>) -> Self {
        Self::new(StatusCode::BAD_REQUEST, "invalid_request", message)
    }

    pub fn unauthorized() -> Self {
        Self::new(
            StatusCode::UNAUTHORIZED,
            "unauthorized",
            "Sign in to continue.",
        )
    }

    pub fn forbidden() -> Self {
        Self::new(StatusCode::FORBIDDEN, "csrf", "Refresh the page and retry.")
    }

    pub fn not_found() -> Self {
        Self::new(
            StatusCode::NOT_FOUND,
            "not_found",
            "That resource does not exist.",
        )
    }

    pub fn internal(err: impl std::fmt::Display) -> Self {
        tracing::error!("internal error: {err}");
        Self::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "internal_error",
            "Something went wrong on our side.",
        )
    }

    /// Sends a form submission back to the page it came from with a message.
    pub fn redirect(mut self, target: impl Into<String>) -> Self {
        self.location = Some(target.into());
        self
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        if let Some(location) = self.location {
            return (
                StatusCode::SEE_OTHER,
                [(axum::http::header::LOCATION, location)],
            )
                .into_response();
        }
        (
            self.status,
            Json(serde_json::json!({ "error": { "code": self.code, "message": self.message } })),
        )
            .into_response()
    }
}

pub type ApiResult<T> = Result<T, ApiError>;
