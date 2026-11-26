use chrono::{SecondsFormat, Utc};
use serde::Serialize;
use sha2::{Digest, Sha256};
use sqlx::{FromRow, SqlitePool};

pub fn now_iso() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true)
}

pub fn hash_token(token: &str) -> String {
    hex::encode(Sha256::digest(token.as_bytes()))
}

// ---------------------------------------------------------------- posts

#[derive(Debug, Serialize, FromRow)]
pub struct PostRow {
    pub id: String,
    pub slug: String,
    pub title: String,
    pub excerpt: String,
    #[serde(skip)]
    pub body_markdown: String,
    pub body_html: String,
    #[serde(skip)]
    #[sqlx(rename = "tags")]
    pub tags_json: String,
    pub cover_note: String,
    pub cover_image: String,
    pub status: String,
    pub reading_min: i64,
    pub published_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl PostRow {
    pub fn tags(&self) -> Vec<String> {
        serde_json::from_str(&self.tags_json).unwrap_or_default()
    }

    pub fn is_published(&self) -> bool {
        self.status == "published"
    }
}

pub async fn list_published(db: &SqlitePool, limit: i64) -> sqlx::Result<Vec<PostRow>> {
    sqlx::query_as(
        "SELECT * FROM posts WHERE status = 'published'
         ORDER BY COALESCE(published_at, created_at) DESC LIMIT ?",
    )
    .bind(limit)
    .fetch_all(db)
    .await
}

pub async fn published_by_tag(db: &SqlitePool, tag: &str, limit: i64) -> sqlx::Result<Vec<PostRow>> {
    sqlx::query_as(
        "SELECT * FROM posts WHERE status = 'published'
         AND EXISTS (SELECT 1 FROM json_each(posts.tags) WHERE value = ?)
         ORDER BY COALESCE(published_at, created_at) DESC LIMIT ?",
    )
    .bind(tag)
    .bind(limit)
    .fetch_all(db)
    .await
}

pub async fn tag_counts(db: &SqlitePool) -> Vec<(String, i64)> {
    sqlx::query_as(
        "SELECT value AS tag, COUNT(*) FROM posts, json_each(posts.tags)
         WHERE posts.status = 'published' GROUP BY value ORDER BY value",
    )
    .fetch_all(db)
    .await
    .unwrap_or_default()
}

pub async fn published_by_slug(db: &SqlitePool, slug: &str) -> sqlx::Result<Option<PostRow>> {
    sqlx::query_as("SELECT * FROM posts WHERE slug = ? AND status = 'published'")
        .bind(slug)
        .fetch_optional(db)
        .await
}

pub async fn all_posts(db: &SqlitePool) -> sqlx::Result<Vec<PostRow>> {
    sqlx::query_as("SELECT * FROM posts ORDER BY COALESCE(published_at, created_at) DESC LIMIT 200")
        .fetch_all(db)
        .await
}

pub async fn post_by_id(db: &SqlitePool, id: &str) -> sqlx::Result<Option<PostRow>> {
    sqlx::query_as("SELECT * FROM posts WHERE id = ?")
        .bind(id)
        .fetch_optional(db)
        .await
}

pub async fn related(db: &SqlitePool, exclude_id: &str, limit: i64) -> Vec<PostRow> {
    sqlx::query_as(
        "SELECT * FROM posts WHERE status = 'published' AND id != ?
         ORDER BY COALESCE(published_at, created_at) DESC LIMIT ?",
    )
    .bind(exclude_id)
    .bind(limit)
    .fetch_all(db)
    .await
    .unwrap_or_default()
}

pub struct Draft {
    pub title: String,
    pub slug: String,
    pub excerpt: String,
    pub markdown: String,
    pub html: String,
    pub tags_json: String,
    pub cover_note: String,
    pub cover_image: String,
    pub status: String,
    pub reading_min: i64,
}

pub async fn insert_post(db: &SqlitePool, draft: Draft) -> sqlx::Result<String> {
    let id = uuid::Uuid::new_v4().to_string();
    let timestamp = now_iso();
    let published_at = (draft.status == "published").then(|| timestamp.clone());
    sqlx::query(
        "INSERT INTO posts (id, slug, title, excerpt, body_markdown, body_html, tags, cover_note, cover_image,
                            status, reading_min, published_at, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(&draft.slug)
    .bind(&draft.title)
    .bind(&draft.excerpt)
    .bind(&draft.markdown)
    .bind(&draft.html)
    .bind(&draft.tags_json)
    .bind(&draft.cover_note)
    .bind(&draft.cover_image)
    .bind(&draft.status)
    .bind(draft.reading_min)
    .bind(&published_at)
    .bind(&timestamp)
    .bind(&timestamp)
    .execute(db)
    .await?;
    Ok(id)
}

pub async fn update_post(db: &SqlitePool, id: &str, draft: Draft) -> sqlx::Result<Option<String>> {
    let existing: Option<Option<String>> = sqlx::query_scalar("SELECT published_at FROM posts WHERE id = ?")
        .bind(id)
        .fetch_optional(db)
        .await?;
    let Some(existing_published) = existing else {
        return Ok(None);
    };
    let published_at = match (draft.status.as_str(), existing_published) {
        ("published", Some(value)) => Some(value),
        ("published", None) => Some(now_iso()),
        _ => None,
    };
    sqlx::query(
        "UPDATE posts SET slug = ?, title = ?, excerpt = ?, body_markdown = ?, body_html = ?,
                          tags = ?, cover_note = ?, cover_image = ?, status = ?, reading_min = ?, published_at = ?, updated_at = ?
         WHERE id = ?",
    )
    .bind(&draft.slug)
    .bind(&draft.title)
    .bind(&draft.excerpt)
    .bind(&draft.markdown)
    .bind(&draft.html)
    .bind(&draft.tags_json)
    .bind(&draft.cover_note)
    .bind(&draft.cover_image)
    .bind(&draft.status)
    .bind(draft.reading_min)
    .bind(&published_at)
    .bind(now_iso())
    .bind(id)
    .execute(db)
    .await?;
    Ok(Some(draft.slug.to_string()))
}

pub async fn delete_post(db: &SqlitePool, id: &str) -> sqlx::Result<bool> {
    let deleted = sqlx::query("DELETE FROM posts WHERE id = ?")
        .bind(id)
        .execute(db)
        .await?;
    Ok(deleted.rows_affected() > 0)
}

pub async fn count_posts(db: &SqlitePool) -> (i64, i64) {
    sqlx::query_as("SELECT COUNT(*), COUNT(NULLIF(status, 'draft')) FROM posts")
        .fetch_one(db)
        .await
        .unwrap_or((0, 0))
}

// ---------------------------------------------------------------- conversations

#[derive(FromRow, Serialize)]
pub struct MessageRow {
    pub id: String,
    pub author: String,
    pub body: String,
    pub created_at: String,
}

#[derive(Serialize, FromRow)]
pub struct ConversationSummary {
    pub id: String,
    pub name: String,
    pub email: String,
    pub topic: String,
    pub status: String,
    pub created_at: String,
    pub last_message_at: String,
    pub message_count: i64,
}

#[derive(FromRow)]
pub struct ConversationRow {
    pub id: String,
    pub token_hash: String,
    pub name: String,
    pub email: String,
    pub topic: String,
    pub status: String,
}

pub struct NewConversation<'a> {
    pub name: &'a str,
    pub email: &'a str,
    pub topic: &'a str,
    pub token_hash: &'a str,
    pub body: &'a str,
}

pub async fn create_conversation(db: &SqlitePool, input: NewConversation<'_>) -> sqlx::Result<String> {
    let id = uuid::Uuid::new_v4().to_string();
    let timestamp = now_iso();
    let mut tx = db.begin().await?;
    sqlx::query(
        "INSERT INTO conversations (id, token_hash, name, email, topic, status, created_at, updated_at, last_message_at)
         VALUES (?, ?, ?, ?, ?, 'new', ?, ?, ?)",
    )
    .bind(&id)
    .bind(input.token_hash)
    .bind(input.name)
    .bind(input.email)
    .bind(input.topic)
    .bind(&timestamp)
    .bind(&timestamp)
    .bind(&timestamp)
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        "INSERT INTO messages (id, conversation_id, author, body, created_at) VALUES (?, ?, 'visitor', ?, ?)",
    )
    .bind(uuid::Uuid::new_v4().to_string())
    .bind(&id)
    .bind(input.body)
    .bind(&timestamp)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(id)
}

