use std::time::Duration;

use sqlx::{
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions},
    SqlitePool,
};

pub const SCHEMAS: [&str; 5] = [
    include_str!("../migrations/0001_init.sql"),
    include_str!("../migrations/0002_portfolio.sql"),
    include_str!("../migrations/0003_analytics.sql"),
    include_str!("../migrations/0004_project_image.sql"),
    include_str!("../migrations/0005_post_cover_image.sql"),
];
pub async fn connect(database_path: &str) -> Result<SqlitePool, Box<dyn std::error::Error>> {
    if let Some(parent) = std::path::Path::new(database_path).parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }
    let options = SqliteConnectOptions::new()
        .filename(database_path)
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal)
        .foreign_keys(true)
        .busy_timeout(Duration::from_secs(5));
    Ok(SqlitePoolOptions::new().max_connections(8).connect_with(options).await?)
}

/// Applies the schema idempotently. Each statement runs separately because
/// SQLite's driver executes one statement per call. Comment-only lines are
/// dropped first: a semicolon inside a comment would otherwise split a
/// statement in half.
pub async fn migrate(db: &SqlitePool) -> Result<(), Box<dyn std::error::Error>> {
    for schema in SCHEMAS {
        let sql: String = schema
            .lines()
            .filter(|line| !line.trim_start().starts_with("--"))
            .collect::<Vec<_>>()
            .join("\n");
        for statement in sql.split(';') {
            let statement = statement.trim();
            if statement.is_empty() {
                continue;
            }
            if let Err(err) = sqlx::query(statement).execute(db).await {
                let msg = err.to_string();
                if !msg.contains("duplicate column name") {
                    return Err(Box::new(err));
                }
            }
        }
    }
    Ok(())
}
