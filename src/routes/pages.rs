use axum::{
    extract::{Path, Query, State},
    http::{header, StatusCode},
    response::{Html, IntoResponse, Response},
};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::content::{
    all_posts, published_by_slug, published_by_tag, related, tag_counts, PostRow,
};
use crate::portfolio::{self, ProjectRow, Site};
use crate::state::{ApiError, ApiResult, AppState};
use crate::template::{escape_html, render};
use crate::text::{normalise_headings, plain_text};
use crate::views::{
    blog_posting_schema, breadcrumb_schema, creative_work_schema, date_label, initials_of, json_ld,
    page_title, person_schema, post_image_alt, project_image_alt, social_row, topic_options,
    website_schema, Page, HOME, POST, WORK_DETAIL, WRITING,
};

fn base(state: &AppState) -> &str {
    state.base_url()
}

fn html(body: String) -> Response {
    Html(body).into_response()
}

fn post_card(post: &PostRow) -> Value {
    let tags = post.tags();
    json!({
        "slug": post.slug,
        "title": post.title,
        "excerpt": post.excerpt,
        "reading_min": post.reading_min,
        "published_at": post.published_at.clone().unwrap_or_default(),
        "date": post.published_at.as_deref().map(date_label).unwrap_or_else(|| date_label(&post.created_at)),
        "tag": tags.first().cloned().unwrap_or_default(),
        "cover_note": if post.cover_note.is_empty() { "Notes from the build." } else { &post.cover_note },
        "cover_image": post.cover_image,
        "image_alt": post_image_alt(post),
        "tags": tags.join(" · "),
    })
}

fn post_topics(posts: &[PostRow]) -> Vec<String> {
    let mut topics = Vec::new();
    for tag in posts.iter().flat_map(PostRow::tags) {
        if !topics
            .iter()
            .any(|topic: &String| topic.eq_ignore_ascii_case(&tag))
        {
            topics.push(tag);
        }
    }
    topics
}

fn tags_html(post: &PostRow) -> String {
    post.tags()
        .iter()
        .map(|tag| {
            format!(
                r#"<span class="chip" role="listitem">{}</span>"#,
                escape_html(tag)
            )
        })
        .collect::<Vec<_>>()
        .join("")
}

/// Card payload for the home grid. Only the fields the markup renders are sent;
/// the case study itself is fetched from `/api/projects/{slug}` or a work page.
fn project_card(project: &ProjectRow, index: usize) -> Value {
    let mut card = project.card();
    let object = card.as_object_mut().expect("card is an object");
    object.insert("image_alt".into(), json!(project_image_alt(project)));
    if let Some(Value::Array(stack)) = object.remove("stack") {
        object.insert(
            "stack_tags".into(),
            Value::Array(
                stack
                    .into_iter()
                    .map(|label| json!({ "label": label }))
                    .collect(),
            ),
        );
    }
    object.insert("wide".into(), json!(index == 0));
    let is_play = project.link_href.contains("play.google.com");
    object.insert("playstore".into(), json!(is_play));
    if is_play {
        object.insert("playstore_href".into(), json!(project.link_href));
    }
    card
}

/// Filter buttons with live counts, so adding a project never leaves a stale
/// hard-coded number on the home page.
fn filter_buttons(site: &Site) -> Vec<Value> {
    site.categories
        .iter()
        .map(|category| {
            let count = site
                .projects
                .iter()
                .filter(|project| project.category == category.id)
                .count();
            json!({ "id": category.id, "label": category.label, "count": count })
        })
        .collect()
}

// ---------------------------------------------------------------- home

