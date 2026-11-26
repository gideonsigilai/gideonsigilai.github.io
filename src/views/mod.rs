
use serde_json::{json, Value};

pub mod icons;

use crate::portfolio::{ProjectRow, Site, SocialRow};
use crate::template::{escape_html, render};
use crate::views::icons::icon;

// Built by build.rs → web/build.ts: templates/layout.html and
// templates/admin_layout.html, each with its own stylesheet, client script, and
// icon inlined. Not checked in; the build recreates both documents.
pub const LAYOUT: &str = include_str!(concat!(env!("OUT_DIR"), "/portfolio.html"));
pub const ADMIN_LAYOUT: &str = include_str!(concat!(env!("OUT_DIR"), "/admin.html"));
pub const HOME: &str = include_str!("../../templates/home.html");
pub const WORK_DETAIL: &str = include_str!("../../templates/work_detail.html");
pub const WRITING: &str = include_str!("../../templates/writing.html");
pub const POST: &str = include_str!("../../templates/post.html");
pub const MESSAGE: &str = include_str!("../../templates/message.html");
pub const NOT_FOUND: &str = include_str!("../../templates/not_found.html");
pub const LOGIN: &str = include_str!("../../templates/login.html");
pub const ADMIN_INBOX: &str = include_str!("../../templates/admin_inbox.html");
pub const ADMIN_POSTS: &str = include_str!("../../templates/admin_posts.html");
pub const ADMIN_CONTENT: &str = include_str!("../../templates/admin_content.html");
pub const ADMIN_ANALYTICS: &str = include_str!("../../templates/admin_analytics.html");

pub struct Page {
    pub title: String,
    pub description: String,
    pub canonical: String,
    pub path: String,
    pub og_type: &'static str,
    pub head_extra: String,
    pub noindex: bool,
    pub social_image: Option<String>,
    pub social_image_alt: Option<String>,
}

impl Page {
    pub fn new(title: impl Into<String>, description: impl Into<String>, path: impl Into<String>) -> Self {
        let path = path.into();
        Self {
            title: title.into(),
            description: description.into(),
            canonical: path.clone(),
            path,
            og_type: "website",
            head_extra: String::new(),
            noindex: false,
            social_image: None,
            social_image_alt: None,
        }
    }

    pub fn head(mut self, extra: impl Into<String>) -> Self {
        self.head_extra = extra.into();
        self
    }

    pub fn article(mut self) -> Self {
        self.og_type = "article";
        self
    }

    pub fn noindex(mut self) -> Self {
        self.noindex = true;
        self
    }

    pub fn social_image(mut self, url: impl Into<String>) -> Self {
        self.social_image = Some(url.into());
        self
    }

    pub fn social_image_alt(mut self, text: impl Into<String>) -> Self {
        self.social_image_alt = Some(text.into());
        self
    }
}

pub fn json_ld(nodes: Vec<Value>) -> String {
    let payload = if nodes.len() == 1 { nodes[0].clone() } else { Value::Array(nodes) };
    format!(
        r#"<script type="application/ld+json">{}</script>"#,
        serde_json::to_string(&payload)
            .unwrap_or_default()
            .replace('<', "\\u003c")
    )
}

/// Dashboard tabs. The three pages are the whole signed-in surface.
fn admin_nav_items(path: &str) -> Value {
    let in_section = |section: &str| path == section || path.starts_with(&format!("{section}/"));
    json!([
        { "href": "/", "label": "Inbox", "short": "01", "active": path == "/" },
        { "href": "/posts", "label": "Writing", "short": "02", "active": in_section("/posts") },
        { "href": "/content", "label": "Portfolio", "short": "03", "active": in_section("/content") },
        { "href": "/analytics", "label": "Analytics", "short": "04", "active": in_section("/analytics") }
    ])
}

