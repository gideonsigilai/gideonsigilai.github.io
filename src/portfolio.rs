//! Portfolio content: profile, socials, projects, toolkit, principles, and
//! milestones. Everything lives in SQLite and is edited from `/admin/content`,
//! so a content change never needs a rebuild.

use serde_json::{json, Value};
use sqlx::{Column, FromRow, Row, SqlitePool};

use crate::template::escape_html;
use crate::text::{clean_text, looks_like_email, slugify};

// ---------------------------------------------------------------- rows

#[derive(Debug, Clone, FromRow)]
pub struct ProfileRow {
    pub name: String,
    pub job_title: String,
    pub description: String,
    pub location: String,
    pub email: String,
    pub availability: String,
    pub hero_lead: String,
    pub hero_focus: String,
    pub hero_focus_detail: String,
    pub about_intro: String,
}

impl ProfileRow {
    /// Used only when the singleton row is missing (a hand-edited database).
    /// Keeps the site rendering instead of failing every request.
    pub fn fallback() -> Self {
        Self {
            name: "Portfolio".into(),
            job_title: "Software Engineer".into(),
            description: "Software engineer building thoughtful products.".into(),
            location: String::new(),
            email: String::new(),
            availability: "Available for new work".into(),
            hero_lead: String::new(),
            hero_focus: String::new(),
            hero_focus_detail: String::new(),
            about_intro: String::new(),
        }
    }
}

#[derive(Debug, Clone, FromRow)]
pub struct SocialRow {
    pub label: String,
    pub handle: String,
    pub href: String,
    pub icon: String,
}

#[derive(Debug, Clone, FromRow)]
pub struct CategoryRow {
    pub id: String,
    pub label: String,
}

#[derive(Debug, Clone, FromRow)]
pub struct ProjectRow {
    pub slug: String,
    pub name: String,
    pub category: String,
    pub category_label: String,
    pub kicker: String,
    pub tagline: String,
    pub summary: String,
    pub year: String,
    pub role: String,
    pub status: String,
    pub accent: String,
    pub stack: String,
    pub features: String,
    pub story: String,
    pub outcome: String,
    pub link_label: String,
    pub link_href: String,
    pub image: String,
}

#[derive(Debug, Clone, FromRow)]
pub struct ToolGroupRow {
    pub id: String,
    pub number: String,
    pub title: String,
    pub subtitle: String,
}

#[derive(Debug, Clone, FromRow)]
pub struct ToolRow {
    pub name: String,
    pub group_id: String,
    pub description: String,
    pub project_slug: String,
}

#[derive(Debug, Clone, FromRow)]
pub struct PrincipleRow {
    pub title: String,
    pub body: String,
}

#[derive(Debug, Clone, FromRow)]
pub struct MilestoneRow {
    pub marker: String,
    pub title: String,
    pub detail: String,
}

fn json_strings(raw: &str) -> Vec<String> {
    serde_json::from_str::<Vec<String>>(raw).unwrap_or_default()
}

fn json_pairs(raw: &str) -> Vec<(String, String)> {
    serde_json::from_str::<Vec<Value>>(raw)
        .unwrap_or_default()
        .into_iter()
        .map(|item| {
            (
                item["title"].as_str().unwrap_or_default().to_string(),
                item["detail"].as_str().unwrap_or_default().to_string(),
            )
        })
        .collect()
}

impl ProjectRow {
    pub fn stack(&self) -> Vec<String> {
        json_strings(&self.stack)
    }

    pub fn story(&self) -> Vec<String> {
        json_strings(&self.story)
    }

    pub fn outcome(&self) -> Vec<String> {
        json_strings(&self.outcome)
    }

    pub fn features(&self) -> Vec<(String, String)> {
        json_pairs(&self.features)
    }

    /// Card shape used by the home page grid and `/api/projects`.
    pub fn card(&self) -> Value {
        json!({
            "slug": self.slug,
            "name": self.name,
            "category": self.category,
            "kicker": self.kicker,
            "tagline": self.tagline,
            "summary": self.summary,
            "year": self.year,
            "status": self.status,
            "accent": self.accent,
            "image": self.image,
            "stack": self.stack(),
        })
    }

    /// Full case-study shape used by `/work/{slug}` and `/api/projects/{slug}`.
    pub fn detail(&self) -> Value {
        let mut value = self.card();
        let object = value.as_object_mut().expect("card is an object");
        object.insert("category_label".into(), json!(self.category_label));
        object.insert("role".into(), json!(self.role));
        object.insert(
            "features".into(),
            json!(self
                .features()
                .into_iter()
                .map(|(title, detail)| json!({ "title": title, "detail": detail }))
                .collect::<Vec<_>>()),
        );
        object.insert("story".into(), json!(self.story()));
        object.insert("outcome".into(), json!(self.outcome()));
        object.insert(
            "link".into(),
            if self.link_href.is_empty() {
                Value::Null
            } else {
                json!({ "label": self.link_label, "href": self.link_href })
            },
        );
        object.insert("stack_line".into(), json!(self.stack().join(" · ")));
        value
    }
}

// ---------------------------------------------------------------- reads

pub async fn profile(db: &SqlitePool) -> Option<ProfileRow> {
    sqlx::query_as(
        "SELECT name, job_title, description, location, email, availability,
                hero_lead, hero_focus, hero_focus_detail, about_intro
         FROM profile WHERE id = 1",
    )
    .fetch_optional(db)
    .await
    .ok()
    .flatten()
}

pub async fn socials(db: &SqlitePool) -> Vec<SocialRow> {
    sqlx::query_as("SELECT id, label, handle, href, icon FROM social_links ORDER BY position, id")
        .fetch_all(db)
        .await
        .unwrap_or_default()
}

pub async fn categories(db: &SqlitePool) -> Vec<CategoryRow> {
    sqlx::query_as("SELECT id, label FROM project_categories ORDER BY position, id")
        .fetch_all(db)
        .await
        .unwrap_or_default()
}