pub async fn home(State(state): State<AppState>) -> ApiResult<Response> {
    let posts = all_posts(&state.db)
        .await
        .map_err(ApiError::internal)?
        .into_iter()
        .filter(PostRow::is_published)
        .collect::<Vec<_>>();

    let cards: Vec<Value> = posts.iter().take(3).map(post_card).collect();
    let site = Site::load(&state.db).await;

    let project_values: Vec<Value> = site
        .projects
        .iter()
        .enumerate()
        .map(|(index, project)| project_card(project, index))
        .collect();

    // Chips carry only the tool name: descriptions and project links arrive
    // from /api/toolkit when the visitor explores, keeping the page light.
    let groups: Vec<Value> = site
        .groups
        .iter()
        .map(|group| {
            let chips = site
                .tools
                .iter()
                .filter(|tool| tool.group_id == group.id)
                .map(|tool| {
                    format!(
                        r#"<button type="button" class="tool-chip" data-tool="{}" aria-pressed="false" aria-controls="tool-explorer">{}</button>"#,
                        escape_html(&tool.name),
                        escape_html(&tool.name),
                    )
                })
                .collect::<String>();
            json!({
                "id": group.id,
                "number": group.number,
                "title": group.title,
                "subtitle": group.subtitle,
                "chips": chips,
            })
        })
        .collect();

    let body = render(
        HOME,
        &json!({
            "name": site.profile.name,
            "author": site.profile.name,
            "job_title": site.profile.job_title,
            "location": site.profile.location,
            "email": site.profile.email,
            "availability": site.profile.availability,
            "hero_lead": site.profile.hero_lead,
            "hero_focus": site.profile.hero_focus,
            "hero_focus_detail": site.profile.hero_focus_detail,
            "about_intro": site.profile.about_intro,
            "initials": format!("{}.", initials_of(&site.profile.name)),
            "socials": social_row(&site.socials, false),
            "projects": project_values,
            "project_total": site.projects.len(),
            "filters": filter_buttons(&site),
            "principles": site.principles.iter().map(|principle| json!({
                "title": principle.title,
                "body": principle.body,
            })).collect::<Vec<_>>(),
            "milestones": site.milestones.iter().map(|milestone| json!({
                "marker": milestone.marker,
                "title": milestone.title,
                "detail": milestone.detail,
            })).collect::<Vec<_>>(),
            "tool_groups": groups,
            "tool_count": site.tools.len(),
            "group_count": site.groups.len(),
            "topics": topic_options(&site.topics),
            "has_posts": !posts.is_empty(),
            "posts": cards,
        }),
    );

    let head = json_ld(vec![
        person_schema(&site, base(&state)),
        website_schema(&site, base(&state)),
        json!({
            "@context": "https://schema.org",
            "@type": "ProfilePage",
            "@id": format!("{}/#profile", base(&state)),
            "url": format!("{}/", base(&state)),
            "name": format!("{} — {}", site.profile.name, site.profile.job_title),
            "description": site.profile.description,
            "inLanguage": "en",
            "mainEntity": { "@id": format!("{}/#person", base(&state)) },
            "isPartOf": { "@id": format!("{}/#website", base(&state)) },
            "primaryImageOfPage": {
                "@type": "ImageObject",
                "url": format!("{}/images/gideon-sigilai.png", base(&state)),
                "width": 1200,
                "height": 630,
                "caption": format!("Branded portfolio artwork for {}.", site.profile.name)
            }
        }),
    ]);
    let page = Page::new(
        format!(
            "{} — {} in {}",
            site.profile.name, site.profile.job_title, site.profile.location
        ),
        site.profile.description.clone(),
        "/",
    )
    .head(head);
    Ok(html(crate::views::render_page(&site, page, &state, body)))
}

// ---------------------------------------------------------------- work detail

