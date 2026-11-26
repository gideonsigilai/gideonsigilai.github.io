-- Device and pageview telemetry.
-- No IP addresses or personal data are stored.

CREATE TABLE IF NOT EXISTS pageviews (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    path TEXT NOT NULL DEFAULT '/',
    referrer TEXT NOT NULL DEFAULT '',
    device_type TEXT NOT NULL DEFAULT 'desktop',
    os TEXT NOT NULL DEFAULT 'Other',
    browser TEXT NOT NULL DEFAULT 'Other',
    screen_resolution TEXT NOT NULL DEFAULT '',
    viewport TEXT NOT NULL DEFAULT '',
    visitor_hash TEXT NOT NULL DEFAULT '',
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
);

CREATE INDEX IF NOT EXISTS idx_pageviews_created_at ON pageviews(created_at);
CREATE INDEX IF NOT EXISTS idx_pageviews_device ON pageviews(device_type);
CREATE INDEX IF NOT EXISTS idx_pageviews_os ON pageviews(os);
CREATE INDEX IF NOT EXISTS idx_pageviews_browser ON pageviews(browser);