pub async fn projects(db: &SqlitePool) -> Vec<ProjectRow> {
    sqlx::query_as("SELECT * FROM projects ORDER BY position, created_at")
        .fetch_all(db)
        .await
        .unwrap_or_default()
}

pub async fn visible_projects(db: &SqlitePool) -> Vec<ProjectRow> {
    sqlx::query_as("SELECT * FROM projects WHERE visible = 1 ORDER BY position, created_at")
        .fetch_all(db)
        .await
        .unwrap_or_default()
}

pub async fn visible_project(db: &SqlitePool, slug: &str) -> Option<ProjectRow> {
    sqlx::query_as("SELECT * FROM projects WHERE slug = ? AND visible = 1")
        .bind(slug)
        .fetch_optional(db)
        .await
        .ok()
        .flatten()
}

pub async fn tool_groups(db: &SqlitePool) -> Vec<ToolGroupRow> {
    sqlx::query_as("SELECT id, number, title, subtitle FROM tool_groups ORDER BY position, id")
        .fetch_all(db)
        .await
        .unwrap_or_default()
}

pub async fn tools(db: &SqlitePool) -> Vec<ToolRow> {
    sqlx::query_as(
        "SELECT id, name, group_id, description, project_slug FROM tools ORDER BY position, id",
    )
    .fetch_all(db)
    .await
    .unwrap_or_default()
}

pub async fn principles(db: &SqlitePool) -> Vec<PrincipleRow> {
    sqlx::query_as("SELECT id, title, body FROM principles ORDER BY position, id")
        .fetch_all(db)
        .await
        .unwrap_or_default()
}

pub async fn milestones(db: &SqlitePool) -> Vec<MilestoneRow> {
    sqlx::query_as("SELECT id, marker, title, detail FROM milestones ORDER BY position, id")
        .fetch_all(db)
        .await
        .unwrap_or_default()
}

// ---------------------------------------------------------------- field specs

/// How a stored column maps onto an admin input.
#[derive(Clone, Copy)]
pub enum Kind {
    Line {
        max: usize,
    },
    Paragraph {
        max: usize,
        rows: u8,
    },
    Slug {
        max: usize,
    },
    Choice(&'static [&'static str]),
    Lookup(Lookup),
    Toggle,
    /// One item per line, stored as a JSON array of strings.
    Strings {
        max: usize,
    },
    /// `Title :: Detail` per line, stored as a JSON array of objects.
    Pairs {
        max: usize,
    },
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Lookup {
    Categories,
    Groups,
    Projects,
}