pub async fn work_detail(
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> ApiResult<Response> {
    let site = Site::load(&state.db).await;
    let Some(project) = portfolio::visible_project(&state.db, &slug).await else {
        return not_found_response(&state);
    };

    let stack = project.stack();
    let stack_chips = stack
        .iter()
        .map(|item| {
            format!(
                r#"<span class="chip" role="listitem">{}</span>"#,
                escape_html(item)
            )
        })
        .collect::<String>();

    let story = project
        .story()
        .iter()
        .map(|item| format!("<p>{}</p>", escape_html(item)))
        .collect::<String>();

    let outcome_list = project
        .outcome()
        .iter()
        .map(|item| format!(r#"<li><span>{}</span></li>"#, escape_html(item)))
        .collect::<String>();

    let project_link = if project.link_href.is_empty() {
        String::new()
    } else {
        format!(
            r#"<p style="margin-top:1.15rem"><a class="text-link" href="{}" target="_blank" rel="noopener noreferrer">{} ↗</a></p>"#,
            escape_html(&project.link_href),
            escape_html(if project.link_label.is_empty() {
                "Open"
            } else {
                &project.link_label
            }),
        )
    };

    let others: Vec<Value> = site
        .projects
        .iter()
        .filter(|item| item.slug != project.slug)
        .map(|item| item.card())
        .collect();

    let body = render(
        WORK_DETAIL,
        &json!({
            "author": site.profile.name,
            "project": json!({
                "slug": project.slug,
                "name": project.name,
                "category_label": project.category_label,
                "kicker": project.kicker,
                "tagline": project.tagline,
                "summary": project.summary,
                "year": project.year,
                "role": project.role,
                "status": project.status,
                "accent": project.accent,
                "image": project.image,
                "image_alt": project_image_alt(&project),
                "stack_line": stack.join(" · "),
                "playstore": project.link_href.contains("play.google.com"),
                "playstore_href": if project.link_href.contains("play.google.com") { project.link_href.clone() } else { String::new() },
                "features": project.features().into_iter().map(|(title, detail)| json!({
                    "title": title,
                    "detail": detail,
                })).collect::<Vec<_>>(),
            }),
            "stack_chips": stack_chips,
            "story": story,
            "outcome_list": outcome_list,
            "project_link": project_link,
            "others": others,
            "socials": social_row(&site.socials, false),
        }),
    );

    let head = json_ld(vec![
        person_schema(&site, base(&state)),
        website_schema(&site, base(&state)),
        creative_work_schema(&site, base(&state), &project),
        breadcrumb_schema(
            base(&state),
            &[
                ("Home", "/"),
                ("Work", "/#work"),
                (&project.name, &format!("/work/{slug}")),
            ],
        ),
    ]);
    let discipline = project
        .category_label
        .rsplit('/')
        .next()
        .unwrap_or_default()
        .trim();
    let project_title = if discipline.is_empty()
        || project
            .name
            .to_lowercase()
            .contains(&discipline.to_lowercase())
    {
        format!("{} case study", project.name)
    } else {
        format!("{} {} case study", project.name, discipline)
    };
    let description = format!(
        "{} {} case study by {}, {} in {}. Technologies: {}.",
        project.summary,
        project.name,
        site.profile.name,
        site.profile.job_title,
        site.profile.location,
        stack.join(", ")
    );
    let page = Page::new(
        page_title(&site, &project_title),
        description,
        format!("/work/{slug}"),
    )
    .head(head);
    let page = if !project.image.is_empty() {
        let image_url =
            if project.image.starts_with("http://") || project.image.starts_with("https://") {
                project.image.clone()
            } else {
                format!("{}{}", base(&state), project.image)
            };
        page.social_image(image_url)
            .social_image_alt(project_image_alt(&project))
    } else {
        page
    };
    Ok(html(crate::views::render_page(&site, page, &state, body)))
}

// ---------------------------------------------------------------- writing

pub async fn writing(State(state): State<AppState>) -> ApiResult<Response> {
    let posts = all_posts(&state.db)
        .await
        .map_err(ApiError::internal)?
        .into_iter()
        .filter(PostRow::is_published)
        .collect::<Vec<_>>();
    let has_posts = !posts.is_empty();
    let topics = post_topics(&posts);
    let cards: Vec<Value> = posts
        .iter()
        .map(|post| {
            let mut card = post_card(post);
            card["tags"] = Value::String(tags_html(post));
            card
        })
        .collect();

    let site = Site::load(&state.db).await;
    let body = render(
        WRITING,
        &json!({
            "posts": cards,
            "has_posts": has_posts,
            "author": site.profile.name,
        }),
    );
    let topic_summary = topics
        .iter()
        .take(5)
        .map(String::as_str)
        .collect::<Vec<_>>()
        .join(", ");
    let description = if topic_summary.is_empty() {
        format!(
            "Software engineering notes by {}, {} in {}. Ideas and lessons from building software.",
            site.profile.name, site.profile.job_title, site.profile.location
        )
    } else {
        format!(
            "Software engineering notes by {} on {}. Practical architecture and development lessons from shipped projects.",
            site.profile.name, topic_summary
        )
    };
    let head = json_ld(vec![
        person_schema(&site, base(&state)),
        website_schema(&site, base(&state)),
        json!({
            "@context": "https://schema.org",
            "@type": "CollectionPage",
            "@id": format!("{}/writing#collection", base(&state)),
            "url": format!("{}/writing", base(&state)),
            "name": format!("Software engineering notes — {}", site.profile.name),
            "description": description,
            "keywords": topics.join(", "),
            "inLanguage": "en",
            "author": { "@id": format!("{}/#person", base(&state)) },
            "isPartOf": { "@id": format!("{}/#website", base(&state)) },
            "mainEntity": {
                "@type": "ItemList",
                "numberOfItems": posts.len(),
                "itemListElement": posts.iter().enumerate().map(|(index, post)| json!({
                    "@type": "ListItem",
                    "position": index + 1,
                    "item": {
                        "@type": "BlogPosting",
                        "@id": format!("{}/writing/{}#article", base(&state), post.slug),
                        "url": format!("{}/writing/{}", base(&state), post.slug),
                        "headline": post.title,
                        "description": post.excerpt,
                        "keywords": post.tags().join(", "),
                        "datePublished": post.published_at,
                        "author": { "@id": format!("{}/#person", base(&state)) }
                    }
                })).collect::<Vec<_>>()
            }
        }),
        breadcrumb_schema(base(&state), &[("Home", "/"), ("Writing", "/writing")]),
    ]);
    let page = Page::new(
        page_title(&site, "Software engineering notes"),
        description,
        "/writing",
    )
    .head(head);
    Ok(html(crate::views::render_page(&site, page, &state, body)))
}

// ---------------------------------------------------------------- post

pub async fn post_detail(
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> ApiResult<Response> {
    let site = Site::load(&state.db).await;
    let Some(post) = published_by_slug(&state.db, &slug)
        .await
        .map_err(ApiError::internal)?
    else {
        return not_found_response(&state);
    };
    let more = related(&state.db, &post.id, 3).await;
    let article = normalise_headings(&post.body_html, &post.title);
    let plain = plain_text(&post.body_html);
    let tags = post.tags();

    let body = render(
        POST,
        &json!({
            "post": {
                "slug": post.slug,
                "title": post.title,
                "excerpt": post.excerpt,
                "reading_min": post.reading_min,
                "cover_image": post.cover_image,
                "image_alt": post_image_alt(&post),
                "published_at": post.published_at.clone().unwrap_or_default(),
            },
            "date": post.published_at.as_deref().map(date_label).unwrap_or_default(),
            "topic_chips": tags_html(&post),
            "article": article,
            "socials": social_row(&site.socials, false),
            "author": site.profile.name,
            "job_title": site.profile.job_title,
            "hero_focus": site.profile.hero_focus,
            "location": site.profile.location,
            "has_more": !more.is_empty(),
            "more": more.iter().map(|item| json!({
                "slug": item.slug,
                "title": item.title,
            })).collect::<Vec<_>>(),
        }),
    );

    let head = json_ld(vec![
        person_schema(&site, base(&state)),
        website_schema(&site, base(&state)),
        blog_posting_schema(&site, base(&state), &post, &plain),
        breadcrumb_schema(
            base(&state),
            &[
                ("Home", "/"),
                ("Writing", "/writing"),
                (&post.title, &format!("/writing/{slug}")),
            ],
        ),
    ]);
    let article_tags = tags
        .iter()
        .map(|tag| {
            format!(
                r#"<meta property="article:tag" content="{}">"#,
                escape_html(tag)
            )
        })
        .collect::<String>();
    let extra = format!(
        r#"<meta property="article:published_time" content="{}"><meta property="article:modified_time" content="{}"><meta property="article:author" content="{}"><meta property="article:section" content="{}">{}{}"#,
        escape_html(post.published_at.as_deref().unwrap_or_default()),
        escape_html(&post.updated_at),
        escape_html(&format!("{}/#about", base(&state))),
        escape_html(tags.first().map(String::as_str).unwrap_or_default()),
        article_tags,
        head
    );
    let social_image_alt = post_image_alt(&post);
    let page = Page::new(
        page_title(&site, &post.title),
        post.excerpt,
        format!("/writing/{slug}"),
    )
    .article()
    .head(extra);
    let page = if !post.cover_image.is_empty() {
        let image_url = if post.cover_image.starts_with("http://")
            || post.cover_image.starts_with("https://")
        {
            post.cover_image.clone()
        } else {
            format!("{}{}", base(&state), post.cover_image)
        };
        page.social_image(image_url)
            .social_image_alt(social_image_alt)
    } else {
        page
    };
    Ok(html(crate::views::render_page(&site, page, &state, body)))
}

// ---------------------------------------------------------------- feeds

pub async fn rss(State(state): State<AppState>) -> Response {
    let posts = all_posts(&state.db).await.unwrap_or_default();
    let site = Site::load(&state.db).await;
    let base_url = base(&state).to_string();
    let published: Vec<&PostRow> = posts.iter().filter(|post| post.is_published()).collect();
    let last_build = published
        .first()
        .map(|post| post.updated_at.clone())
        .unwrap_or_else(crate::content::now_iso);

    let items = published
        .iter()
        .map(|post| {
            let url = format!("{base_url}/writing/{}", post.slug);
            let categories = post
                .tags()
                .iter()
                .map(|tag| format!("      <category>{}</category>", escape_html(tag)))
                .collect::<Vec<_>>()
                .join("\n");
            format!(
                "    <item>\n      <title>{}</title>\n      <link>{url}</link>\n      <guid isPermaLink=\"true\">{url}</guid>\n      <pubDate>{}</pubDate>\n      <description>{}</description>\n{categories}\n      <content:encoded><![CDATA[{}]]></content:encoded>\n    </item>",
                post.title,
                rfc2822(post.published_at.as_deref().unwrap_or(&post.created_at)),
                post.excerpt,
                post.body_html,
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    let xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0" xmlns:content="http://purl.org/rss/1.0/modules/content/" xmlns:atom="http://www.w3.org/2005/Atom">
  <channel>
    <title>{site} — Writing</title>
    <link>{base_url}/writing</link>
    <atom:link href="{base_url}/rss.xml" rel="self" type="application/rss+xml" />
    <description>{description}</description>
    <language>en</language>
    <lastBuildDate>{last}</lastBuildDate>
    <managingEditor>{email} ({site})</managingEditor>
{items}
  </channel>
</rss>
"#,
        site = site.profile.name,
        description = escape_html(&site.profile.description),
        email = site.profile.email,
        last = rfc2822(&last_build),
    );

    (
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "application/rss+xml; charset=utf-8"),
            (header::CACHE_CONTROL, "public, max-age=300"),
        ],
        xml,
    )
        .into_response()
}

fn rfc2822(value: &str) -> String {
    chrono::DateTime::parse_from_rfc3339(value)
        .map(|parsed| parsed.to_rfc2822())
        .unwrap_or_else(|_| value.to_string())
}

pub async fn sitemap(State(state): State<AppState>) -> Response {
    let base_url = base(&state);
    // Read only public records and the stored modification times, without
    // loading article bodies or imposing the dashboard's 200-record limit.
    let projects = sqlx::query_as::<_, (String, String, String)>(
        "SELECT slug, updated_at, image FROM projects WHERE visible = 1 ORDER BY position, slug",
    )
    .fetch_all(&state.db)
    .await;
    let posts = sqlx::query_as::<_, (String, String, String)>(
        "SELECT slug, updated_at, cover_image FROM posts WHERE status = 'published'
         ORDER BY COALESCE(published_at, created_at) DESC, slug",
    )
    .fetch_all(&state.db)
    .await;
    let (Ok(projects), Ok(posts)) = (projects, posts) else {
        return StatusCode::SERVICE_UNAVAILABLE.into_response();
    };

    let mut xml = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\" xmlns:image=\"http://www.google.com/schemas/sitemap-image/1.1\">\n",
    );
    // Several home-page sections have no stored timestamp. Do not claim that
    // a profile/project timestamp accounts for all of the home page's edits.
    sitemap_entry(
        &mut xml,
        &format!("{base_url}/"),
        None,
        Some(&format!("{base_url}/images/gideon-sigilai.png")),
    );
    let writing_modified = posts
        .iter()
        .filter_map(|(_, updated, _)| sitemap_modified(updated))
        .max();
    sitemap_entry(
        &mut xml,
        &format!("{base_url}/writing"),
        writing_modified,
        None,
    );
    for (slug, updated, image) in projects {
        let image_url = if image.is_empty() {
            format!("{base_url}/images/work/{slug}.svg")
        } else if image.starts_with("http://") || image.starts_with("https://") {
            image.clone()
        } else {
            format!("{base_url}{image}")
        };
        sitemap_entry(
            &mut xml,
            &format!("{base_url}/work/{slug}"),
            sitemap_modified(&updated),
            Some(&image_url),
        );
    }
    for (slug, updated, cover_image) in posts {
        let image_url = if cover_image.is_empty() {
            format!("{base_url}/images/writing/{slug}.svg")
        } else if cover_image.starts_with("http://") || cover_image.starts_with("https://") {
            cover_image.clone()
        } else {
            format!("{base_url}{cover_image}")
        };
        sitemap_entry(
            &mut xml,
            &format!("{base_url}/writing/{slug}"),
            sitemap_modified(&updated),
            Some(&image_url),
        );
    }
    xml.push_str("</urlset>\n");
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "application/xml; charset=utf-8")],
        xml,
    )
        .into_response()
}

