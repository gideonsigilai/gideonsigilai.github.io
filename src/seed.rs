use crate::content::{has_any_post, insert_post, now_iso, store_admin_hash, stored_admin_hash, Draft};
use crate::state::Config;
use crate::text::{normalise_headings, reading_minutes, render_markdown};
use sqlx::SqlitePool;

/// Hashes `ADMIN_PASSWORD` into the database on first boot. Once a hash exists,
/// the plaintext variable is ignored, so it can be removed from the environment.
pub async fn bootstrap_admin(db: &SqlitePool, config: &Config) -> Result<(), Box<dyn std::error::Error>> {
    if config.admin_password_hash.is_some() {
        return Ok(());
    }
    if stored_admin_hash(db).await.is_some() {
        return Ok(());
    }
    let Some(password) = &config.admin_password else {
        tracing::warn!("ADMIN_PASSWORD not set — the dashboard stays locked until it is.");
        return Ok(());
    };
    if password.chars().count() < 12 {
        tracing::warn!("ADMIN_PASSWORD is short; use at least 12 characters.");
    }
    use argon2::{
        password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
        Argon2,
    };
    let salt = SaltString::generate(&mut OsRng);
    let hash = Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map_err(|err| format!("hash admin password: {err}"))?
        .to_string();
    store_admin_hash(db, &hash).await?;
    tracing::info!("admin password hash stored");
    let _ = now_iso();
    Ok(())
}

struct Seed {
    slug: &'static str,
    title: &'static str,
    excerpt: &'static str,
    tags: &'static [&'static str],
    cover_note: &'static str,
    cover_image: &'static str,
    published_at: &'static str,
    markdown: &'static str,
}

