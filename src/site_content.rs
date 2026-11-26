use serde_json::{json, Value};

pub const SITE_NAME: &str = "Gideon Sigilai";
pub const SITE_DESCRIPTION: &str = "Gideon Sigilai is a software engineer in Nairobi, Kenya, building thoughtful mobile and web products with Flutter, Go, Firebase, Rust, and on-device machine learning.";
pub const SITE_LOCATION: &str = "Nairobi, Kenya";
pub const SITE_EMAIL: &str = "gideon.sigilai3000@gmail.com";
pub const SITE_GITHUB: &str = "https://github.com/gideonsigilai";
pub const SITE_X: &str = "https://x.com/kenyanutmost";
pub const SITE_LINKEDIN: &str = "https://www.linkedin.com/in/gideon-sigilai";
pub const SITE_ULTRALYTICS: &str = "https://platform.ultralytics.com/gideon-sigilai";

pub fn socials() -> Value {
    json!([
        { "label": "GitHub", "handle": "@gideonsigilai", "href": SITE_GITHUB, "icon": "github" },
        { "label": "X", "handle": "@kenyanutmost", "href": SITE_X, "icon": "x" },
        { "label": "LinkedIn", "handle": "Gideon Sigilai", "href": SITE_LINKEDIN, "icon": "linkedin" },
        { "label": "Ultralytics", "handle": "@gideon-sigilai", "href": SITE_ULTRALYTICS, "icon": "model" },
        { "label": "Email", "handle": SITE_EMAIL, "href": format!("mailto:{SITE_EMAIL}"), "icon": "mail" }
    ])
}

pub fn projects() -> Value {
    json!([
        {
            "slug": "onekitty",
            "name": "OneKitty",
            "category": "mobile",
            "category_label": "Mobile & web / Fintech",
            "kicker": "ONEKITTY / FINTECH",
            "tagline": "Better together.",
            "summary": "Making group finance feel effortless.",
            "year": "2025 — now",
            "role": "Flutter engineer · mobile experience",
            "status": "Shipped · private client",
            "stack": ["Flutter", "GetX", "Dio", "GetStorage", "Payments"],
            "accent": "lime",
            "features": [
                { "title": "Contributions without confusion", "detail": "Group pots, scheduled contributions, and a running ledger so every member can see where the money went without asking." },
                { "title": "Statements people keep", "detail": "Filterable transactions with exportable statements for personal records and group reporting." },
                { "title": "Payments that fit the region", "detail": "Regional payment integrations, KYC flows, and multilingual copy built around how people actually transact." },
                { "title": "More than money", "detail": "Event ticket validation inside the same app, so community life does not need a second product." }
            ],
            "story": [
                "Group finance breaks in predictable places: someone forgets a contribution, the ledger drifts from reality, and trust erodes. OneKitty treats the ledger as the product.",
                "I built the mobile experience end to end in Flutter — a single controller per slice, sealed UI state, and a design system that holds up from a 5-inch phone to a tablet. Every mutation re-reads from the source of truth, so the balance on screen is never a guess.",
                "The hard part was not the UI. It was making failure legible: a payment that is pending must never look like a payment that succeeded."
            ],
            "outcome": [
                "One codebase serving phone and tablet layouts",
                "Statement export and PDF-ready transaction history",
                "KYC, multilingual, and payment flows behind a single state model"
            ]
        },
        {
            "slug": "onechurch",
            "name": "OneChurch",
            "category": "mobile",
            "category_label": "Mobile & web / Community",
            "kicker": "ONECHURCH / COMMUNITY",
            "tagline": "People first. Always.",
            "summary": "A connected home for church communities.",
            "year": "2025",
            "role": "Flutter engineer · web and admin",
            "status": "Shipped · private client",
            "stack": ["Flutter Web", "Firebase", "REST APIs", "Responsive UI"],
            "accent": "clay",
            "features": [
                { "title": "Member experience", "detail": "Announcements, events, giving, and small-group coordination in one place instead of five group chats." },
                { "title": "Administration that respects time", "detail": "Role-based admin screens for attendance, scheduling, and member records built for non-technical staff." },
                { "title": "One system, two surfaces", "detail": "Shared Flutter code deployed to mobile and web, with layouts that adapt instead of stretching." },
                { "title": "Safe by default", "detail": "Firebase hosting and rules keep member data scoped to the role that needs it." }
            ],
            "story": [
                "A church is a community with an operations problem. Leaders need to know who is coming, who needs care, and what is happening this week — without asking an administrator to become a developer.",
                "I built the shared Flutter layer and the admin experience, concentrating on the unglamorous parts: predictable forms, clear empty states, role-aware navigation, and responsive layouts for the web build.",
                "The result is a platform that feels like one product on a phone and a laptop, because it is one product."
            ],
            "outcome": [
                "Cross-platform Flutter with a responsive web build",
                "Role-aware administration for volunteers, not engineers",
                "Firebase hosting and rules behind a single deployment"
            ]
        },
        {
            "slug": "plant-disease-detector",
            "name": "Plant Disease Detector",
            "category": "ai",
            "category_label": "Applied AI / Agriculture",
            "kicker": "PLANT HEALTH / MACHINE LEARNING",
            "tagline": "Small leaf. Big insight.",
            "summary": "Bringing image classification onto the device.",
            "year": "2026",
            "role": "ML + Flutter engineer",
            "status": "Model published · on-device prototype",
            "stack": ["TensorFlow", "TFLite", "Flutter", "Python", "Dataset design"],
            "accent": "sky",
            "features": [
                { "title": "38 crop classes on-device", "detail": "A classifier trained across tomatoes, potatoes, corn, and fruit, exported to TFLite for offline inference." },
                { "title": "Curated, documented datasets", "detail": "Two published datasets — a 38-class field set and a 4-class rose foliage set — used to train and validate the model." },
                { "title": "Offline by design", "detail": "No upload, no round trip. A photo is analysed on the phone in a field with two bars of signal." },
                { "title": "Advice, not just a label", "detail": "The result screen leads with a next action and asks for a second photo when confidence is low." }
            ],
            "story": [
                "Most plant-disease demos die in the notebook. The interesting constraint was deployment: a mid-range Android phone, no reliable connection, and a user who needs a decision, not a probability.",
                "I started from the budget — inference time, app size, and battery — then trained with quantisation in mind, verifying the exported graph against the same labelled sample rather than trusting the training split.",
                "The datasets are public, the model is portable, and the app treats uncertainty as a first-class state."
            ],
            "outcome": [
                "Published: 38-class plant disease dataset (2 GB) and 4-class rose foliage dataset",
                "TFLite export verified against a held-out sample after quantisation",
                "Offline-first Flutter flow with confidence-aware guidance"
            ],
            "link": { "label": "View published datasets", "href": SITE_ULTRALYTICS }
        }
    ])
}