#[derive(Clone, Copy)]
pub enum IdKind {
    /// A generated UUID.
    Uuid,
    /// A slug derived from the named column, stable for the row's lifetime.
    Slug(&'static str),
}

pub struct Field {
    pub column: &'static str,
    pub label: &'static str,
    pub kind: Kind,
    pub required: bool,
    pub hint: &'static str,
}

pub struct Collection {
    /// URL segment, e.g. `projects`.
    pub slug: &'static str,
    pub table: &'static str,
    pub title: &'static str,
    pub single: &'static str,
    pub blurb: &'static str,
    /// Column shown as the headline in the admin list.
    pub summary_column: &'static str,
    /// Column shown as the muted second line.
    pub detail_column: &'static str,
    pub fields: &'static [Field],
    pub id: IdKind,
    /// Moving a row is limited to rows sharing this column's value.
    pub scope_column: Option<&'static str>,
}

const PROJECT_FIELDS: &[Field] = &[
    Field { column: "name", label: "Name", kind: Kind::Line { max: 80 }, required: true, hint: "" },
    Field { column: "slug", label: "Slug", kind: Kind::Slug { max: 80 }, required: false, hint: "Used in /work/{slug}. Derived from the name when blank." },
    Field { column: "summary", label: "Summary", kind: Kind::Line { max: 160 }, required: true, hint: "One line under the project name." },
    Field { column: "tagline", label: "Tagline", kind: Kind::Line { max: 60 }, required: false, hint: "Large type on the card artwork." },
    Field { column: "category", label: "Filter", kind: Kind::Lookup(Lookup::Categories), required: true, hint: "Which filter button shows this project." },
    Field { column: "category_label", label: "Category label", kind: Kind::Line { max: 80 }, required: false, hint: "Shown above the case-study title." },
    Field { column: "kicker", label: "Kicker", kind: Kind::Line { max: 60 }, required: false, hint: "Small caps over the artwork." },
    Field { column: "year", label: "Timeline", kind: Kind::Line { max: 40 }, required: false, hint: "" },
    Field { column: "role", label: "Role", kind: Kind::Line { max: 100 }, required: false, hint: "" },
    Field { column: "status", label: "Status", kind: Kind::Line { max: 60 }, required: false, hint: "" },
    Field { column: "accent", label: "Accent", kind: Kind::Choice(&["lime", "clay", "sky"]), required: true, hint: "" },
    Field { column: "stack", label: "Stack", kind: Kind::Strings { max: 40 }, required: false, hint: "One technology per line." },
    Field { column: "features", label: "What's inside", kind: Kind::Pairs { max: 20 }, required: false, hint: "One per line, as \"Title :: Detail\"." },
    Field { column: "story", label: "The problem", kind: Kind::Strings { max: 20 }, required: false, hint: "One paragraph per line." },
    Field { column: "outcome", label: "Outcomes", kind: Kind::Strings { max: 20 }, required: false, hint: "One result per line." },
    Field { column: "link_label", label: "Link label", kind: Kind::Line { max: 60 }, required: false, hint: "Only used when a link URL is set." },
    Field { column: "link_href", label: "Link URL", kind: Kind::Line { max: 300 }, required: false, hint: "Optional, e.g. https://…" },
    Field { column: "image", label: "Cover image", kind: Kind::Line { max: 300 }, required: false, hint: "Optional image path/URL, e.g. /images/plant_doc.jpg. Leave blank for generated artwork." },
    Field { column: "visible", label: "Visible on the site", kind: Kind::Toggle, required: false, hint: "" },
];

const TOOL_GROUP_FIELDS: &[Field] = &[
    Field {
        column: "title",
        label: "Title",
        kind: Kind::Line { max: 80 },
        required: true,
        hint: "",
    },
    Field {
        column: "number",
        label: "Number",
        kind: Kind::Line { max: 8 },
        required: false,
        hint: "Order label such as 01.",
    },
    Field {
        column: "subtitle",
        label: "Subtitle",
        kind: Kind::Line { max: 80 },
        required: false,
        hint: "",
    },
];

const TOOL_FIELDS: &[Field] = &[
    Field {
        column: "name",
        label: "Name",
        kind: Kind::Line { max: 40 },
        required: true,
        hint: "",
    },
    Field {
        column: "group_id",
        label: "Group",
        kind: Kind::Lookup(Lookup::Groups),
        required: true,
        hint: "",
    },
    Field {
        column: "description",
        label: "Description",
        kind: Kind::Paragraph { max: 300, rows: 4 },
        required: false,
        hint: "Shown in the toolkit panel when the chip is selected.",
    },
    Field {
        column: "project_slug",
        label: "Related project",
        kind: Kind::Lookup(Lookup::Projects),
        required: false,
        hint: "Optional link to a case study.",
    },
];

const PRINCIPLE_FIELDS: &[Field] = &[
    Field {
        column: "title",
        label: "Title",
        kind: Kind::Line { max: 60 },
        required: true,
        hint: "",
    },
    Field {
        column: "body",
        label: "Body",
        kind: Kind::Paragraph { max: 300, rows: 4 },
        required: true,
        hint: "",
    },
];

const MILESTONE_FIELDS: &[Field] = &[
    Field {
        column: "title",
        label: "Title",
        kind: Kind::Line { max: 90 },
        required: true,
        hint: "",
    },
    Field {
        column: "marker",
        label: "Marker",
        kind: Kind::Line { max: 12 },
        required: false,
        hint: "Short label such as 2024 or Now.",
    },
    Field {
        column: "detail",
        label: "Detail",
        kind: Kind::Line { max: 120 },
        required: false,
        hint: "",
    },
];

const SOCIAL_FIELDS: &[Field] = &[
    Field {
        column: "label",
        label: "Label",
        kind: Kind::Line { max: 40 },
        required: true,
        hint: "",
    },
    Field {
        column: "handle",
        label: "Handle",
        kind: Kind::Line { max: 60 },
        required: false,
        hint: "",
    },
    Field {
        column: "href",
        label: "URL",
        kind: Kind::Line { max: 300 },
        required: true,
        hint: "https://… or mailto:…",
    },
    Field {
        column: "icon",
        label: "Icon",
        kind: Kind::Choice(&["github", "x", "linkedin", "mail", "model", "link"]),
        required: true,
        hint: "",
    },
];

const CATEGORY_FIELDS: &[Field] = &[Field {
    column: "label",
    label: "Label",
    kind: Kind::Line { max: 40 },
    required: true,
    hint: "Shown on the filter button.",
}];

pub const COLLECTIONS: &[Collection] = &[
    Collection {
        slug: "projects",
        table: "projects",
        title: "Projects",
        single: "project",
        blurb: "Case studies: the home page grid and every /work page.",
        summary_column: "name",
        detail_column: "slug",
        fields: PROJECT_FIELDS,
        id: IdKind::Uuid,
        scope_column: None,
    },
    Collection {
        slug: "tool-groups",
        table: "tool_groups",
        title: "Toolkit groups",
        single: "group",
        blurb: "The three (or more) headings in the toolkit section.",
        summary_column: "title",
        detail_column: "subtitle",
        fields: TOOL_GROUP_FIELDS,
        id: IdKind::Slug("title"),
        scope_column: None,
    },
    Collection {
        slug: "tools",
        table: "tools",
        title: "Tools",
        single: "tool",
        blurb: "Chips in the toolkit, described on demand by /api/toolkit.",
        summary_column: "name",
        detail_column: "group_id",
        fields: TOOL_FIELDS,
        id: IdKind::Uuid,
        scope_column: Some("group_id"),
    },
    Collection {
        slug: "principles",
        table: "principles",
        title: "Principles",
        single: "principle",
        blurb: "The short list inside the About section.",
        summary_column: "title",
        detail_column: "body",
        fields: PRINCIPLE_FIELDS,
        id: IdKind::Uuid,
        scope_column: None,
    },
    Collection {
        slug: "milestones",
        table: "milestones",
        title: "Milestones",
        single: "milestone",
        blurb: "Dated markers under the About section.",
        summary_column: "title",
        detail_column: "marker",
        fields: MILESTONE_FIELDS,
        id: IdKind::Uuid,
        scope_column: None,
    },
    Collection {
        slug: "categories",
        table: "project_categories",
        title: "Project filters",
        single: "filter",
        blurb: "Filter buttons above the project grid.",
        summary_column: "label",
        detail_column: "id",
        fields: CATEGORY_FIELDS,
        id: IdKind::Slug("label"),
        scope_column: None,
    },
    Collection {
        slug: "socials",
        table: "social_links",
        title: "Social links",
        single: "link",
        blurb: "Footer, hero, and case-study sidebars.",
        summary_column: "label",
        detail_column: "handle",
        fields: SOCIAL_FIELDS,
        id: IdKind::Uuid,
        scope_column: None,
    },
];

pub fn collection(slug: &str) -> Option<&'static Collection> {
    COLLECTIONS.iter().find(|item| item.slug == slug)
}

// ---------------------------------------------------------------- admin forms

