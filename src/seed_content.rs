//! First-run content. Installed only when the portfolio tables are empty, so an
//! operator who edits or deletes a row never sees it come back on restart.

use std::collections::HashMap;

use sqlx::SqlitePool;

use crate::portfolio::{self, collection, save_profile, Collection, LookupOptions, ProfileRow};

type SeedRow = &'static [(&'static str, &'static str)];

fn form_of(seed: &SeedRow) -> HashMap<String, String> {
    seed.iter()
        .map(|(key, value)| (key.to_string(), value.to_string()))
        .collect()
}

fn default_profile() -> ProfileRow {
    ProfileRow {
        name: "Gideon Sigilai".to_string(),
        job_title: "Software Engineer".to_string(),
        description: "Gideon Sigilai is a software engineer in Nairobi, Kenya, building thoughtful mobile and web products with Flutter, Go, Firebase, Rust, and on-device machine learning.".to_string(),
        location: "Nairobi, Kenya".to_string(),
        email: "gideon.sigilai3000@gmail.com".to_string(),
        availability: "Available for new work".to_string(),
        hero_lead: "I turn complex problems into simple, thoughtful software — from the first tap to the systems behind it.".to_string(),
        hero_focus: "Mobile · Web · Applied AI".to_string(),
        hero_focus_detail: "Flutter and Dart for product surfaces; Go, Rust, and Firebase for the systems underneath.".to_string(),
        about_intro: "I'm a Nairobi-based software engineer who enjoys connecting the dots between a good idea and a product people can actually use. My core craft is Flutter development: responsive interfaces, clear architecture, and the small details that make an app feel right.".to_string(),
    }
}

/// Installs the starter portfolio. No-op once any project exists.
pub async fn seed_portfolio(db: &SqlitePool) -> Result<(), Box<dyn std::error::Error>> {
    if !portfolio::is_empty(db).await {
        return Ok(());
    }

    save_profile(db, &default_profile()).await?;

    // Filters and groups come first: projects and tools point at them, and the
    // seed data reuses the original identifiers.
    insert(db, "categories", CATEGORIES).await?;
    insert(db, "tool-groups", TOOL_GROUPS).await?;
    insert(db, "projects", PROJECTS).await?;
    insert(db, "tools", TOOLS).await?;
    insert(db, "principles", PRINCIPLES).await?;
    insert(db, "milestones", MILESTONES).await?;
    insert(db, "socials", SOCIALS).await?;

    tracing::info!(
        projects = PROJECTS.len(),
        tools = TOOLS.len(),
        "seeded portfolio content"
    );
    Ok(())
}

async fn insert(
    db: &SqlitePool,
    slug: &'static str,
    rows: &[SeedRow],
) -> Result<(), Box<dyn std::error::Error>> {
    let collection: &Collection = collection(slug).expect("seeded collection is declared");
    for seed in rows {
        let form = form_of(seed);
        // Lookups are re-read per row so a row can point at one inserted above.
        let options = LookupOptions::load(db).await;
        let values = portfolio::parse_form(collection, &form, &options)
            .map_err(|error| format!("seed {slug}: {error}"))?;
        let explicit_id = form
            .get("id")
            .map(String::as_str)
            .filter(|id| !id.is_empty());
        portfolio::insert_row_with_id(db, collection, explicit_id, &values).await?;
    }
    Ok(())
}

const CATEGORIES: &[SeedRow] = &[
    &[("id", "mobile"), ("label", "Mobile & web")],
    &[("id", "ai"), ("label", "Applied AI")],
];

const TOOL_GROUPS: &[SeedRow] = &[
    &[
        ("id", "interfaces"),
        ("number", "01"),
        ("title", "Interfaces that feel right"),
        ("subtitle", "Mobile & web"),
    ],
    &[
        ("id", "systems"),
        ("number", "02"),
        ("title", "Systems that hold up"),
        ("subtitle", "Backend & data"),
    ],
    &[
        ("id", "explore"),
        ("number", "03"),
        ("title", "Ideas worth exploring"),
        ("subtitle", "AI & developer tools"),
    ],
];

