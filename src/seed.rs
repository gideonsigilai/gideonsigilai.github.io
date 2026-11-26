use crate::content::{insert_post, now_iso, store_admin_hash, stored_admin_hash, Draft};
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
    Seed {
        slug: "offline-first-is-a-design-decision",
        title: "Offline-first is a design decision, not a feature",
        excerpt: "Why treating offline as a mode you bolt on later guarantees a broken experience - and how to shape the data model so the network is just another cache.",
        tags: &["Mobile", "Architecture", "Flutter"],
        cover_note: "Lessons from shipping apps in low-signal areas.",
        cover_image: "/images/flutter-cover.png",
        published_at: "2026-07-12T08:00:00Z",
        markdown: r#"Offline is not a toggle. It is a statement about what your app believes is true when nobody is watching. Most teams discover this the hard way: they ship a feature, then get a bug report that says "it works on wifi, it spins forever on the bus".

## The network is a cache, not a dependency

If a screen cannot render without a successful request, the network is a dependency. If it can render from local data and refresh in the background, the network is a cache. The difference shows up in three places.

* **First paint**: can the user do anything before the first byte arrives?
* **Failure mode**: does a dropped connection look like an error or like "slightly stale"?
* **Conflict resolution**: when two writes disagree, who wins?

Design the second and third before you write the first screen. Retrofitting them is a rewrite.

## Model the queue, not the request

```dart
abstract class SyncState {
  const SyncState();
}

class Synced extends SyncState {
  const Synced(this.at);
  final DateTime at;
}

class Pending extends SyncState {
  const Pending(this.operation);
  final Operation operation;
}

class Failed extends SyncState {
  const Failed(this.operation, this.reason);
  final Operation operation;
  final String reason;
}
```

Every mutation becomes an operation in a queue. The queue is the source of truth for "what the user asked for"; the server is the source of truth for "what actually happened". Reconcile them on reconnect.

## Three rules I keep

1. Read from local storage, always. The network only updates local storage.
2. Writes append to a queue; the queue drains when online. Never block the UI on a write.
3. Show staleness honestly. "Last updated 2 hours ago" is a feature, not an apology.

## What changes in review

* No `await api.get()` in a widget's build path.
* Every write has a retry policy and a visible pending state.
* Conflicts resolve to a user-visible choice, not a silent overwrite.

The bus is a real place. Design for it.
"#,
    },
    Seed {
        slug: "designing-for-low-bandwidth",
        title: "Designing for low bandwidth",
        excerpt: "Practical choices for building interfaces that stay usable on slow, expensive, or intermittent connections - from Nairobi to anywhere the signal drops.",
        tags: &["Design", "Performance", "Mobile"],
        cover_note: "Field notes from shipping in Nairobi.",
        cover_image: "/images/plant_doc_showcase.png",
        published_at: "2026-06-20T09:00:00Z",
        markdown: r#"I build in Nairobi, where a "fast" connection is a luxury and a cheap one is a lottery. That constraint has made me a better designer than any style guide.

## Weight is a feature

Every kilobyte is a tax on someone's data bundle. The discipline is simple: measure the page, then cut it.

* Images: serve the right size, lazy-load below the fold, prefer SVG for icons.
* Fonts: subset to the glyphs you use, ship at most two weights.
* JavaScript: ship none if the server can render it. This site does exactly that.

## Progressive by default

A page should be useful before it is pretty. Order the markup so the meaningful content arrives first and the decoration loads later, if at all.

```html
<!-- The article text is the first thing in the document. -->
<article>…</article>
<!-- Enhancements load after, and the page still works without them. -->
<script src="/static/site.js" defer></script>
```

If the JavaScript fails, the user still reads the article. If the CSS fails, the user still reads the article. If the image fails, the user still reads the article. The article is the product.

## Honest loading states

Spinners lie. They say "working" when the request may have already failed. Prefer:

* Skeletons that match the real layout, so the jump on load is small.
* A timeout with a retry button, not an infinite spinner.
* Cached data shown immediately, with a subtle "updating…" indicator.

## Test on a throttled phone

Chrome DevTools throttling is a start. A real 3G connection on a mid-range Android is the truth. I keep a cheap device on my desk and test every release on it. If it feels slow there, it is slow.

The web is for everyone. Build like it.
"#,
    },
    Seed {
        slug: "a-checklist-for-shipping-flutter",
        title: "A checklist for shipping Flutter to production",
        excerpt: "The boring, unglamorous list I run through before a Flutter app goes to real users - from signing to crash reporting to the first-week monitoring plan.",
        tags: &["Flutter", "Release", "DevOps"],
        cover_note: "From `flutter build` to a store listing and beyond.",
        cover_image: "/images/flutter-logo.png",
        published_at: "2026-05-08T07:30:00Z",
        markdown: r#"Shipping is not `flutter build apk`. It is everything around that command. Here is the list I actually use.

## Before the build

* `flutter analyze` is clean. Not "mostly clean". Clean.
* `flutter test` passes on a cold checkout.
* Version code incremented. Both `version` and `build` in `pubspec.yaml`.
* Release signing configured and verified with `keytool -list`.
* ProGuard/R8 rules tested on a release build, not just debug.

## The build itself

```bash
flutter build appbundle --release --obfuscate --split-debug-info=debug-info/
```

Keep the `debug-info/` directory. You will need it the first time a crash report arrives and the stack trace is useless without symbols.

## Store listing

* Screenshots on real devices, not simulators. Simulators lie about notches.
* A short description that says what the app does, not what it is.
* Privacy policy URL that actually resolves.
* Data safety form filled honestly. Reviewers check.

## First week

* Crash reporting wired and verified with a deliberate test crash in a beta build.
* Analytics on the three events that matter: install, first meaningful action, return visit.
* A channel for feedback that a human reads. Not a form that emails nobody.
* A rollback plan. For Flutter that means a previous build uploaded and ready.

## After the first week

Read the crash reports. Read the reviews. Fix the top three issues before adding anything new. The first release is a hypothesis; the second release is the answer.

Shipping is a habit, not an event.
"#,
    },
];

/// Seeds starter essays. Idempotent by slug: a post the operator edited or
/// deleted is not re-inserted, but new starter essays are added on upgrade.
pub async fn seed_posts(db: &SqlitePool) -> Result<(), Box<dyn std::error::Error>> {
    for seed in SEEDS {
        if crate::content::published_by_slug(db, seed.slug).await.ok().flatten().is_some() {
            continue;
        }
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
    tracing::info!(count = SEEDS.len(), "seeded starter posts");
    Ok(())
}