fn sitemap_modified(value: &str) -> Option<chrono::DateTime<chrono::Utc>> {
    chrono::DateTime::parse_from_rfc3339(value)
        .ok()
        .map(|timestamp| timestamp.with_timezone(&chrono::Utc))
}

fn sitemap_entry(
    xml: &mut String,
    loc: &str,
    modified: Option<chrono::DateTime<chrono::Utc>>,
    image: Option<&str>,
) {
    xml.push_str("  <url>\n    <loc>");
    xml.push_str(&escape_html(loc));
    xml.push_str("</loc>\n");
    if let Some(modified) = modified {
        xml.push_str("    <lastmod>");
        xml.push_str(&modified.to_rfc3339_opts(chrono::SecondsFormat::Secs, true));
        xml.push_str("</lastmod>\n");
    }
    if let Some(image) = image {
        xml.push_str("    <image:image><image:loc>");
        xml.push_str(&escape_html(image));
        xml.push_str("</image:loc></image:image>\n");
    }
    xml.push_str("  </url>\n");
}

pub async fn robots(State(state): State<AppState>) -> Response {
    let base_url = base(&state);
    let text = format!(
        "User-agent: *\nAllow: /\nDisallow: /messages/\nDisallow: /health\n\nSitemap: {base_url}/sitemap.xml\n"
    );
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "text/plain; charset=utf-8")],
        text,
    )
        .into_response()
}