const PROJECTS: &[SeedRow] = &[
    &[
        ("name", "OneKitty"),
        ("slug", "onekitty"),
        ("summary", "Making group finance feel effortless."),
        ("tagline", "Better together."),
        ("category", "mobile"),
        ("category_label", "Mobile & web / Fintech"),
        ("kicker", "ONEKITTY / FINTECH"),
        ("year", "2025 — now"),
        ("role", "Flutter engineer · mobile experience"),
        ("status", "Shipped · private client"),
        ("accent", "lime"),
        ("stack", "Flutter\nGetX\nDio\nGetStorage\nPayments"),
        (
            "features",
            "Contributions without confusion :: Group pots, scheduled contributions, and a running ledger so every member can see where the money went without asking.\nStatements people keep :: Filterable transactions with exportable statements for personal records and group reporting.\nPayments that fit the region :: Regional payment integrations, KYC flows, and multilingual copy built around how people actually transact.\nMore than money :: Event ticket validation inside the same app, so community life does not need a second product.",
        ),
        (
            "story",
            "Group finance breaks in predictable places: someone forgets a contribution, the ledger drifts from reality, and trust erodes. OneKitty treats the ledger as the product.\nI built the mobile experience end to end in Flutter — a single controller per slice, sealed UI state, and a design system that holds up from a 5-inch phone to a tablet. Every mutation re-reads from the source of truth, so the balance on screen is never a guess.\nThe hard part was not the UI. It was making failure legible: a payment that is pending must never look like a payment that succeeded.",
        ),
        (
            "outcome",
            "One codebase serving phone and tablet layouts\nStatement export and PDF-ready transaction history\nKYC, multilingual, and payment flows behind a single state model",
        ),
        ("link_label", "View on Google Play"),
        ("link_href", "https://play.google.com/store/search?q=onekitty&c=apps&hl=en"),
        ("image", "/images/onekitty.jpg"),
        ("visible", "1"),
    ],
    &[
        ("name", "OneChurch"),
        ("slug", "onechurch"),
        ("summary", "A connected home for church communities."),
        ("tagline", "People first. Always."),
        ("category", "mobile"),
        ("category_label", "Mobile & web / Community"),
        ("kicker", "ONECHURCH / COMMUNITY"),
        ("year", "2025"),
        ("role", "Flutter engineer · web and admin"),
        ("status", "Shipped · private client"),
        ("accent", "clay"),
        ("stack", "Flutter Web\nFirebase\nREST APIs\nResponsive UI"),
        (
            "features",
            "Member experience :: Announcements, events, giving, and small-group coordination in one place instead of five group chats.\nAdministration that respects time :: Role-based admin screens for attendance, scheduling, and member records built for non-technical staff.\nOne system, two surfaces :: Shared Flutter code deployed to mobile and web, with layouts that adapt instead of stretching.\nSafe by default :: Firebase hosting and rules keep member data scoped to the role that needs it.",
        ),
        (
            "story",
            "A church is a community with an operations problem. Leaders need to know who is coming, who needs care, and what is happening this week — without asking an administrator to become a developer.\nI built the shared Flutter layer and the admin experience, concentrating on the unglamorous parts: predictable forms, clear empty states, role-aware navigation, and responsive layouts for the web build.\nThe result is a platform that feels like one product on a phone and a laptop, because it is one product.",
        ),
        (
            "outcome",
            "Cross-platform Flutter with a responsive web build\nRole-aware administration for volunteers, not engineers\nFirebase hosting and rules behind a single deployment",
        ),
        ("link_label", ""),
        ("link_href", ""),
        ("image", "/images/onechurch.jpg"),
        ("visible", "1"),
    ],
    &[
        ("name", "Plant Disease Detector"),
        ("slug", "plant-disease-detector"),
        ("summary", "Bringing image classification onto the device."),
        ("tagline", "Small leaf. Big insight."),
        ("category", "ai"),
        ("category_label", "Applied AI / Agriculture"),
        ("kicker", "PLANT HEALTH / MACHINE LEARNING"),
        ("year", "2026"),
        ("role", "ML + Flutter engineer"),
        ("status", "Model published · on-device prototype"),
        ("accent", "sky"),
        ("stack", "TensorFlow\nTFLite\nFlutter\nPython\nDataset design"),
        (
            "features",
            "38 crop classes on-device :: A classifier trained across tomatoes, potatoes, corn, and fruit, exported to TFLite for offline inference.\nCurated, documented datasets :: Two published datasets — a 38-class field set and a 4-class rose foliage set — used to train and validate the model.\nOffline by design :: No upload, no round trip. A photo is analysed on the phone in a field with two bars of signal.\nAdvice, not just a label :: The result screen leads with a next action and asks for a second photo when confidence is low.",
        ),
        (
            "story",
            "Most plant-disease demos die in the notebook. The interesting constraint was deployment: a mid-range Android phone, no reliable connection, and a user who needs a decision, not a probability.\nI started from the budget — inference time, app size, and battery — then trained with quantisation in mind, verifying the exported graph against the same labelled sample rather than trusting the training split.\nThe datasets are public, the model is portable, and the app treats uncertainty as a first-class state.",
        ),
        (
            "outcome",
            "Published: 38-class plant disease dataset (2 GB) and 4-class rose foliage dataset\nTFLite export verified against a held-out sample after quantisation\nOffline-first Flutter flow with confidence-aware guidance",
        ),
        ("link_label", "View on Google Play"),
        ("link_href", "https://play.google.com/store/apps/details?id=com.saweapps.onlyfarms&hl=en"),
        ("image", "/images/plant_doc.jpg"),
        ("visible", "1"),
    ],
    &[
        ("name", "Meioufei Logistics"),
        ("slug", "meioufei"),
        ("summary", "Coordinating sourcing and sea freight between China and Kenya."),
        ("tagline", "Your bridge between China and Kenya."),
        ("category", "mobile"),
        ("category_label", "Web / Logistics"),
        ("kicker", "MEIOUFEI / LOGISTICS"),
        ("year", "2026"),
        ("role", "Full-stack engineer · web"),
        ("status", "Live · public site"),
        ("accent", "clay"),
        ("stack", "Astro\nTailwind\nNetlify\ni18n"),
        (
            "features",
            "Sourcing support :: Help identifying suitable suppliers and product options in China.\nPurchasing support :: Practical coordination from supplier discussion to purchase preparation.\nLCL & FCL shipping :: Sea freight coordination for shared or full-container cargo.\nCargo consolidation :: Bring eligible goods from multiple suppliers into one shipment.\nMultilingual :: 16 languages including Kiswahili and Chinese for Kenyan and Chinese partners.",
        ),
        (
            "story",
            "Meioufei connects Kenyan businesses with suppliers and sea freight from China. I built the public site as a fast, multilingual Astro app with clear service pages, a step-by-step process, FAQs, and a quote request flow — so an importer can understand the route and ask for a quote without a sales call.\nThe focus was clarity: one point of contact, transparent quoting, and content that works across 16 languages.",
        ),
        (
            "outcome",
            "Live multilingual site across 16 languages\nService, process, FAQ, and quote-request flows\nDeployed on Netlify with edge delivery",
        ),
        ("link_label", "Visit site"),
        ("link_href", "https://meioufei.netlify.app/"),
        ("image", "/images/meioufei.png"),
        ("visible", "1"),
    ],
    &[
        ("name", "MindPocket"),
        ("slug", "mindpocket"),
        ("summary", "Screenshots and voice notes, kept in one offline, searchable pocket."),
        ("tagline", "Save it now. Find it later."),
        ("category", "mobile"),
        ("category_label", "Mobile & web / Personal knowledge"),
        ("kicker", "MINDPOCKET / PERSONAL KNOWLEDGE"),
        ("year", "2026"),
        ("role", "Flutter engineer · mobile, desktop, and on-device ML"),
        ("status", "Closed testing · Google Play"),
        ("accent", "sky"),
        ("stack", "Flutter\nGetX\nDio\nGetStorage\nSQLite + FTS5\nML Kit\nsherpa-onnx"),
        (
            "features",
            "One searchable pocket :: Screenshots, voice notes, typed notes, and extracted text live in one local collection, with FTS5 search over titles, notes, tags, OCR, and transcripts.\nScreenshots that file themselves :: Android share intent, image picker, and OS folder scans import only unseen images, with durable receipts and thumbnails.\nVoice that stays on device :: Optional sherpa-onnx Whisper models are downloaded, SHA-256 verified, and transcribed in a background isolate, with pause and resume.\nOn-device OCR :: ML Kit's bundled Latin recognizer on Android and iOS, Apple Vision on macOS, Tesseract on Windows and Linux — no Play Services, no first-use download.\nJobs that survive interruption :: A persisted queue runs one job at a time; interrupted work is requeued on launch and transcription never blocks OCR.\nRelated memories :: Related images surface from at least 90% character-level text similarity, computed off the main thread.",
        ),
        (
            "story",
            "MindPocket started from a simple frustration: the screenshots and voice memos that matter most are scattered across galleries and recorders, and finding one later means scrolling.\nI built it as a single offline collection in Flutter, with GetX for state and dependency injection, SQLite plus FTS5 for search, and background isolates for OCR and transcription so the UI stays responsive while heavy work runs.\nEverything stays on the device by default. Cloud analysis and AI grouping upload text only, never image bytes, and only when a proxy is configured. The hard parts were making failure legible — a pending transcription must never look finished, and a cancelled job must leave no orphaned files.",
        ),
        (
            "outcome",
            "Android-first release in closed testing on Google Play\nOn-device OCR across Android, iOS, macOS, Windows, and Linux\nOffline transcription with verified Whisper models\nPersistent job queue with retry, cancel, and requeue",
        ),
        ("link_label", "View on Google Play"),
        ("link_href", "https://play.google.com/store/apps/details?id=com.saweapps.mindpocket&hl=en"),
        ("image", "/images/mindpocket.jpg"),
        ("visible", "1"),
    ],
];