fn nav_items(path: &str) -> Value {
    let writing_active = path == "/writing" || path.starts_with("/writing/");
    json!([
        { "href": "/#work", "label": "Work", "short": "01", "active": false },
        { "href": "/#about", "label": "About", "short": "02", "active": false },
        { "href": "/#toolkit", "label": "Toolkit", "short": "03", "active": false },
        { "href": "/writing", "label": "Journal", "short": "04", "active": writing_active },
        { "href": "/#contact", "label": "Contact", "short": "05", "active": false, "contact": true }
    ])
}

/// Renders a public page into the portfolio shell (port 8050).
pub fn render_page(
    site: &Site,
    page: Page,
    state: &crate::state::AppState,
    body: String,
) -> String {
    render_shell(LAYOUT, false, site, page, state, body)
}

/// Renders a signed-in page into the dashboard shell (port 8051).
pub fn render_admin_page(site: &Site, page: Page, state: &crate::state::AppState, body: String) -> String {
    render_shell(ADMIN_LAYOUT, true, site, page, state, body)
}

fn render_shell(
    shell: &str,
    admin: bool,
    site: &Site,
    page: Page,
    state: &crate::state::AppState,
    body: String,
) -> String {
    // Absolutise the canonical so crawlers and share cards agree, using the
    // origin that actually serves the document: dashboard pages are canonical
    // on the dashboard origin, which is also why nothing links to them.
    let base = if admin { state.config.admin_url.as_str() } else { state.base_url() };
    let canonical = if page.canonical.starts_with("https://") || page.canonical.starts_with("http://") {
        page.canonical.clone()
    } else {
        format!("{base}{}", page.canonical)
    };
    let profile = &site.profile;
    let social_image = page
        .social_image
        .clone()
        .unwrap_or_else(|| format!("{}/images/gideon-sigilai.png", state.base_url()));
    let social_image_alt = page
        .social_image_alt
        .clone()
        .unwrap_or_else(|| {
            format!(
                "{} — {}, {}. Branded portfolio artwork for mobile, web, and applied AI.",
                profile.name, profile.job_title, profile.location
            )
        });
    let social_image_type = if social_image.ends_with(".jpg") || social_image.ends_with(".jpeg") {
        "image/jpeg"
    } else if social_image.ends_with(".png") {
        "image/png"
    } else if social_image.ends_with(".webp") {
        "image/webp"
    } else if social_image.ends_with(".svg") {
        "image/svg+xml"
    } else {
        "image/png"
    };
    let data = json!({
        "title": page.title,
        "description": page.description,
        "canonical": canonical,
        "og_type": page.og_type,
        "social_image": social_image,
        "social_image_type": social_image_type,
        "social_image_alt": social_image_alt,
        "site_name": profile.name,
        "job_title": profile.job_title,
        "path": page.path,
        "noindex": page.noindex,
        "head_extra": page.head_extra,
        "body": body,
        "nav": if admin { admin_nav_items(&page.path) } else { nav_items(&page.path) },
        "socials": socials_json(&site.socials),
        "email": profile.email,
        "year": chrono::Utc::now().format("%Y").to_string(),
        "location": profile.location,
        "initials": initials_of(&profile.name),
        "twitter": twitter_handle(&site.socials),
        // The browser posts contact messages and pulls JSON from the backend,
        // which runs on its own origin.
        "api_base": state.api_url(),
        // "View site" from the dashboard always points at the portfolio, even
        // though the dashboard's own canonical is its private origin.
        "site_url": state.base_url(),
    });
    render(shell, &data)
}

pub fn page_title(site: &Site, title: &str) -> String {
    format!("{} — {}", title, site.profile.name)
}

/// Initials for the brand mark, e.g. "gs".
pub fn initials_of(name: &str) -> String {
    let initials: String = name
        .split_whitespace()
        .filter_map(|word| word.chars().next())
        .take(2)
        .collect();
    initials.to_lowercase()
}

/// The X/Twitter handle from the social list, used by the card metadata.
fn twitter_handle(socials: &[SocialRow]) -> String {
    socials
        .iter()
        .find(|social| social.icon == "x")
        .map(|social| social.handle.clone())
        .unwrap_or_default()
}

