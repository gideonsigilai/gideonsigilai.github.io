// Inlines the two web apps before the crate is compiled.
//
// `web/build.ts` runs Bun over `web/portfolio.html` and `web/admin.html` and
// writes two documents, each holding its own stylesheet, client script, and the
// icon, with the mustache placeholders from the matching shell left intact:
//
//   $OUT_DIR/portfolio.html  templates/layout.html       → port 8050
//   $OUT_DIR/admin.html      templates/admin_layout.html → port 8051
//
// `src/views/mod.rs` embeds both, so neither server issues a stylesheet, script,
// or favicon request. Nothing generated is checked in.

use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let out_dir = PathBuf::from(std::env::var_os("OUT_DIR").expect("OUT_DIR is always set"));

    for input in [
        "web/build.ts",
        "web/portfolio.html",
        "web/admin.html",
        "templates/layout.html",
        "templates/admin_layout.html",
        "templates/admin_analytics.html",
        "static/base.css",
        "static/site.css",
        "static/detail.css",
        "static/admin.css",
        "static/core.js",
        "static/site.js",
        "static/icon.svg",
        "static/gideon-sigilai.png",
    ] {
        println!("cargo:rerun-if-changed={input}");
    }

    let bun = Command::new("bun")
        .arg("run")
        .arg(root.join("web/build.ts"))
        .arg(&out_dir)
        .current_dir(root)
        .output();

    match bun {
        Ok(result) if result.status.success() => {
            print!("{}", String::from_utf8_lossy(&result.stdout));
        }
        Ok(result) => {
            eprintln!("{}", String::from_utf8_lossy(&result.stderr));
            panic!("bun run web/build.ts failed: {}", result.status);
        }
        Err(error) => panic!(
            "bun is required to build the web apps (install from https://bun.sh): {error}"
        ),
    }

    for document in ["portfolio.html", "admin.html"] {
        let path = out_dir.join(document);
        if !path.is_file() {
            panic!(
                "web/build.ts reported success but wrote no {}",
                path.display()
            );
        }
    }
}
