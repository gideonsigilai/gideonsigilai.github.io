                "stack_line": project.stack().join(" · "),
    let page = if !project.image.is_empty() {
        let image_url = if project.image.starts_with("http://") || project.image.starts_with("https://") {
            project.image.clone()
        } else {
            format!("{}{}", base(&state), project.image)
        };
        page.social_image(image_url)
            .social_image_alt(format!("{} project artwork by {}.", project.name, site.profile.name))
    } else {
        page
    };
    let projects = sqlx::query_as::<_, (String, String, String)>(
        "SELECT slug, updated_at, image FROM projects WHERE visible = 1 ORDER BY position, slug",
    let posts = sqlx::query_as::<_, (String, String, String)>(
        "SELECT slug, updated_at, cover_image FROM posts WHERE status = 'published'
        .filter_map(|(_, updated, _)| sitemap_modified(updated))
    for (slug, updated, image) in projects {
        let image_url = if image.is_empty() {
            format!("{base_url}/images/work/{slug}.svg")
        } else if image.starts_with("http://") || image.starts_with("https://") {
            image.clone()
        } else {
            format!("{base_url}{image}")
        };
            Some(&image_url),
    for (slug, updated, cover_image) in posts {
        let image_url = if cover_image.is_empty() {
            format!("{base_url}/images/writing/{slug}.svg")
        } else if cover_image.starts_with("http://") || cover_image.starts_with("https://") {
            cover_image.clone()
        } else {
            format!("{base_url}{cover_image}")
        };
            Some(&image_url),