const TOOLS: &[SeedRow] = &[
    &[("name", "Flutter"), ("group_id", "interfaces"), ("description", "One codebase, carefully adapted to mobile and web. Flutter is at the heart of my product development."), ("project_slug", "onekitty")],
    &[("name", "Dart"), ("group_id", "interfaces"), ("description", "The language behind my Flutter work, connecting interface logic, asynchronous operations, and application models."), ("project_slug", "onekitty")],
    &[("name", "GetX"), ("group_id", "interfaces"), ("description", "State management, dependency injection, and navigation to keep complex application flows organised."), ("project_slug", "onechurch")],
    &[("name", "Responsive UI"), ("group_id", "interfaces"), ("description", "Layouts that adapt to the space available, from a phone in one hand to a full desktop workspace."), ("project_slug", "onechurch")],
    &[("name", "Go"), ("group_id", "systems"), ("description", "A focus for backend development: clear server logic, API design, and dependable services."), ("project_slug", "")],
    &[("name", "Rust"), ("group_id", "systems"), ("description", "Where I put the parts that must not be wrong: typed validation, sanitised rendering, and the server behind this site."), ("project_slug", "")],
    &[("name", "Firebase"), ("group_id", "systems"), ("description", "Cloud services used across my projects, including hosting Flutter web experiences."), ("project_slug", "onechurch")],
    &[("name", "Supabase"), ("group_id", "systems"), ("description", "A relational approach to application data, combining PostgreSQL with backend services."), ("project_slug", "")],
    &[("name", "REST APIs"), ("group_id", "systems"), ("description", "Connecting interfaces to real data, with attention to loading states, errors, and predictable data flows."), ("project_slug", "onekitty")],
    &[("name", "Python"), ("group_id", "explore"), ("description", "A practical workspace for preparing datasets, training models, and evaluating results."), ("project_slug", "plant-disease-detector")],
    &[("name", "TensorFlow"), ("group_id", "explore"), ("description", "Building and training image classification models for plant disease detection."), ("project_slug", "plant-disease-detector")],
    &[("name", "TFLite"), ("group_id", "explore"), ("description", "Bringing trained models onto a device, connecting machine learning with a usable mobile experience."), ("project_slug", "plant-disease-detector")],
    &[("name", "Git"), ("group_id", "explore"), ("description", "Version control for tracking changes and collaborating throughout the development process."), ("project_slug", "")],
];

