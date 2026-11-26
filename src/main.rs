pub mod analytics;
mod content;
mod db;
mod portfolio;
mod routes;
mod seed;
mod seed_content;
mod state;
mod template;
mod text;
mod views;

use state::{AppState, Config, RateLimiter};
use std::sync::Arc;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::registry()
        .with(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "portfolio=info,tower_http=warn".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let config = Config::from_env().map_err(|err| format!("configuration: {err}"))?;
    let database = db::connect(&config.database_path).await?;
    db::migrate(&database).await?;
    seed::bootstrap_admin(&database, &config).await?;
    seed::seed_posts(&database).await?;
    seed_content::seed_portfolio(&database).await?;

    let site_bind = config.site_bind;
    let admin_bind = config.admin_bind;
    let api_bind = config.api_bind;
    let state = AppState {
        db: database,
        config: Arc::new(config),
        limiter: Arc::new(tokio::sync::Mutex::new(RateLimiter::new())),
    };

    // Three origins, one binary, one database. They share the process so a
    // write on the dashboard is visible to the site on the very next request,
    // but each listener exposes only its own routes and security headers.
    let site = serve(site_bind, routes::site(state.clone())).await?;
    let admin = serve(admin_bind, routes::admin(state.clone())).await?;
    let api = serve(api_bind, routes::api(state.clone())).await?;
    tracing::info!(
        portfolio = %format!("http://{site}"),
        dashboard = %format!("http://{admin}"),
        backend = %format!("http://{api}"),
        "portfolio listening"
    );

    tokio::signal::ctrl_c().await?;
    tracing::info!("shutting down");
    Ok(())
}

/// Binds one address and serves it in the background until the process stops.
/// Every port is bound before the first request is answered, so a port clash
/// fails loudly at startup instead of leaving one surface silently missing.
async fn serve(
    bind: std::net::SocketAddr,
    router: axum::Router,
) -> Result<std::net::SocketAddr, Box<dyn std::error::Error>> {
    let listener = tokio::net::TcpListener::bind(bind)
        .await
        .map_err(|error| format!("cannot bind {bind}: {error}"))?;
    let local = listener.local_addr()?;
    tokio::spawn(async move {
        if let Err(error) = axum::serve(listener, router).await {
            tracing::error!(%local, %error, "server stopped");
        }
    });
    Ok(local)
}