/// Choices for `Lookup` fields, loaded once per admin request.
#[derive(Default)]
pub struct LookupOptions {
    pub categories: Vec<(String, String)>,
    pub groups: Vec<(String, String)>,
    pub projects: Vec<(String, String)>,
}

impl LookupOptions {
    pub async fn load(db: &SqlitePool) -> Self {
        Self {
            categories: categories(db)
                .await
                .into_iter()
                .map(|row| (row.id, row.label))
                .collect(),
            groups: tool_groups(db)
                .await
                .into_iter()
                .map(|row| (row.id, row.title))
                .collect(),
            projects: projects(db)
                .await
                .into_iter()
                .map(|row| (row.slug, row.name))
                .collect(),
        }
    }

    fn choices(&self, lookup: Lookup) -> &[(String, String)] {
        match lookup {
            Lookup::Categories => &self.categories,
            Lookup::Groups => &self.groups,
            Lookup::Projects => &self.projects,
        }
    }
}

fn is_block(field: &Field) -> bool {
    matches!(
        field.kind,
        Kind::Paragraph { .. } | Kind::Strings { .. } | Kind::Pairs { .. }
    )
}

/// The form value of a stored column, ready to drop into an input.
pub fn display_value(field: &Field, raw: &str) -> String {
    match field.kind {
        Kind::Strings { .. } => json_strings(raw).join("\n"),
        Kind::Pairs { .. } => json_pairs(raw)
            .into_iter()
            .map(|(title, detail)| format!("{title} :: {detail}"))
            .collect::<Vec<_>>()
            .join("\n"),
        Kind::Toggle => (raw == "1").to_string(),
        _ => raw.to_string(),
    }
}

/// Validates one submitted value and returns the text stored in SQLite.
pub fn parse_value(field: &Field, raw: &str, options: &LookupOptions) -> Result<String, String> {
    let trimmed = raw.trim();
    if field.required && trimmed.is_empty() {
        return Err(format!("{} is required.", field.label));
    }
    let within = |value: &str, max: usize| -> Result<(), String> {
        if value.chars().count() > max {
            Err(format!("{} must be at most {max} characters.", field.label))
        } else {
            Ok(())
        }
    };

    match field.kind {
        Kind::Line { max } => {
            within(trimmed, max)?;
            Ok(trimmed.replace(['\r', '\n'], " "))
        }
        Kind::Paragraph { max, .. } => {
            within(trimmed, max)?;
            Ok(trimmed.to_string())
        }
        Kind::Slug { max } => {
            if trimmed.is_empty() {
                return Ok(String::new());
            }
            let slug = slugify(trimmed);
            if slug.is_empty() || slug.len() > max {
                return Err(format!(
                    "{} must be a single URL-safe word up to {max} characters.",
                    field.label
                ));
            }
            Ok(slug)
        }
        Kind::Choice(allowed) => {
            if trimmed.is_empty() {
                return Ok(allowed.first().copied().unwrap_or_default().to_string());
            }
            if allowed.contains(&trimmed) {
                Ok(trimmed.to_string())
            } else {
                Err(format!(
                    "{}: choose one of {}.",
                    field.label,
                    allowed.join(", ")
                ))
            }
        }
        Kind::Lookup(lookup) => {
            if trimmed.is_empty() {
                if field.required {
                    return Err(format!("{} is required.", field.label));
                }
                return Ok(String::new());
            }
            if options.choices(lookup).iter().any(|(id, _)| id == trimmed) {
                Ok(trimmed.to_string())
            } else {
                Err(format!("{}: that option no longer exists.", field.label))
            }
        }
        Kind::Toggle => Ok(if trimmed.is_empty() { "0" } else { "1" }.to_string()),
        Kind::Strings { max } => {
            let items: Vec<String> = raw
                .lines()
                .map(str::trim)
                .filter(|line| !line.is_empty())
                .take(max)
                .map(|line| line.chars().take(600).collect())
                .collect();
            Ok(serde_json::to_string(&items).unwrap_or_else(|_| "[]".into()))
        }
        Kind::Pairs { max } => {
            let items: Vec<Value> = raw
                .lines()
                .map(str::trim)
                .filter(|line| !line.is_empty())
                .take(max)
                .map(|line| {
                    let (title, detail) = line.split_once("::").unwrap_or((line, ""));
                    json!({
                        "title": title.trim().chars().take(120).collect::<String>(),
                        "detail": detail.trim().chars().take(400).collect::<String>(),
                    })
                })
                .collect();
            Ok(serde_json::to_string(&items).unwrap_or_else(|_| "[]".into()))
        }
    }
}