const PRINCIPLES: &[SeedRow] = &[
    &[("title", "Clarity over cleverness"), ("body", "If a screen needs a paragraph to explain, the design is not finished. Interfaces should teach themselves.")],
    &[("title", "Respect the network"), ("body", "Loading, empty, and failed are design states, not error handling afterthoughts. Every request gets a visible answer.")],
    &[("title", "Craft is a requirement"), ("body", "Spacing, focus rings, and 44-pixel targets are how a product shows it respects the person holding it.")],
    &[("title", "Secure by default"), ("body", "Secrets stay server-side, untrusted text is rendered safely, and every mutation is validated twice.")],
];

const MILESTONES: &[SeedRow] = &[
    &[
        ("title", "B.Sc. Computer Science"),
        ("marker", "2024"),
        ("detail", "Gretsa University, Thika"),
    ],
    &[
        ("title", "First place · Mobile Development"),
        ("marker", "01"),
        ("detail", "Gretsa National Programming Contest, March 2024"),
    ],
    &[
        ("title", "Product engineering"),
        ("marker", "Now"),
        ("detail", "Flutter, Rust, and on-device ML for real users"),
    ],
];

const SOCIALS: &[SeedRow] = &[
    &[
        ("label", "GitHub"),
        ("handle", "@gideonsigilai"),
        ("href", "https://github.com/gideonsigilai"),
        ("icon", "github"),
    ],
    &[
        ("label", "LinkedIn"),
        ("handle", "Gideon Sigilai"),
        ("href", "https://www.linkedin.com/in/gideon-sigilai"),
        ("icon", "linkedin"),
    ],
    &[
        ("label", "Ultralytics"),
        ("handle", "@gideon-sigilai"),
        ("href", "https://platform.ultralytics.com/gideon-sigilai"),
        ("icon", "model"),
    ],
    &[
        ("label", "Email"),
        ("handle", "gideon.sigilai3000@gmail.com"),
        ("href", "mailto:gideon.sigilai3000@gmail.com"),
        ("icon", "mail"),
    ],
];