fn socials_json(socials: &[SocialRow]) -> Value {
    Value::Array(
        socials
            .iter()
            .map(|social| {
                json!({
                    "label": social.label,
                    "handle": social.handle,
                    "href": social.href,
                    "icon": social.icon,
                })
            })
            .collect(),
    )
}

pub fn person_schema(site: &Site, base: &str) -> Value {
    let profile = &site.profile;
    let mut names = profile.name.split_whitespace();
    let given_name = names.next().unwrap_or_default();
    let family_name = names.collect::<Vec<_>>().join(" ");
    let mut person = json!({
        "@context": "https://schema.org",
        "@type": "Person",
        "@id": format!("{base}/#person"),
        "name": profile.name,
        "jobTitle": profile.job_title,
        "description": profile.description,
        "url": format!("{base}/"),
        "address": { "@type": "PostalAddress", "addressLocality": profile.location },
        "sameAs": site.socials.iter()
            .filter(|social| social.href.starts_with("https://") || social.href.starts_with("http://"))
            .map(|social| social.href.as_str())
            .collect::<Vec<_>>(),
        "knowsAbout": site.tools.iter().map(|tool| tool.name.as_str()).collect::<Vec<_>>()
    });
    if !given_name.is_empty() {
        person["givenName"] = json!(given_name);
    }
    if !family_name.is_empty() {
        person["familyName"] = json!(family_name);
    }
    person
}

pub fn website_schema(site: &Site, base: &str) -> Value {
    json!({
        "@context": "https://schema.org",
        "@type": "WebSite",
        "@id": format!("{base}/#website"),
        "url": format!("{base}/"),
        "name": format!("{} — Portfolio", site.profile.name),
        "description": site.profile.description,
        "inLanguage": "en",
        "publisher": { "@id": format!("{base}/#person"), "@type": "Person", "name": site.profile.name }
    })
}

pub fn breadcrumb_schema(base: &str, items: &[(&str, &str)]) -> Value {
    json!({
        "@context": "https://schema.org",
        "@type": "BreadcrumbList",
        "itemListElement": items.iter().enumerate().map(|(index, (name, path))| json!({
            "@type": "ListItem",
            "position": index + 1,
            "name": name,
            "item": format!("{base}{path}")
        })).collect::<Vec<_>>()
    })
}

pub fn blog_posting_schema(site: &Site, base: &str, post: &crate::content::PostRow, plain: &str) -> Value {
    let image = if post.cover_image.is_empty() {
        cover_art_schema(
            base,
            &format!("/images/writing/{}.svg", post.slug),
            &format!("Cover artwork for {} by {}.", post.title, site.profile.name),
        )
    } else {
        let url = if post.cover_image.starts_with("http://") || post.cover_image.starts_with("https://") {
            post.cover_image.clone()
        } else {
            format!("{base}{}", post.cover_image)
        };
        json!({
            "@type": "ImageObject",
            "url": url,
            "contentUrl": url,
            "encodingFormat": if post.cover_image.ends_with(".jpg") || post.cover_image.ends_with(".jpeg") { "image/jpeg" } else if post.cover_image.ends_with(".png") { "image/png" } else if post.cover_image.ends_with(".webp") { "image/webp" } else { "image/svg+xml" },
            "width": 1200,
            "height": 630,
            "caption": format!("Cover artwork for {} by {}.", post.title, site.profile.name),
        })
    };
    json!({
        "@context": "https://schema.org",
        "@type": "BlogPosting",
        "@id": format!("{base}/writing/{}#article", post.slug),
        "url": format!("{base}/writing/{}", post.slug),
        "mainEntityOfPage": { "@type": "WebPage", "@id": format!("{base}/writing/{}", post.slug) },
        "headline": post.title,
        "description": post.excerpt,
        "inLanguage": "en",
        "datePublished": post.published_at,
        "dateModified": post.updated_at,
        "wordCount": plain.split_whitespace().count(),
        "timeRequired": format!("PT{}M", post.reading_min),
        "keywords": post.tags().join(", "),
        "author": { "@id": format!("{base}/#person"), "@type": "Person", "name": site.profile.name, "url": format!("{base}/") },
        "publisher": { "@id": format!("{base}/#person"), "@type": "Person", "name": site.profile.name, "url": format!("{base}/") },
        "image": image,
        "isPartOf": { "@id": format!("{base}/#website") }
    })
}

