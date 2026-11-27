use ammonia::Builder;
use pulldown_cmark::{html, Options, Parser};

use crate::state::{ApiError, ApiResult};

/// Renders owner-written Markdown to HTML and strips anything executable.
pub fn render_markdown(markdown: &str) -> String {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_FOOTNOTES);
    options.insert(Options::ENABLE_SMART_PUNCTUATION);
    let parser = Parser::new_ext(markdown, options);
    let mut html_out = String::new();
    html::push_html(&mut html_out, parser);
    Builder::default()
        .add_tags(["figure", "figcaption", "del", "ins", "mark"])
        .add_tag_attributes("img", ["loading", "decoding"])
        .clean(&html_out)
        .to_string()
}

/// The page already shows the title as its `<h1>`, so demote every heading one
/// level and drop a leading H1 that would repeat the title.
pub fn normalise_headings(html: &str, title: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let mut cursor = 0;
    while let Some(offset) = html[cursor..].find('<') {
        let start = cursor + offset;
        out.push_str(&html[cursor..start]);

        let mut demoted = false;
        for level in [1u8, 2, 3, 4, 5] {
            let open = format!("<h{level}>");
            let close = format!("</h{level}>");
            if html[start..].starts_with(&open) {
                out.push_str(&format!("<h{}>", level + 1));
                cursor = start + open.len();
                demoted = true;
                break;
            }
            if html[start..].starts_with(&close) {
                out.push_str(&format!("</h{}>", level + 1));
                cursor = start + close.len();
                demoted = true;
                break;
            }
        }
        if !demoted {
            out.push('<');
            cursor = start + 1;
        }
    }
    out.push_str(&html[cursor..]);

    let escaped_title = title
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;");
    let leading = format!("<h2>{escaped_title}</h2>");
    let trimmed = out.trim_start();
    if trimmed.starts_with(&leading) {
        trimmed[leading.len()..].trim_start().to_string()
    } else {
        out
    }
}

pub fn reading_minutes(markdown: &str) -> i64 {
    let words = markdown.split_whitespace().count();
    ((words as f64 / 220.0).ceil() as i64).max(1)
}

pub fn derive_excerpt(markdown: &str) -> String {
    let mut text = String::new();
    let mut in_fence = false;
    for line in markdown.lines() {
        let line = line.trim();
        if line.starts_with("```") {
            in_fence = !in_fence;
            continue;
        }
        if in_fence || line.is_empty() || line.starts_with('#') || line.starts_with('>') {
            continue;
        }
        if !text.is_empty() {
            text.push(' ');
        }
        text.push_str(line.trim_start_matches(['-', '*', '+', ' ']));
        if text.chars().count() > 200 {
            break;
        }
    }
    let clipped: String = text.chars().take(180).collect();
    if text.chars().count() > 180 {
        format!("{}…", clipped.trim_end())
    } else {
        clipped
    }
}

/// Strips tags for metadata and meta descriptions.
pub fn plain_text(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let mut inside = false;
    for ch in html.chars() {
        match ch {
            '<' => inside = true,
            '>' => {
                inside = false;
                out.push(' ');
            }
            _ if !inside => out.push(ch),
            _ => {}
        }
    }
    out.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn slugify(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut last_dash = true;
    for ch in input.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
            last_dash = false;
        } else if !last_dash {
            out.push('-');
            last_dash = true;
        }
    }
    out.trim_matches('-').to_string()
}

pub fn clean_text(value: &str, min: usize, max: usize, field: &str) -> ApiResult<String> {
    let trimmed = value.trim();
    let length = trimmed.chars().count();
    if length < min {
        return Err(ApiError::bad(format!("{field} is too short.")));
    }
    if length > max {
        return Err(ApiError::bad(format!("{field} is too long.")));
    }
    Ok(trimmed.to_string())
}

pub fn looks_like_email(value: &str) -> bool {
    if value.len() > 254 || value.contains(char::is_whitespace) {
        return false;
    }
    let mut parts = value.split('@');
    let (Some(local), Some(domain), None) = (parts.next(), parts.next(), parts.next()) else {
        return false;
    };
    !local.is_empty()
        && local.len() <= 64
        && domain.contains('.')
        && !domain.starts_with('.')
        && !domain.ends_with('.')
        && !domain.contains("..")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn markdown_strips_scripts_and_handlers() {
        let html = render_markdown(
            "# hi\n\n<script>alert(1)</script>\n\n<img src=x onerror=alert(1)>\n\n[link](javascript:alert(1))",
        );
        assert!(!html.contains("<script"));
        assert!(!html.contains("onerror"));
        assert!(!html.contains("javascript:"));
        assert!(html.contains("<h1>hi</h1>"));
    }

    #[test]
    fn headings_are_demoted_and_title_duplicate_dropped() {
        let html = render_markdown("# State that does not fight you\n\nIntro text.");
        let normalised = normalise_headings(&html, "State that does not fight you");
        assert!(!normalised.contains("<h2>State that does not fight you</h2>"));
        assert!(normalised.contains("Intro text."));

        let nested = render_markdown("## Section\n\n#### Deep");
        let normalised = normalise_headings(&nested, "T");
        assert!(normalised.contains("<h3>Section</h3>"));
        assert!(normalised.contains("<h5>Deep</h5>"));
    }

    #[test]
    fn slugs_are_ascii_and_collapse_separators() {
        assert_eq!(
            slugify("State that does not fight you!"),
            "state-that-does-not-fight-you"
        );
        assert_eq!(slugify("  Rust  &  Next.js  "), "rust-next-js");
        assert_eq!(slugify("---"), "");
    }

    #[test]
    fn excerpt_skips_headings_and_code() {
        assert_eq!(
            derive_excerpt("# Title\n\nFirst line here.\n\n> quote"),
            "First line here."
        );
        assert_eq!(derive_excerpt("```\ncode\n```\n\nReal text."), "Real text.");
    }

    #[test]
    fn reading_time_has_a_floor() {
        assert_eq!(reading_minutes("one two three"), 1);
        assert_eq!(reading_minutes(&"word ".repeat(440)), 2);
    }

    #[test]
    fn plain_text_removes_markup() {
        assert_eq!(
            plain_text("<p>Hello <strong>there</strong>.</p>"),
            "Hello there ."
        );
    }
}
