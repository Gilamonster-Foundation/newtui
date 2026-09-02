//! **The leaf invariant: this crate's default dependency closure is EMPTY.**
//!
//! `newtui` is meant to be pulled into newt, wyvern, gilamonster, and any
//! foreign TUI that wants the components without newt's release train — the
//! same standing `precedence-ladder` holds in this line. That only works while
//! it stays a leaf. The moment a renderer, a terminal backend, or a data
//! library becomes non-optional, every consumer inherits it, including the
//! headless ones that drive components and never draw a frame.
//!
//! Asserted rather than documented, because a comment claiming an empty
//! closure goes stale the first time someone adds a convenience dependency.

use std::process::Command;

fn metadata(args: &[&str]) -> String {
    let output = Command::new(env!("CARGO"))
        .args(["metadata", "--format-version", "1", "--no-deps"])
        .args(args)
        .output()
        .expect("cargo metadata runs");
    assert!(
        output.status.success(),
        "cargo metadata failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).expect("metadata is UTF-8")
}

/// Every runtime dependency is OPTIONAL. A non-optional entry ships to every
/// consumer including the `--no-default-features` one, and would end the
/// empty-closure claim before any feature was even enabled.
#[test]
fn every_runtime_dependency_is_optional() {
    let meta = metadata(&[]);
    let value: serde_json::Value = serde_json::from_str(&meta).expect("metadata parses");
    let package = value["packages"]
        .as_array()
        .expect("packages")
        .iter()
        .find(|p| p["name"] == "newtui")
        .expect("this crate is in its own metadata");

    let mut shipped = Vec::new();
    for dep in package["dependencies"].as_array().expect("dependencies") {
        let kind = dep["kind"].as_str().unwrap_or("normal");
        // Dev-dependencies ship to nobody: they are absent from every
        // consumer's resolved closure.
        if kind == "dev" || kind == "build" {
            continue;
        }
        if dep["optional"] != serde_json::Value::Bool(true) {
            shipped.push(dep["name"].as_str().unwrap_or("?").to_string());
        }
    }
    assert!(
        shipped.is_empty(),
        "these dependencies ship to EVERY consumer, ending the leaf claim: {shipped:?}"
    );
}

/// **No path or git dependencies, at any feature setting.**
///
/// A path or git edge either fails to build from a registry or drags an
/// unpublishable cycle into a downstream repo — and it can swap an
/// implementation under a consumer without a version bump.
#[test]
fn nothing_comes_from_a_path_or_a_git_url() {
    let meta = metadata(&["--all-features"]);
    let value: serde_json::Value = serde_json::from_str(&meta).expect("metadata parses");
    let package = value["packages"]
        .as_array()
        .expect("packages")
        .iter()
        .find(|p| p["name"] == "newtui")
        .expect("this crate is in its own metadata");

    for dep in package["dependencies"].as_array().expect("dependencies") {
        let name = dep["name"].as_str().unwrap_or("?");
        assert!(
            dep["path"].is_null(),
            "`{name}` is a PATH dependency; a consumer resolving from a registry \
             cannot build this crate"
        );
        let source = dep["source"].as_str().unwrap_or("");
        assert!(
            !source.starts_with("git+"),
            "`{name}` comes from git ({source}); pin it to a published version \
             instead"
        );
    }
}
