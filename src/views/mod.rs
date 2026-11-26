    pub social_image: Option<String>,
    pub social_image_alt: Option<String>,
            social_image: None,
            social_image_alt: None,

    pub fn social_image(mut self, url: impl Into<String>) -> Self {
        self.social_image = Some(url.into());
        self
    }

    pub fn social_image_alt(mut self, text: impl Into<String>) -> Self {
        self.social_image_alt = Some(text.into());
        self
    }
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
        "social_image": social_image,
        "social_image_type": social_image_type,
        "social_image_alt": social_image_alt,
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
        "image": image,
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
        "image": image,
        "inLanguage": "en",