pub fn creative_work_schema(site: &Site, base: &str, project: &ProjectRow) -> Value {
    let image_url = if project.image.is_empty() {
        format!("{base}/images/work/{}.svg", project.slug)
    } else if project.image.starts_with("http://") || project.image.starts_with("https://") {
        project.image.clone()
    } else {
        format!("{base}{}", project.image)
    };
    let image = if project.image.is_empty() {
        cover_art_schema(
            base,
            &format!("/images/work/{}.svg", project.slug),
            &format!("{} project artwork by {}.", project.name, site.profile.name),
        )
    } else {
        json!({
            "@type": "ImageObject",
            "url": image_url,
            "contentUrl": image_url,
            "encodingFormat": if project.image.ends_with(".jpg") || project.image.ends_with(".jpeg") { "image/jpeg" } else if project.image.ends_with(".png") { "image/png" } else if project.image.ends_with(".webp") { "image/webp" } else { "image/svg+xml" },
            "width": 1200,
            "height": 630,
            "caption": format!("{} project artwork by {}.", project.name, site.profile.name),
        })
    };
    json!({
        "@context": "https://schema.org",
        "@type": "CreativeWork",
        "@id": format!("{base}/work/{}#project", project.slug),
        "name": project.name,
        "description": project.summary,
        "url": format!("{base}/work/{}", project.slug),
        "mainEntityOfPage": { "@type": "WebPage", "@id": format!("{base}/work/{}", project.slug) },
        "creator": { "@id": format!("{base}/#person"), "@type": "Person", "name": site.profile.name, "url": format!("{base}/") },
        "image": image,
        "keywords": project.stack().join(", "),
        "inLanguage": "en",
        "isPartOf": { "@id": format!("{base}/#website") }
    })
}

fn cover_art_schema(base: &str, path: &str, caption: &str) -> Value {
    json!({
        "@type": "ImageObject",
        "url": format!("{base}{path}"),
        "contentUrl": format!("{base}{path}"),
        "encodingFormat": "image/svg+xml",
        "width": 1200,
        "height": 630,
        "caption": caption
    })
}

pub fn date_label(value: &str) -> String {
    chrono::DateTime::parse_from_rfc3339(value)
        .map(|parsed| parsed.format("%-d %B %Y").to_string())
        .unwrap_or_else(|_| value.to_string())
}

pub fn short_date(value: &str) -> String {
    chrono::DateTime::parse_from_rfc3339(value)
        .map(|parsed| parsed.format("%-d %b, %H:%M").to_string())
        .unwrap_or_else(|_| value.to_string())
}

// ---------------------------------------------------------------- shared fragments

pub fn social_row(socials: &[SocialRow], with_handles: bool) -> String {
    let mut out = String::from(r#"<div class="social-row" aria-label="Elsewhere">"#);
    for social in socials {
        let external = !social.href.starts_with("mailto:");
        let attrs = if external {
            r#" target="_blank" rel="noopener noreferrer me""#
        } else {
            ""
        };
        let label = if with_handles { &social.label } else { &social.handle };
        out.push_str(&format!(
            r#"<a class="social-link" href="{}"{} aria-label="{} — {}">{}{}</a>"#,
            escape_html(&social.href),
            attrs,
            escape_html(&social.label),
            escape_html(&social.handle),
            icon(&social.icon, ""),
            escape_html(label),
        ));
    }
    out.push_str("</div>");
    out
}

/// Escaped option list for the contact form's topic picker.
pub fn topic_options(topics: &[String]) -> String {
    topics
        .iter()
        .map(|topic| format!(r#"<option value="{}">{}</option>"#, escape_html(topic), escape_html(topic)))
        .collect()
}
