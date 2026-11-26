use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::SqlitePool;

#[derive(Debug, Deserialize, Serialize, Default)]
pub struct TelemetryPayload {
    pub path: Option<String>,
    pub referrer: Option<String>,
    pub device: Option<String>,
    pub os: Option<String>,
    pub browser: Option<String>,
    pub screen: Option<String>,
    pub viewport: Option<String>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct CountItem {
    pub label: String,
    pub count: i64,
    #[sqlx(default)]
    pub percentage: i64,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct DayTrend {
    pub day: String,
    pub views: i64,
    pub visitors: i64,
    #[sqlx(default)]
    pub height_px: i64,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct RecentPageview {
    pub path: String,
    pub device_type: String,
    pub os: String,
    pub browser: String,
    pub viewport: String,
    pub created_at: String,
    #[sqlx(default)]
    pub time_ago: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct AnalyticsSummary {
    pub total_views: i64,
    pub unique_visitors: i64,
    pub today_views: i64,
    pub today_visitors: i64,
    pub mobile_pct: i64,
    pub top_device: String,
    pub top_os: String,
    pub top_browser: String,
    pub devices: Vec<CountItem>,
    pub os_list: Vec<CountItem>,
    pub browsers: Vec<CountItem>,
    pub viewports: Vec<CountItem>,
    pub trend: Vec<DayTrend>,
    pub recent: Vec<RecentPageview>,
}

/// Parses standard User-Agent headers to derive (device_type, os, browser).
pub fn parse_user_agent(ua: &str) -> (&'static str, &'static str, &'static str) {
    let ua_lower = ua.to_ascii_lowercase();

    // 1. Device type
    let device = if ua.contains("iPad")
        || ua_lower.contains("tablet")
        || (ua.contains("Android") && !ua.contains("Mobile"))
    {
        "tablet"
    } else if ua.contains("iPhone")
        || ua.contains("iPod")
        || (ua.contains("Android") && ua.contains("Mobile"))
        || ua_lower.contains("mobile")
        || ua_lower.contains("phone")
    {
        "mobile"
    } else {
        "desktop"
    };

    // 2. Operating System
    let os = if ua.contains("iPhone") || ua.contains("iPad") || ua.contains("iPod") || ua.contains("CPU OS") {
        "iOS"
    } else if ua.contains("Android") {
        "Android"
    } else if ua.contains("Macintosh") || ua.contains("Mac OS X") {
        "macOS"
    } else if ua.contains("Windows") {
        "Windows"
    } else if ua.contains("CrOS") {
        "ChromeOS"
    } else if ua.contains("Linux") {
        "Linux"
    } else {
        "Other"
    };

    // 3. Browser
    let browser = if ua.contains("Edg/") || ua.contains("Edge/") {
        "Edge"
    } else if ua.contains("OPR/") || ua.contains("Opera/") {
        "Opera"
    } else if ua.contains("SamsungBrowser") {
        "Samsung Internet"
    } else if ua.contains("Chrome/") || ua.contains("CriOS/") {
        "Chrome"
    } else if ua.contains("Firefox/") || ua.contains("FxiOS/") {
        "Firefox"
    } else if ua.contains("Safari/") && !ua.contains("Chrome/") && !ua.contains("Chromium/") {
        "Safari"
    } else {
        "Other"
    };

    (device, os, browser)
}

/// Generates a privacy-preserving daily visitor hash without storing IP addresses.
pub fn visitor_hash(ip: &str, ua: &str) -> String {
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let mut hasher = Sha256::new();
    hasher.update(today.as_bytes());
    hasher.update(b":portfolio-telemetry-salt:");
    hasher.update(ip.as_bytes());
    hasher.update(ua.as_bytes());
    let hex = hex::encode(hasher.finalize());
    hex[..16].to_string()
}

pub async fn record_pageview(
    db: &SqlitePool,
    path: &str,
    referrer: &str,
    device_type: &str,
    os: &str,
    browser: &str,
    screen: &str,
    viewport: &str,
    hash: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO pageviews (path, referrer, device_type, os, browser, screen_resolution, viewport, visitor_hash)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)"
    )
    .bind(path)
    .bind(referrer)
    .bind(device_type)
    .bind(os)
    .bind(browser)
    .bind(screen)
    .bind(viewport)
    .bind(hash)
    .execute(db)
    .await?;

    Ok(())
}

fn compute_percentages(mut items: Vec<CountItem>, total: i64) -> Vec<CountItem> {
    if total > 0 {
        for item in &mut items {
            item.percentage = ((item.count as f64 / total as f64) * 100.0).round() as i64;
        }
    }
    items
}

pub async fn get_analytics_summary(db: &SqlitePool) -> Result<AnalyticsSummary, sqlx::Error> {
    let total_views: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM pageviews")
        .fetch_one(db)
        .await
        .unwrap_or(0);

    let unique_visitors: i64 = sqlx::query_scalar("SELECT COUNT(DISTINCT visitor_hash) FROM pageviews")
        .fetch_one(db)
        .await
        .unwrap_or(0);

    let today_prefix = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let today_views: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM pageviews WHERE created_at LIKE ?1"
    )
    .bind(format!("{today_prefix}%"))
    .fetch_one(db)
    .await
    .unwrap_or(0);

    let today_visitors: i64 = sqlx::query_scalar(
        "SELECT COUNT(DISTINCT visitor_hash) FROM pageviews WHERE created_at LIKE ?1"
    )
    .bind(format!("{today_prefix}%"))
    .fetch_one(db)
    .await
    .unwrap_or(0);

    let raw_devices: Vec<(String, i64)> = sqlx::query_as(
        "SELECT device_type, COUNT(*) as count FROM pageviews GROUP BY device_type ORDER BY count DESC"
    )
    .fetch_all(db)
    .await
    .unwrap_or_default();

    let device_items: Vec<CountItem> = raw_devices
        .into_iter()
        .map(|(label, count)| CountItem { label, count, percentage: 0 })
        .collect();
    let devices = compute_percentages(device_items, total_views);

    let mobile_count: i64 = devices
        .iter()
        .filter(|d| d.label == "mobile")
        .map(|d| d.count)
        .sum();
    let mobile_pct = if total_views > 0 {
        ((mobile_count as f64 / total_views as f64) * 100.0).round() as i64
    } else {
        0
    };

    let raw_os: Vec<(String, i64)> = sqlx::query_as(
        "SELECT os, COUNT(*) as count FROM pageviews GROUP BY os ORDER BY count DESC LIMIT 6"
    )
    .fetch_all(db)
    .await
    .unwrap_or_default();
    let os_items: Vec<CountItem> = raw_os
        .into_iter()
        .map(|(label, count)| CountItem { label, count, percentage: 0 })
        .collect();
    let os_list = compute_percentages(os_items, total_views);

    let raw_browsers: Vec<(String, i64)> = sqlx::query_as(
        "SELECT browser, COUNT(*) as count FROM pageviews GROUP BY browser ORDER BY count DESC LIMIT 6"
    )
    .fetch_all(db)
    .await
    .unwrap_or_default();
    let browser_items: Vec<CountItem> = raw_browsers
        .into_iter()
        .map(|(label, count)| CountItem { label, count, percentage: 0 })
        .collect();
    let browsers = compute_percentages(browser_items, total_views);

    let raw_viewports: Vec<(String, i64)> = sqlx::query_as(
        "SELECT viewport, COUNT(*) as count FROM pageviews WHERE viewport != '' GROUP BY viewport ORDER BY count DESC LIMIT 5"
    )
    .fetch_all(db)
    .await
    .unwrap_or_default();
    let viewport_items: Vec<CountItem> = raw_viewports
        .into_iter()
        .map(|(label, count)| CountItem { label, count, percentage: 0 })
        .collect();
    let viewports = compute_percentages(viewport_items, total_views);

    let raw_trend: Vec<(String, i64, i64)> = sqlx::query_as(
        "SELECT substr(created_at, 1, 10) as day, COUNT(*) as views, COUNT(DISTINCT visitor_hash) as visitors
         FROM pageviews
         GROUP BY day
         ORDER BY day DESC
         LIMIT 10"
    )
    .fetch_all(db)
    .await
    .unwrap_or_default();

    let max_views = raw_trend.iter().map(|(_, v, _)| *v).max().unwrap_or(1).max(1);
    let mut trend: Vec<DayTrend> = raw_trend
        .into_iter()
        .rev()
        .map(|(day, views, visitors)| {
            let height_px = ((views as f64 / max_views as f64) * 80.0).round().max(4.0) as i64;
            DayTrend { day, views, visitors, height_px }
        })
        .collect();

    if trend.is_empty() {
        trend.push(DayTrend {
            day: today_prefix,
            views: 0,
            visitors: 0,
            height_px: 4,
        });
    }

    let raw_recent: Vec<(String, String, String, String, String, String)> = sqlx::query_as(
        "SELECT path, device_type, os, browser, viewport, created_at
         FROM pageviews
         ORDER BY id DESC
         LIMIT 15"
    )
    .fetch_all(db)
    .await
    .unwrap_or_default();

    let now = chrono::Utc::now();
    let recent: Vec<RecentPageview> = raw_recent
        .into_iter()
        .map(|(path, device_type, os, browser, viewport, created_at)| {
            let time_ago = chrono::DateTime::parse_from_rfc3339(&created_at)
                .map(|t| {
                    let diff = now.signed_duration_since(t.with_timezone(&chrono::Utc));
                    if diff.num_minutes() < 1 {
                        "just now".to_string()
                    } else if diff.num_minutes() < 60 {
                        format!("{}m ago", diff.num_minutes())
                    } else if diff.num_hours() < 24 {
                        format!("{}h ago", diff.num_hours())
                    } else {
                        format!("{}d ago", diff.num_days())
                    }
                })
                .unwrap_or_else(|_| "recent".to_string());
            RecentPageview {
                path,
                device_type,
                os,
                browser,
                viewport,
                created_at,
                time_ago,
            }
        })
        .collect();

    let top_device = devices.first().map(|d| d.label.clone()).unwrap_or_else(|| "None yet".into());
    let top_os = os_list.first().map(|o| o.label.clone()).unwrap_or_else(|| "None yet".into());
    let top_browser = browsers.first().map(|b| b.label.clone()).unwrap_or_else(|| "None yet".into());

    Ok(AnalyticsSummary {
        total_views,
        unique_visitors,
        today_views,
        today_visitors,
        mobile_pct,
        top_device,
        top_os,
        top_browser,
        devices,
        os_list,
        browsers,
        viewports,
        trend,
        recent,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ua_parsing() {
        let iphone = "Mozilla/5.0 (iPhone; CPU iPhone OS 17_4 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.4 Mobile/15E148 Safari/604.1";
        assert_eq!(parse_user_agent(iphone), ("mobile", "iOS", "Safari"));

        let ipad = "Mozilla/5.0 (iPad; CPU OS 17_4 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.4 Mobile/15E148 Safari/604.1";
        assert_eq!(parse_user_agent(ipad), ("tablet", "iOS", "Safari"));

        let android_phone = "Mozilla/5.0 (Linux; Android 14; Pixel 8) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/123.0.0.0 Mobile Safari/537.36";
        assert_eq!(parse_user_agent(android_phone), ("mobile", "Android", "Chrome"));

        let mac_chrome = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/123.0.0.0 Safari/537.36";
        assert_eq!(parse_user_agent(mac_chrome), ("desktop", "macOS", "Chrome"));

        let win_edge = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/123.0.0.0 Safari/537.36 Edg/123.0.0.0";
        assert_eq!(parse_user_agent(win_edge), ("desktop", "Windows", "Edge"));
    }

    #[test]
    fn test_visitor_hash_privacy() {
        let h1 = visitor_hash("192.168.1.1", "agent1");
        let h2 = visitor_hash("192.168.1.1", "agent1");
        let h3 = visitor_hash("192.168.1.2", "agent1");
        assert_eq!(h1, h2);
        assert_ne!(h1, h3);
        assert_eq!(h1.len(), 16);
    }
}
