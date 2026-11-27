use serde_json::Value;

/// Minimal mustache-style renderer: `{{name}}` escapes, `{{{name}}}` is raw,
/// `{{#name}}…{{/name}}` renders once for a truthy value or iterates a list, and
/// `{{^name}}…{{/name}}` renders when the value is empty.
pub fn render(template: &str, data: &Value) -> String {
    let mut out = String::with_capacity(template.len() * 2);
    render_into(template, &[data], &mut out);
    out
}

/// `stack` is the mustache context chain, innermost last. A section pushes a
/// context; lookups walk back outwards until a key resolves.
fn render_into(template: &str, stack: &[&Value], out: &mut String) {
    let mut rest = template;
    while let Some(start) = rest.find("{{") {
        out.push_str(&rest[..start]);
        let after = &rest[start + 2..];

        // Triple braces: raw output.
        if let Some(body) = after.strip_prefix('{') {
            let Some(end) = body.find("}}}") else {
                out.push_str(&rest[start..]);
                return;
            };
            let key = body[..end].trim();
            out.push_str(&stringify(lookup(stack, key)));
            rest = &body[end + 3..];
            continue;
        }

        let Some(end) = after.find("}}") else {
            out.push_str(&rest[start..]);
            return;
        };
        let tag = after[..end].trim();
        rest = &after[end + 2..];

        if let Some(name) = tag.strip_prefix('#') {
            let name = name.trim();
            let (block, remainder) = split_block(rest, name);
            rest = remainder;
            let Some(block) = block else { continue };
            match lookup(stack, name) {
                Value::Array(items) => {
                    let mut inner: Vec<&Value> = stack.to_vec();
                    inner.push(&Value::Null);
                    for item in &items {
                        *inner.last_mut().expect("stack is never empty") = item;
                        render_into(block, &inner, out);
                    }
                }
                Value::Bool(false) | Value::Null => {}
                Value::String(text) if text.is_empty() => {}
                value => {
                    let mut inner: Vec<&Value> = stack.to_vec();
                    inner.push(&value);
                    render_into(block, &inner, out);
                }
            }
            continue;
        }

        if let Some(name) = tag.strip_prefix('^') {
            let name = name.trim();
            let (block, remainder) = split_block(rest, name);
            rest = remainder;
            let Some(block) = block else { continue };
            let empty = match lookup(stack, name) {
                Value::Bool(false) | Value::Null => true,
                Value::String(text) => text.is_empty(),
                Value::Array(items) => items.is_empty(),
                _ => false,
            };
            if empty {
                render_into(block, stack, out);
            }
            continue;
        }

        let value = lookup(stack, tag);
        if requires_raw(tag) {
            out.push_str(&stringify(value));
        } else {
            out.push_str(&escape_html(&stringify(value)));
        }
    }
    out.push_str(rest);
}

/// Fields holding pre-rendered or pre-escaped HTML.
fn requires_raw(tag: &str) -> bool {
    matches!(tag, "body" | "article" | "csrf" | "head_extra")
}

fn split_block<'a>(rest: &'a str, name: &str) -> (Option<&'a str>, &'a str) {
    let closing = format!("{{{{/{name}}}}}");
    if let Some(end) = rest.find(&closing) {
        return (Some(&rest[..end]), &rest[end + closing.len()..]);
    }
    (None, rest)
}

/// Resolves a dotted path against the innermost context that has its first key.
fn lookup(stack: &[&Value], path: &str) -> Value {
    let mut keys = path.split('.');
    let first = match keys.next() {
        Some(first) => first,
        None => return Value::Null,
    };
    for context in stack.iter().rev() {
        let Some(mut current) = context.get(first) else {
            continue;
        };
        for key in path.split('.').skip(1) {
            match current.get(key) {
                Some(next) => current = next,
                None => return Value::Null,
            }
        }
        return current.clone();
    }
    Value::Null
}

fn stringify(value: Value) -> String {
    match value {
        Value::Null => String::new(),
        Value::String(text) => text,
        Value::Bool(flag) => flag.to_string(),
        Value::Number(number) => number.to_string(),
        Value::Array(_) | Value::Object(_) => String::new(),
    }
}

pub fn escape_html(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(ch),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn interpolates_and_escapes() {
        let data = json!({ "name": "<script>x</script>", "raw": "<b>ok</b>" });
        assert_eq!(
            render("Hi {{name}}", &data),
            "Hi &lt;script&gt;x&lt;/script&gt;"
        );
        assert_eq!(render("Hi {{{raw}}}", &data), "Hi <b>ok</b>");
    }

    #[test]
    fn iterates_lists_and_nested_fields() {
        let data = json!({ "posts": [{ "title": "A" }, { "title": "B" }] });
        assert_eq!(render("{{#posts}}[{{title}}]{{/posts}}", &data), "[A][B]");
    }

    #[test]
    fn inverted_section_handles_empty_list() {
        let data = json!({ "posts": [] });
        assert_eq!(render("{{^posts}}none{{/posts}}", &data), "none");
    }

    #[test]
    fn missing_keys_render_empty() {
        assert_eq!(render("x{{nope}}y", &json!({})), "xy");
    }

    #[test]
    fn dotted_paths_resolve() {
        let data = json!({ "site": { "name": "Gideon" } });
        assert_eq!(render("{{site.name}}", &data), "Gideon");
    }

    #[test]
    fn flag_sections_keep_parent_context() {
        let data = json!({
            "has_posts": true,
            "featured": { "slug": "alpha" },
            "has_none": false
        });
        assert_eq!(
            render(
                "{{#has_posts}}<a href=\"/writing/{{featured.slug}}\">{{/has_posts}}",
                &data
            ),
            "<a href=\"/writing/alpha\">"
        );
        assert_eq!(render("x{{#has_none}}hidden{{/has_none}}y", &data), "xy");
    }

    #[test]
    fn object_sections_push_their_own_context() {
        let data = json!({
            "csrf": "root-token",
            "detail": { "name": "Ada", "messages": [{ "body": "hi" }] }
        });
        assert_eq!(
            render(
                "{{#detail}}{{name}}|{{csrf}}{{#messages}}[{{body}}]{{/messages}}{{/detail}}",
                &data
            ),
            "Ada|root-token[hi]"
        );
    }
}