pub fn project(slug: &str) -> Option<Value> {
    projects().as_array()?.iter().find(|item| item["slug"] == slug).cloned()
}

pub fn tool_groups() -> Value {
    json!([
        { "id": "interfaces", "number": "01", "title": "Interfaces that feel right", "subtitle": "Mobile & web" },
        { "id": "systems", "number": "02", "title": "Systems that hold up", "subtitle": "Backend & data" },
        { "id": "explore", "number": "03", "title": "Ideas worth exploring", "subtitle": "AI & developer tools" }
    ])
}

pub fn tools() -> Value {
    json!([
        { "name": "Flutter", "group": "interfaces", "description": "One codebase, carefully adapted to mobile and web. Flutter is at the heart of my product development.", "project": "onekitty" },
        { "name": "Dart", "group": "interfaces", "description": "The language behind my Flutter work, connecting interface logic, asynchronous operations, and application models.", "project": "onekitty" },
        { "name": "GetX", "group": "interfaces", "description": "State management, dependency injection, and navigation to keep complex application flows organised.", "project": "onechurch" },
        { "name": "Responsive UI", "group": "interfaces", "description": "Layouts that adapt to the space available, from a phone in one hand to a full desktop workspace.", "project": "onechurch" },
        { "name": "Go", "group": "systems", "description": "A focus for backend development: clear server logic, API design, and dependable services." },
        { "name": "Rust", "group": "systems", "description": "Where I put the parts that must not be wrong: typed validation, sanitised rendering, and the server behind this site." },
        { "name": "Firebase", "group": "systems", "description": "Cloud services used across my projects, including hosting Flutter web experiences.", "project": "onechurch" },
        { "name": "Supabase", "group": "systems", "description": "A relational approach to application data, combining PostgreSQL with backend services." },
        { "name": "REST APIs", "group": "systems", "description": "Connecting interfaces to real data, with attention to loading states, errors, and predictable data flows.", "project": "onekitty" },
        { "name": "Python", "group": "explore", "description": "A practical workspace for preparing datasets, training models, and evaluating results.", "project": "plant-disease-detector" },
        { "name": "TensorFlow", "group": "explore", "description": "Building and training image classification models for plant disease detection.", "project": "plant-disease-detector" },
        { "name": "TFLite", "group": "explore", "description": "Bringing trained models onto a device, connecting machine learning with a usable mobile experience.", "project": "plant-disease-detector" },
        { "name": "Git", "group": "explore", "description": "Version control for tracking changes and collaborating throughout the development process." }
    ])
}

pub fn principles() -> Value {
    json!([
        { "title": "Clarity over cleverness", "body": "If a screen needs a paragraph to explain, the design is not finished. Interfaces should teach themselves." },
        { "title": "Respect the network", "body": "Loading, empty, and failed are design states, not error handling afterthoughts. Every request gets a visible answer." },
        { "title": "Craft is a requirement", "body": "Spacing, focus rings, and 44-pixel targets are how a product shows it respects the person holding it." },
        { "title": "Secure by default", "body": "Secrets stay server-side, untrusted text is rendered safely, and every mutation is validated twice." }
    ])
}

pub fn milestones() -> Value {
    json!([
        { "marker": "2024", "title": "B.Sc. Computer Science", "detail": "Gretsa University, Thika" },
        { "marker": "01", "title": "First place · Mobile Development", "detail": "Gretsa National Programming Contest, March 2024" },
        { "marker": "Now", "title": "Product engineering", "detail": "Flutter, Rust, and on-device ML for real users" }
    ])
}

pub fn topics() -> Vec<&'static str> {
    vec!["New project", "Collaboration", "Speaking / writing", "Hiring", "Something else"]
}