pub async fn manifest(State(state): State<AppState>) -> Response {
    let site = Site::load(&state.db).await;
    let manifest = json!({
        "name": format!("{} — {}", site.profile.name, site.profile.job_title),
        "short_name": short_name(&site.profile.name),
        "description": site.profile.description,
        "start_url": "/",
        "display": "standalone",
        "background_color": "#f6f7f1",
        "theme_color": "#f6f7f1",
        "lang": "en",
        "categories": ["portfolio", "technology"],
        "icons": [{ "src": "/static/icon.svg", "sizes": "any", "type": "image/svg+xml", "purpose": "any" }]
    });
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "application/manifest+json")],
        manifest.to_string(),
    )
        .into_response()
}

fn short_name(name: &str) -> String {
    let initials: String = name
        .split_whitespace()
        .filter_map(|word| word.chars().next())
        .take(3)
        .collect();
    if initials.is_empty() {
        name.chars().take(12).collect()
    } else {
        initials
    }
}

/// `fallback` for the portfolio and dashboard surfaces: a rendered 404 page.
pub async fn not_found_handler(State(state): State<AppState>) -> Response {
    not_found_response(&state)
        .unwrap_or_else(|_| (StatusCode::NOT_FOUND, "Not found").into_response())
}

/// Shared 404 for unknown slugs.
pub fn not_found_response(state: &AppState) -> ApiResult<Response> {
    let site = Site::placeholder();
    let body = render(
        crate::views::NOT_FOUND,
        &json!({
            "code": "404",
            "heading": r#"This page went <span class="serif">missing.</span>"#,
            "copy": "The link may be old or mistyped. Everything worth reading is one step away.",
        }),
    );
    let page = Page::new(
        page_title(&site, "Page not found"),
        "That page could not be found.",
        "/404",
    )
    .noindex();
    Ok((
        StatusCode::NOT_FOUND,
        Html(crate::views::render_page(&site, page, state, body)),
    )
        .into_response())
}