/// Renders every input for a collection editor. Returns escaped HTML.
pub fn fields_html(
    collection: &Collection,
    current: &[(String, String)],
    options: &LookupOptions,
) -> String {
    let value_of = |column: &str| {
        current
            .iter()
            .find(|(key, _)| key == column)
            .map(|(_, value)| value.as_str())
            .unwrap_or_default()
    };

    let mut out = String::new();
    let mut index = 0;
    while index < collection.fields.len() {
        let field = &collection.fields[index];
        // Two short fields share a row; long prose and toggles take the width.
        let next = collection.fields.get(index + 1).filter(|next| {
            !is_block(field) && !is_block(next) && !matches!(field.kind, Kind::Toggle)
        });

        match next {
            Some(next) => {
                out.push_str(r#"<div class="editor-row">"#);
                out.push_str(&field_html(field, value_of(field.column), options));
                out.push_str(&field_html(next, value_of(next.column), options));
                out.push_str("</div>");
                index += 2;
            }
            None => {
                out.push_str(&field_html(field, value_of(field.column), options));
                index += 1;
            }
        }
    }
    out
}

fn field_html(field: &Field, raw: &str, options: &LookupOptions) -> String {
    let value = display_value(field, raw);
    let id = format!("field-{}", field.column.replace('_', "-"));
    let required = if field.required { " required" } else { "" };
    let hint = if field.hint.is_empty() {
        String::new()
    } else {
        format!(r#"<span class="hint">{}</span>"#, escape_html(field.hint))
    };

    if matches!(field.kind, Kind::Toggle) {
        return format!(
            r#"<div class="field"><label class="toggle-row"><input id="{id}" type="checkbox" name="{}" value="1"{}> <span>{}</span></label>{hint}</div>"#,
            field.column,
            if value == "true" { " checked" } else { "" },
            escape_html(field.label),
        );
    }

    let control = match field.kind {
        Kind::Line { max } => format!(
            r#"<input id="{id}" name="{}" maxlength="{max}" value="{}"{required}>"#,
            field.column,
            escape_html(&value),
        ),
        Kind::Paragraph { max, rows } => format!(
            r#"<textarea id="{id}" name="{}" rows="{rows}" maxlength="{max}"{required}>{}</textarea>"#,
            field.column,
            escape_html(&value),
        ),
        Kind::Slug { max } => format!(
            r#"<input id="{id}" name="{}" maxlength="{max}" value="{}" placeholder="auto from the title"{required}>"#,
            field.column,
            escape_html(&value),
        ),
        Kind::Choice(allowed) => {
            let options_html = allowed
                .iter()
                .map(|option| {
                    format!(
                        r#"<option value="{}"{}>{}</option>"#,
                        escape_html(option),
                        if *option == value { " selected" } else { "" },
                        escape_html(option),
                    )
                })
                .collect::<String>();
            format!(
                r#"<select id="{id}" name="{}"{required}>{options_html}</select>"#,
                field.column
            )
        }
        Kind::Lookup(lookup) => {
            let choices = options.choices(lookup);
            let mut options_html = String::new();
            if !field.required {
                options_html.push_str(r#"<option value="">— none —</option>"#);
            }
            for (option_id, label) in choices {
                options_html.push_str(&format!(
                    r#"<option value="{}"{}>{}</option>"#,
                    escape_html(option_id),
                    if *option_id == value { " selected" } else { "" },
                    escape_html(label),
                ));
            }
            if choices.is_empty() {
                options_html.push_str(r#"<option value="">Nothing to choose yet</option>"#);
            }
            format!(
                r#"<select id="{id}" name="{}"{required}>{options_html}</select>"#,
                field.column
            )
        }
        Kind::Toggle => unreachable!("handled above"),
        Kind::Strings { .. } | Kind::Pairs { .. } => format!(
            r#"<textarea id="{id}" name="{}" rows="6">{}</textarea>"#,
            field.column,
            escape_html(&value),
        ),
    };

    format!(
        r#"<div class="field"><label for="{id}">{}</label>{control}{hint}</div>"#,
        escape_html(field.label)
    )
}

/// Validates every submitted field. On failure returns the first message.
pub fn parse_form(
    collection: &Collection,
    form: &std::collections::HashMap<String, String>,
    options: &LookupOptions,
) -> Result<Vec<(String, String)>, String> {
    let mut values: Vec<(String, String)> = Vec::with_capacity(collection.fields.len());
    for field in collection.fields {
        let raw = form.get(field.column).cloned().unwrap_or_default();
        values.push((field.column.to_string(), parse_value(field, &raw, options)?));
    }

    // Derive a blank slug from the row's headline column.
    if let Some(field) = collection
        .fields
        .iter()
        .find(|field| matches!(field.kind, Kind::Slug { .. }))
    {
        let source = match collection.id {
            IdKind::Slug(column) => column,
            IdKind::Uuid => collection.summary_column,
        };
        let seed = values
            .iter()
            .find(|(column, _)| *column == field.column)
            .map(|(_, value)| value.clone())
            .unwrap_or_default();
        let derived = if seed.is_empty() {
            let headline = values
                .iter()
                .find(|(column, _)| column == source)
                .map(|(_, value)| value.clone())
                .unwrap_or_default();
            slugify(&headline)
        } else {
            seed
        };
        if derived.is_empty() {
            return Err(format!("{} is required.", field.label));
        }
        if let Some((_, value)) = values
            .iter_mut()
            .find(|(column, _)| *column == field.column)
        {
            *value = derived;
        }
    }

    Ok(values)
}

// ---------------------------------------------------------------- writes

pub fn row_id(collection: &Collection, values: &[(String, String)]) -> String {
    match collection.id {
        IdKind::Uuid => uuid::Uuid::new_v4().to_string(),
        IdKind::Slug(column) => {
            let seed = values
                .iter()
                .find(|(key, _)| key == column)
                .map(|(_, value)| value.as_str())
                .unwrap_or_default();
            let slug = slugify(seed);
            if slug.is_empty() {
                uuid::Uuid::new_v4().to_string()
            } else {
                slug
            }
        }
    }
}

/// Inserts a row. `explicit_id` is used by seed data, which pins identifiers so
/// relationships (tools → groups, projects → filters) stay stable.
pub async fn insert_row_with_id(
    db: &SqlitePool,
    collection: &Collection,
    explicit_id: Option<&str>,
    values: &[(String, String)],
) -> sqlx::Result<String> {
    let id = match explicit_id {
        Some(id) => id.to_string(),
        None => row_id(collection, values),
    };
    let position = next_position(db, collection, values)
        .await
        .unwrap_or_default();
    let timestamped = collection.table == "projects";

    let mut columns: Vec<&str> = vec!["id", "position"];
    columns.extend(values.iter().map(|(column, _)| column.as_str()));
    if timestamped {
        columns.extend(["created_at", "updated_at"]);
    }

    let sql = format!(
        "INSERT INTO {} ({}) VALUES ({})",
        collection.table,
        columns.join(", "),
        vec!["?"; columns.len()].join(", ")
    );

    let now = crate::content::now_iso();
    let mut query = sqlx::query(&sql).bind(&id).bind(position);
    for (_, value) in values {
        query = query.bind(value);
    }
    if timestamped {
        query = query.bind(&now).bind(&now);
    }
    query.execute(db).await?;
    Ok(id)
}

pub async fn update_row(
    db: &SqlitePool,
    collection: &Collection,
    id: &str,
    values: &[(String, String)],
) -> sqlx::Result<bool> {
    if values.is_empty() {
        return Ok(false);
    }
    let assignments = values
        .iter()
        .map(|(column, _)| format!("{column} = ?"))
        .collect::<Vec<_>>()
        .join(", ");
    let sql = if collection.table == "projects" {
        format!(
            "UPDATE {} SET {assignments}, updated_at = ? WHERE id = ?",
            collection.table
        )
    } else {
        format!("UPDATE {} SET {assignments} WHERE id = ?", collection.table)
    };

    let mut query = sqlx::query(&sql);
    for (_, value) in values {
        query = query.bind(value);
    }
    if collection.table == "projects" {
        query = query.bind(crate::content::now_iso());
    }
    let result = query.bind(id).execute(db).await?;
    Ok(result.rows_affected() > 0)
}

pub async fn delete_row(db: &SqlitePool, table: &str, id: &str) -> sqlx::Result<bool> {
    let sql = format!("DELETE FROM {table} WHERE id = ?");
    let result = sqlx::query(&sql).bind(id).execute(db).await?;
    Ok(result.rows_affected() > 0)
}

pub async fn row_values(db: &SqlitePool, table: &str, id: &str) -> Option<Vec<(String, String)>> {
    let sql = format!("SELECT * FROM {table} WHERE id = ?");
    let row = sqlx::query(&sql).bind(id).fetch_optional(db).await.ok()??;
    let mut values = Vec::with_capacity(row.columns().len());
    for column in row.columns() {
        let name = column.name().to_string();
        let text = row
            .try_get::<String, _>(name.as_str())
            .or_else(|_| {
                row.try_get::<i64, _>(name.as_str())
                    .map(|number| number.to_string())
            })
            .unwrap_or_default();
        values.push((name, text));
    }
    Some(values)
}

async fn next_position(
    db: &SqlitePool,
    collection: &Collection,
    values: &[(String, String)],
) -> sqlx::Result<i64> {
    let scope = collection.scope_column.and_then(|column| {
        values
            .iter()
            .find(|(key, _)| key == column)
            .map(|(_, value)| value.clone())
    });
    let sql = match (&scope, collection.scope_column) {
        (Some(_), Some(column)) => format!(
            "SELECT COALESCE(MAX(position), -1) + 1 FROM {} WHERE {column} = ?",
            collection.table
        ),
        _ => format!(
            "SELECT COALESCE(MAX(position), -1) + 1 FROM {}",
            collection.table
        ),
    };
    let mut query = sqlx::query_scalar::<_, i64>(&sql);
    if let Some(value) = &scope {
        query = query.bind(value);
    }
    query.fetch_one(db).await
}

/// Moves a row one slot up or down and rewrites dense positions for the scope.
pub async fn move_row(
    db: &SqlitePool,
    collection: &Collection,
    id: &str,
    delta: i64,
) -> sqlx::Result<bool> {
    let sql = match collection.scope_column {
        Some(column) => format!(
            "SELECT id FROM {} WHERE {column} = (SELECT {column} FROM {} WHERE id = ?) ORDER BY position, id",
            collection.table, collection.table
        ),
        None => format!("SELECT id FROM {} ORDER BY position, id", collection.table),
    };
    let mut query = sqlx::query_scalar::<_, String>(&sql);
    if collection.scope_column.is_some() {
        query = query.bind(id);
    }
    let mut ids = query.fetch_all(db).await?;

    let Some(index) = ids.iter().position(|row_id| row_id == id) else {
        return Ok(false);
    };
    let target = index as i64 + delta;
    if target < 0 || target >= ids.len() as i64 {
        return Ok(false);
    }
    ids.swap(index, target as usize);

    let sql = format!("UPDATE {} SET position = ? WHERE id = ?", collection.table);
    let mut tx = db.begin().await?;
    for (position, row_id) in ids.iter().enumerate() {
        sqlx::query(&sql)
            .bind(position as i64)
            .bind(row_id)
            .execute(&mut *tx)
            .await?;
    }
    tx.commit().await?;
    Ok(true)
}

pub struct AdminRow {
    pub id: String,
    pub title: String,
    pub detail: String,
}

pub async fn admin_rows(db: &SqlitePool, collection: &Collection) -> Vec<AdminRow> {
    let sql = format!(
        "SELECT id, {} AS title, {} AS detail FROM {} ORDER BY position, id",
        collection.summary_column, collection.detail_column, collection.table
    );
    let rows = sqlx::query(&sql).fetch_all(db).await.unwrap_or_default();
    rows.into_iter()
        .map(|row| AdminRow {
            id: row.try_get("id").unwrap_or_default(),
            title: row.try_get("title").unwrap_or_default(),
            detail: row.try_get("detail").unwrap_or_default(),
        })
        .collect()
}

// ---------------------------------------------------------------- profile

pub fn profile_values(profile: &ProfileRow) -> Vec<(String, String)> {
    vec![
        ("name".into(), profile.name.clone()),
        ("job_title".into(), profile.job_title.clone()),
        ("description".into(), profile.description.clone()),
        ("location".into(), profile.location.clone()),
        ("email".into(), profile.email.clone()),
        ("availability".into(), profile.availability.clone()),
        ("hero_lead".into(), profile.hero_lead.clone()),
        ("hero_focus".into(), profile.hero_focus.clone()),
        (
            "hero_focus_detail".into(),
            profile.hero_focus_detail.clone(),
        ),
        ("about_intro".into(), profile.about_intro.clone()),
    ]
}

pub const PROFILE_FIELDS: &[Field] = &[
    Field {
        column: "name",
        label: "Name",
        kind: Kind::Line { max: 80 },
        required: true,
        hint: "Brand, header, and structured data.",
    },
    Field {
        column: "job_title",
        label: "Job title",
        kind: Kind::Line { max: 80 },
        required: true,
        hint: "",
    },
    Field {
        column: "location",
        label: "Location",
        kind: Kind::Line { max: 80 },
        required: true,
        hint: "",
    },
    Field {
        column: "email",
        label: "Email",
        kind: Kind::Line { max: 254 },
        required: true,
        hint: "Used by the contact links.",
    },
    Field {
        column: "availability",
        label: "Availability",
        kind: Kind::Line { max: 80 },
        required: false,
        hint: "Hero badge, e.g. Available for new work.",
    },
    Field {
        column: "description",
        label: "Meta description",
        kind: Kind::Paragraph { max: 400, rows: 3 },
        required: true,
        hint: "Search and share previews.",
    },
    Field {
        column: "hero_lead",
        label: "Hero lead",
        kind: Kind::Paragraph { max: 400, rows: 3 },
        required: true,
        hint: "First paragraph on the home page.",
    },
    Field {
        column: "hero_focus",
        label: "Focus",
        kind: Kind::Line { max: 80 },
        required: false,
        hint: "",
    },
    Field {
        column: "hero_focus_detail",
        label: "Focus detail",
        kind: Kind::Paragraph { max: 300, rows: 2 },
        required: false,
        hint: "",
    },
    Field {
        column: "about_intro",
        label: "About paragraph",
        kind: Kind::Paragraph { max: 800, rows: 4 },
        required: true,
        hint: "Opening of the About section.",
    },
];

pub fn parse_profile(
    form: &std::collections::HashMap<String, String>,
) -> Result<ProfileRow, String> {
    let take = |column: &str, min: usize, max: usize, label: &str, required: bool| {
        let raw = form.get(column).cloned().unwrap_or_default();
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return if required {
                Err(format!("{label} is required."))
            } else {
                Ok(String::new())
            };
        }
        clean_text(trimmed, min, max, label).map_err(|error| error.message)
    };

    let email = take("email", 5, 254, "Email", true)?;
    if !looks_like_email(&email) {
        return Err("That email address does not look right.".into());
    }

    Ok(ProfileRow {
        name: take("name", 2, 80, "Name", true)?,
        job_title: take("job_title", 2, 80, "Job title", true)?,
        description: take("description", 20, 400, "Meta description", true)?,
        location: take("location", 2, 80, "Location", true)?,
        email,
        availability: take("availability", 2, 80, "Availability", false)?,
        hero_lead: take("hero_lead", 10, 400, "Hero lead", true)?,
        hero_focus: take("hero_focus", 2, 80, "Focus", false)?,
        hero_focus_detail: take("hero_focus_detail", 5, 300, "Focus detail", false)?,
        about_intro: take("about_intro", 20, 800, "About paragraph", true)?,
    })
}

pub async fn save_profile(db: &SqlitePool, profile: &ProfileRow) -> sqlx::Result<()> {
    sqlx::query(
        "INSERT INTO profile (id, name, job_title, description, location, email, availability,
                              hero_lead, hero_focus, hero_focus_detail, about_intro, updated_at)
         VALUES (1, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
         ON CONFLICT (id) DO UPDATE SET
            name = excluded.name,
            job_title = excluded.job_title,
            description = excluded.description,
            location = excluded.location,
            email = excluded.email,
            availability = excluded.availability,
            hero_lead = excluded.hero_lead,
            hero_focus = excluded.hero_focus,
            hero_focus_detail = excluded.hero_focus_detail,
            about_intro = excluded.about_intro,
            updated_at = excluded.updated_at",
    )
    .bind(&profile.name)
    .bind(&profile.job_title)
    .bind(&profile.description)
    .bind(&profile.location)
    .bind(&profile.email)
    .bind(&profile.availability)
    .bind(&profile.hero_lead)
    .bind(&profile.hero_focus)
    .bind(&profile.hero_focus_detail)
    .bind(&profile.about_intro)
    .bind(crate::content::now_iso())
    .execute(db)
    .await?;
    Ok(())
}

/// True when the content tables are empty, so seeding can install defaults
/// exactly once.
pub async fn is_empty(db: &SqlitePool) -> bool {
    sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM projects")
        .fetch_one(db)
        .await
        .map(|count| count == 0)
        .unwrap_or(false)
}

// ---------------------------------------------------------------- aggregate

/// Everything the public pages need, read once per request.
pub struct Site {
    pub profile: ProfileRow,
    pub socials: Vec<SocialRow>,
    pub categories: Vec<CategoryRow>,
    pub projects: Vec<ProjectRow>,
    pub groups: Vec<ToolGroupRow>,
    pub tools: Vec<ToolRow>,
    pub principles: Vec<PrincipleRow>,
    pub milestones: Vec<MilestoneRow>,
    /// Contact-form topics offered to visitors.
    pub topics: Vec<String>,
}

impl Site {
    pub async fn load(db: &SqlitePool) -> Self {
        let (profile, socials, categories, projects, groups, tools, principles, milestones) = tokio::join!(
            profile(db),
            socials(db),
            categories(db),
            visible_projects(db),
            tool_groups(db),
            tools(db),
            principles(db),
            milestones(db),
        );
        let profile = profile.unwrap_or_else(ProfileRow::fallback);
        let topics = contact_topics(&categories);
        Self {
            profile,
            socials,
            categories,
            projects,
            groups,
            tools,
            principles,
            milestones,
            topics,
        }
    }
}

/// Contact topics follow the live project filters so the form cannot drift from
/// the site. Generic reasons come first.
fn contact_topics(categories: &[CategoryRow]) -> Vec<String> {
    let mut topics = vec!["New project".to_string(), "Collaboration".to_string()];
    topics.extend(
        categories
            .iter()
            .map(|category| format!("{} work", category.label)),
    );
    topics.push("Something else".to_string());
    topics
}

/// Renders a single field against a column/value list. Shared by collection
/// editors and the profile form.
pub fn field_html_for(field: &Field, current: &[(String, String)]) -> String {
    let raw = current
        .iter()
        .find(|(column, _)| column == field.column)
        .map(|(_, value)| value.as_str())
        .unwrap_or_default();
    field_html(field, raw, &LookupOptions::default())
}

impl Site {
    /// An empty site for error pages, which must render even if the database is
    /// unreachable.
    pub fn placeholder() -> Self {
        Self {
            profile: ProfileRow::fallback(),
            socials: Vec::new(),
            categories: Vec::new(),
            projects: Vec::new(),
            groups: Vec::new(),
            tools: Vec::new(),
            principles: Vec::new(),
            milestones: Vec::new(),
            topics: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn options() -> LookupOptions {
        LookupOptions {
            categories: vec![("mobile".into(), "Mobile & web".into())],
            groups: vec![("interfaces".into(), "Interfaces".into())],
            projects: vec![("onekitty".into(), "OneKitty".into())],
        }
    }

    fn projects() -> &'static Collection {
        collection("projects").expect("projects is declared")
    }

    fn field(column: &str) -> &'static Field {
        projects()
            .fields
            .iter()
            .find(|field| field.column == column)
            .expect("field is declared")
    }

    fn base_form() -> HashMap<String, String> {
        let mut form = HashMap::new();
        form.insert("summary".into(), "A short summary.".into());
        form.insert("category".into(), "mobile".into());
        form.insert("accent".into(), "lime".into());
        form
    }

    #[test]
    fn stored_lists_render_one_item_per_line() {
        let stack = serde_json::to_string(&vec!["Flutter", "Rust"]).unwrap();
        assert_eq!(display_value(field("stack"), &stack), "Flutter\nRust");
        assert_eq!(
            display_value(field("features"), r#"[{"title":"A","detail":"B"}]"#),
            "A :: B"
        );
    }

    #[test]
    fn submitted_lists_are_stored_as_json() {
        let stack = parse_value(field("stack"), "Flutter\n\n  Rust  \n", &options()).unwrap();
        assert_eq!(stack, r#"["Flutter","Rust"]"#);

        let features = parse_value(field("features"), "Title :: Detail\nBare", &options()).unwrap();
        let parsed: Vec<Value> = serde_json::from_str(&features).unwrap();
        assert_eq!(parsed[0]["title"], "Title");
        assert_eq!(parsed[0]["detail"], "Detail");
        assert_eq!(parsed[1]["title"], "Bare");
        assert_eq!(parsed[1]["detail"], "");
    }

    #[test]
    fn required_fields_are_enforced() {
        let mut form = base_form();
        form.insert("name".into(), "   ".into());
        assert_eq!(
            parse_form(projects(), &form, &options()).unwrap_err(),
            "Name is required."
        );
    }

    #[test]
    fn stale_lookup_values_are_rejected() {
        let mut form = base_form();
        form.insert("name".into(), "Example".into());
        form.insert("category".into(), "retired-filter".into());
        assert_eq!(
            parse_form(projects(), &form, &options()).unwrap_err(),
            "Filter: that option no longer exists."
        );
    }

    #[test]
    fn blank_slug_is_derived_from_the_name() {
        let mut form = base_form();
        form.insert("name".into(), "Example Project!".into());
        form.insert("slug".into(), "".into());
        let values = parse_form(projects(), &form, &options()).unwrap();
        let slug = values
            .iter()
            .find(|(column, _)| column == "slug")
            .map(|(_, value)| value.as_str());
        assert_eq!(slug, Some("example-project"));
    }

    #[test]
    fn an_explicit_slug_wins() {
        let mut form = base_form();
        form.insert("name".into(), "Example Project".into());
        form.insert("slug".into(), "Custom Slug".into());
        let values = parse_form(projects(), &form, &options()).unwrap();
        let slug = values
            .iter()
            .find(|(column, _)| column == "slug")
            .map(|(_, value)| value.as_str());
        assert_eq!(slug, Some("custom-slug"));
    }

    #[test]
    fn toggles_store_one_or_zero() {
        assert_eq!(parse_value(field("visible"), "1", &options()).unwrap(), "1");
        assert_eq!(parse_value(field("visible"), "", &options()).unwrap(), "0");
    }

    #[test]
    fn fixed_identifiers_come_from_the_headline() {
        let groups = collection("tool-groups").expect("declared");
        let mut form = HashMap::new();
        form.insert("title".into(), "Interfaces that feel right".into());
        form.insert("number".into(), "01".into());
        let values = parse_form(groups, &form, &options()).unwrap();
        assert_eq!(row_id(groups, &values), "interfaces-that-feel-right");
    }

    #[test]
    fn profile_rejects_an_unusable_email() {
        let mut form = HashMap::new();
        form.insert("name".into(), "Name".into());
        form.insert("job_title".into(), "Engineer".into());
        form.insert(
            "description".into(),
            "A description long enough to pass.".into(),
        );
        form.insert("location".into(), "Nairobi".into());
        form.insert("email".into(), "not-an-email".into());
        form.insert("hero_lead".into(), "A lead paragraph long enough.".into());
        form.insert(
            "about_intro".into(),
            "An about paragraph long enough to pass.".into(),
        );
        assert_eq!(
            parse_profile(&form).unwrap_err(),
            "That email address does not look right."
        );
    }

    #[test]
    fn contact_topics_follow_the_filters() {
        let topics = contact_topics(&[
            CategoryRow {
                id: "mobile".into(),
                label: "Mobile & web".into(),
            },
            CategoryRow {
                id: "ai".into(),
                label: "Applied AI".into(),
            },
        ]);
        assert_eq!(topics[0], "New project");
        assert!(topics.contains(&"Mobile & web work".to_string()));
        assert_eq!(topics.last().unwrap(), "Something else");
    }
}