pub async fn message_thread(db: &SqlitePool, conversation_id: &str) -> sqlx::Result<Vec<MessageRow>> {
    sqlx::query_as(
        "SELECT id, author, body, created_at FROM messages
         WHERE conversation_id = ? ORDER BY created_at ASC",
    )
    .bind(conversation_id)
    .fetch_all(db)
    .await
}

pub async fn conversation_by_token(db: &SqlitePool, token: &str) -> sqlx::Result<Option<ConversationRow>> {
    sqlx::query_as(
        "SELECT id, token_hash, name, email, topic, status FROM conversations WHERE token_hash = ?",
    )
        .bind(hash_token(token))
        .fetch_optional(db)
        .await
}

pub async fn conversation_by_id(db: &SqlitePool, id: &str) -> sqlx::Result<Option<ConversationRow>> {
    sqlx::query_as(
        "SELECT id, token_hash, name, email, topic, status FROM conversations WHERE id = ?",
    )
        .bind(id)
        .fetch_optional(db)
        .await
}

pub async fn conversations(db: &SqlitePool) -> sqlx::Result<Vec<ConversationSummary>> {
    sqlx::query_as(
        "SELECT c.id, c.name, c.email, c.topic, c.status, c.created_at, c.last_message_at,
                (SELECT COUNT(*) FROM messages m WHERE m.conversation_id = c.id) AS message_count
         FROM conversations c ORDER BY c.status = 'archived', c.last_message_at DESC LIMIT 200",
    )
    .fetch_all(db)
    .await
}