#[derive(Deserialize)]
pub struct TagQuery {
    pub tag: Option<String>,
}

/// JSON feed kept for tooling and future clients.
pub async fn json_posts(State(state): State<AppState>, Query(query): Query<TagQuery>) -> Response {
    let posts = match query
        .tag
        .as_deref()
        .map(str::trim)
        .filter(|tag| !tag.is_empty())
    {
        Some(tag) => published_by_tag(&state.db, tag, 50).await,
        None => crate::content::list_published(&state.db, 50).await,
    }
    .unwrap_or_default();

    let payload = json!({
        "posts": posts.iter().map(|post| json!({
            "id": post.id,
            "slug": post.slug,
            "title": post.title,
            "excerpt": post.excerpt,
            "tags": post.tags(),
            "reading_min": post.reading_min,
            "published_at": post.published_at,
            "updated_at": post.updated_at,
        })).collect::<Vec<_>>(),
        "tags": tag_counts(&state.db).await.into_iter()
            .map(|(tag, count)| json!({ "tag": tag, "count": count }))
            .collect::<Vec<_>>(),
    });
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "application/json")],
        payload.to_string(),
    )
        .into_response()
}

/// JSON detail for a single published post, for the Astro frontend.
pub async fn json_post(State(state): State<AppState>, Path(slug): Path<String>) -> Response {
    let Some(post) = published_by_slug(&state.db, &slug)
        .await
        .unwrap_or_default()
    else {
        return (
            StatusCode::NOT_FOUND,
            [(header::CONTENT_TYPE, "application/json")],
            r#"{"error":{"code":"not_found","message":"That post does not exist."}}"#.to_string(),
        )
            .into_response();
    };
    if !post.is_published() {
        return (
            StatusCode::NOT_FOUND,
            [(header::CONTENT_TYPE, "application/json")],
            r#"{"error":{"code":"not_found","message":"That post does not exist."}}"#.to_string(),
        )
            .into_response();
    }
    let more = related(&state.db, &post.id, 3).await;
    let article = normalise_headings(&post.body_html, &post.title);
    let tags = post.tags();
    let topic_chips = tags
        .iter()
        .map(|tag| format!("<span class=\"chip\">{}</span>", escape_html(tag)))
        .collect::<Vec<_>>()
        .join("");
    let payload = json!({
        "post": {
            "slug": post.slug,
            "title": post.title,
            "excerpt": post.excerpt,
            "reading_min": post.reading_min,
            "cover_image": post.cover_image,
            "image_alt": post_image_alt(&post),
            "published_at": post.published_at.clone().unwrap_or_default(),
            "updated_at": post.updated_at,
            "tags": tags,
        },
        "date": post.published_at.as_deref().map(date_label).unwrap_or_default(),
        "article": article,
        "topic_chips": topic_chips,
        "author": "", // filled by client from /api/site if needed
        "more": more.iter().map(|item| json!({
            "slug": item.slug,
            "title": item.title,
        })).collect::<Vec<_>>(),
    });
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "application/json")],
        payload.to_string(),
    )
        .into_response()
}