const SEEDS: &[Seed] = &[
    Seed {
        slug: "state-that-does-not-fight-you",
        title: "State that does not fight you",
        excerpt: "A practical way to keep Flutter screens honest: one source of truth, sealed UI state, and tests that describe behaviour instead of widgets.",
        tags: &["Flutter", "Architecture"],
        cover_note: "Notes from shipping OneKitty and OneChurch.",
        cover_image: "/images/flutter-cover.png",
        published_at: "2026-09-18T07:00:00Z",
        markdown: r#"Most Flutter codebases do not break because of a missing animation. They break because three layers disagree about what is true.

## The symptom

A screen shows a loading spinner while the data is already there. A button submits twice because the guard lives in the widget. A logged-out user briefly sees a balance. None of these are exotic bugs; they are all the same bug wearing different clothes: **state with more than one owner**.

## Pick one owner

In OneKitty I kept a single controller per feature slice. The widget layer reads, the controller decides, the repository fetches. If something needs to change, it travels one way:

```dart
abstract class WalletState {
  const WalletState();
}

class WalletLoading extends WalletState {
  const WalletLoading();
}

class WalletReady extends WalletState {
  const WalletReady(this.balance, this.entries);
  final int balance;
  final List<Entry> entries;
}

class WalletFailed extends WalletState {
  const WalletFailed(this.message);
  final String message;
}
```

Sealed states are boring on purpose. The compiler forces every `switch` to answer every case, so an unhandled failure is a build error instead of a blank screen in Nairobi traffic.

## Guard actions in the controller, not the widget

A double submit is not a UI problem. It is a missing invariant.

```dart
Future<void> send() async {
  if (_state is! WalletReady || _sending) return;
  _sending = true;
  try {
    await repository.send(_amount);
    emit(repository.fetch()); // reload from the source of truth
  } finally {
    _sending = false;
  }
}
```

The button can still be disabled for feedback, but correctness does not depend on it.

## Test behaviour, not trees

Widget tests that assert on `find.byType` break every redesign. Tests that assert on what the user can do survive:

```dart
testWidgets('a pending contribution blocks a second one', (tester) async {
  await tester.pumpWidget(App(fixture: delayedRepository()));
  await tester.tap(find.text('Contribute'));
  await tester.pump();
  expect(find.text('Sending…'), findsOneWidget);
  expect(find.text('Contribute'), findsNothing);
});
```

## What I look for in review

- Every screen has exactly one state object.
- Loading, ready, empty, and failed are all representable.
- Mutations re-read from the source of truth instead of patching local copies.
- Errors are values with user-facing copy, not raw exceptions rendered into text.

That is the whole trick. Nothing clever — just making it impossible for the app to be in a state you never designed.
"#,
    },
    Seed {
        slug: "plant-disease-on-device",
        title: "Plant disease detection, on the device",
        excerpt: "Why an on-device image classifier beats a cloud round trip for a farmer in a low-signal area — and the export pipeline that makes it small enough to ship.",
        tags: &["Machine Learning", "Flutter", "TFLite"],
        cover_note: "TensorFlow to TFLite, 38 crop classes, no internet required.",
        cover_image: "/images/plant_doc_showcase.png",
        published_at: "2026-08-02T09:30:00Z",
        markdown: r#"A farmer standing in a field with two bars of signal does not care about your server's GPU. The interesting constraint in the Plant Disease Detector was never accuracy on a benchmark — it was latency, privacy, and offline behaviour.

## Start from the deployment target

Before training, I wrote down the budget: the model has to run in well under a second on a mid-range Android phone, fit inside a normal app download, and never upload a photo to a server.

- Input resolution: small enough for real-time inference.
- Quantisation: post-training int8 where accuracy held.
- Class set: the 38 PlantVillage classes, grouped so an ambiguous prediction can still say "likely fungal".

## Export, then verify the export

The model that wins on a validation split is not always the model that ships. A quantised model has different numerical behaviour, so compare the two:

```python
interpreter = tf.lite.Interpreter(model_path="model.tflite")
interpreter.allocate_tensors()
# Compare argmax on a labelled sample of 200 images,
# not on the training set you already overfit to.
```

If quantisation moves accuracy by more than a point, narrow the input range or keep float16 for the last layers instead of giving up the speed.

## The app is the product

The Flutter side gets the interesting work: camera capture with a crop guide, a queue for offline analysis, and a result screen that leads with a *next action* instead of a label.

- Confidence under 70%? Ask for a second photo rather than guessing.
- The advice copy matters more than the percentage.
- Store scans locally with a timestamp so a farmer can compare over a season.

## What I would do next

- On-device fine-tuning on the farm's own leaves.
- A disease-class weighting loss so the rare classes are not drowned by healthy leaf images.
- A tiny language model for local-language guidance, generated from a reviewed template.

The lesson generalises: for a used product, the optimiser is the user's environment, not the leaderboard.
"#,
    },
    Seed {
        slug: "the-browser-should-not-hold-the-keys",
        title: "The browser should not hold the keys",
        excerpt: "How this site works: one Rust binary serving the pages, argon2 sessions in httpOnly cookies, double-submit CSRF, and markdown sanitised before it is stored.",
        tags: &["Rust", "Security"],
        cover_note: "The architecture behind this portfolio.",
        cover_image: "/images/rust-cover.png",
        published_at: "2026-10-21T06:15:00Z",
        markdown: r#"Every form on a public site is an invitation. This one accepts messages, so I built it the way I would build a client project: everything that matters happens on the server.

## One process, no public API

The site, the admin dashboard, and the data all live in a single Rust binary. Pages are rendered as HTML on the server, so there is no client-side data fetching to secure and no API surface exposed to the internet.

```text
browser ──(same-origin, httpOnly cookie)──► Rust server ──► SQLite
```

There is no CORS configuration to get wrong because there is nothing cross-origin to talk to.

## Sessions JavaScript cannot read

Sign-in produces a random token stored in an httpOnly, SameSite=Strict cookie. The client bundle never sees it. Mutations also need a CSRF value echoed from a second cookie, checked in constant time against the session on the server.

```rust
// Double submit, compared without an early exit.
let ok = bool::from(session.csrf.as_bytes().ct_eq(supplied.as_bytes()));
```

Passwords are hashed with argon2 on first boot and the plaintext is never stored. Login attempts are rate limited more tightly than the contact form, because guessing a password is a more expensive failure than one extra message.

## Rendering untrusted input

Post bodies are markdown written by an authenticated owner, but they still go through `pulldown-cmark` and then `ammonia`. Nothing to gain from trusting your own database, everything to lose if an import path ever changes.

- Raw HTML in markdown is stripped.
- Script URLs never survive sanitisation.
- Every rendered page carries a restrictive content security policy.

## What this buys

- No keys in the browser.
- No public API to probe.
- Injected content cannot execute scripts.
- One deployable file: copy it next to a SQLite database and run it.

The pattern is not novel; it is just rarely finished. Most tutorials stop at "and then we fetch the data". The work is in the parts that fail quietly — constant-time compares, sanitised output, and cookies the client cannot touch.
"#,
    },
];

/// Seeds the three starter essays only when the posts table is empty, so an
/// operator who deletes them does not get them back on the next restart.
pub async fn seed_posts(db: &SqlitePool) -> Result<(), Box<dyn std::error::Error>> {
    if has_any_post(db).await {
        return Ok(());
    }
    for seed in SEEDS {
        let html = normalise_headings(&render_markdown(seed.markdown), seed.title);
        insert_post(
            db,
            Draft {
                title: seed.title.to_string(),
                slug: seed.slug.to_string(),
                excerpt: seed.excerpt.to_string(),
                markdown: seed.markdown.to_string(),
                html,
                tags_json: serde_json::to_string(seed.tags).unwrap_or_else(|_| "[]".into()),
                cover_note: seed.cover_note.to_string(),
                cover_image: seed.cover_image.to_string(),
                status: "published".to_string(),
                reading_min: reading_minutes(seed.markdown),
            },
        )
        .await?;
        // Backdate so the feed reads like a history rather than a bulk import.
        sqlx::query("UPDATE posts SET published_at = ?, created_at = ?, updated_at = ? WHERE slug = ?")
            .bind(seed.published_at)
            .bind(seed.published_at)
            .bind(seed.published_at)
            .bind(seed.slug)
            .execute(db)
            .await?;
    }
    tracing::info!("seeded starter posts");
    Ok(())
}