pub async fn conversation_stats(db: &SqlitePool) -> (i64, i64) {
    sqlx::query_as("SELECT (SELECT COUNT(*) FROM conversations WHERE status = 'new'), (SELECT COUNT(*) FROM conversations)")
        .fetch_one(db)
        .await
        .unwrap_or((0, 0))
}

pub async fn add_message(
    db: &SqlitePool,
    conversation_id: &str,
    author: &str,
    body: &str,
    status: &str,
) -> sqlx::Result<()> {
    let timestamp = now_iso();
    let mut tx = db.begin().await?;
    sqlx::query(
        "INSERT INTO messages (id, conversation_id, author, body, created_at) VALUES (?, ?, ?, ?, ?)",
    )
    .bind(uuid::Uuid::new_v4().to_string())
    .bind(conversation_id)
    .bind(author)
    .bind(body)
    .bind(&timestamp)
    .execute(&mut *tx)
    .await?;
    sqlx::query("UPDATE conversations SET status = ?, updated_at = ?, last_message_at = ? WHERE id = ?")
        .bind(status)
        .bind(&timestamp)
        .bind(&timestamp)
        .bind(conversation_id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(())
}

pub async fn set_conversation_status(db: &SqlitePool, id: &str, status: &str) -> sqlx::Result<bool> {
    let changed = sqlx::query("UPDATE conversations SET status = ?, updated_at = ? WHERE id = ?")
        .bind(status)
        .bind(now_iso())
        .bind(id)
        .execute(db)
        .await?;
    Ok(changed.rows_affected() > 0)
}

// ---------------------------------------------------------------- admin sessions

pub struct AdminSessionRow {
    pub csrf: String,
    pub expires_at: String,
}

pub async fn session_by_token_hash(db: &SqlitePool, token_hash: &str) -> sqlx::Result<Option<AdminSessionRow>> {
    let row: Option<(String, String)> =
        sqlx::query_as("SELECT csrf_token, expires_at FROM admin_sessions WHERE token_hash = ?")
            .bind(token_hash)
            .fetch_optional(db)
            .await?;
    Ok(row.map(|(csrf, expires_at)| AdminSessionRow { csrf, expires_at }))
}

pub async fn insert_session(
    db: &SqlitePool,
    token_hash: &str,
    csrf: &str,
    expires_at: &str,
) -> sqlx::Result<()> {
    sqlx::query(
        "INSERT INTO admin_sessions (token_hash, csrf_token, created_at, expires_at) VALUES (?, ?, ?, ?)",
    )
    .bind(token_hash)
    .bind(csrf)
    .bind(now_iso())
    .bind(expires_at)
    .execute(db)
    .await?;
    Ok(())
}

pub async fn touch_session(db: &SqlitePool, token_hash: &str, expires_at: &str) {
    sqlx::query("UPDATE admin_sessions SET expires_at = ? WHERE token_hash = ?")
        .bind(expires_at)
        .bind(token_hash)
        .execute(db)
        .await
        .ok();
}

pub async fn delete_session(db: &SqlitePool, token_hash: &str) {
    sqlx::query("DELETE FROM admin_sessions WHERE token_hash = ?")
        .bind(token_hash)
        .execute(db)
        .await
        .ok();
}

pub async fn purge_expired_sessions(db: &SqlitePool) {
    sqlx::query("DELETE FROM admin_sessions WHERE expires_at < ?")
        .bind(now_iso())
        .execute(db)
        .await
        .ok();
}

pub async fn stored_admin_hash(db: &SqlitePool) -> Option<String> {
    sqlx::query_scalar("SELECT value FROM settings WHERE key = 'admin_password_hash'")
        .fetch_optional(db)
        .await
        .ok()
        .flatten()
}

pub async fn store_admin_hash(db: &SqlitePool, hash: &str) -> Result<(), sqlx::Error> {
    sqlx::query("INSERT OR REPLACE INTO settings (key, value) VALUES ('admin_password_hash', ?)")
        .bind(hash)
        .execute(db)
        .await?;
    Ok(())
}

pub async fn has_any_post(db: &SqlitePool) -> bool {
    sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM posts")
        .fetch_one(db)
        .await
        .map(|count| count > 0)
        .unwrap_or(true)
}

pub fn unique_violation(error: &sqlx::Error) -> bool {
    matches!(error, sqlx::Error::Database(db) if db.is_unique_violation())
}
