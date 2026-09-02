//! **The leaf invariant: this crate's shipped dependency closure is EMPTY.**
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
//!
//! # Two halves, and they check different things
//!
//! **The closure walk** ([`the_shipped_closure_is_empty`]) asks cargo what a
//! consumer actually RESOLVES, which is the claim consumers consume. It reads
//! `cargo tree`, so feature resolution is cargo's, not a reimplementation of
//! cargo's here. It used to be `cargo metadata --no-deps`, which suppresses
//! the resolve graph entirely and reports only this crate's own declared
//! `[dependencies]` — the manifest, not the closure, while every doc comment
//! said "closure".
//!
//! **The manifest read** ([`every_runtime_dependency_is_optional`]) forbids
//! ADDITION rather than a specific outcome, which is what keeps the default
//! feature set honest even at a moment when it happens to resolve to nothing.
//!
//! # The anti-vacuous twin
//!
//! Every assertion here has the shape "this list is empty", which is exactly
//! what a reader that reads NOTHING satisfies. So the walker is also pointed
//! at a feature setting whose closure is known to be large, and must come back
//! naming crates in it. Without that, deleting the body of `closure` would
//! turn this file green.

use std::collections::BTreeSet;
use std::process::Command;

/// The transitive SHIPPED-dependency closure under `args`, by name.
///
/// Normal **and** build dependencies count. A build script runs with the full
/// authority of the building user — reading the filesystem, spawning
/// processes, reaching the network — before a line of this crate compiles, so
/// a dependency arriving through `[build-dependencies]` is not a technicality:
/// it is the guard's whole subject coming in another door. Only `dev` is
/// excluded, because dev-dependencies impose nothing on a consumer.
fn closure(args: &[&str]) -> BTreeSet<String> {
    let output = Command::new(env!("CARGO"))
        .args(["tree", "--edges", "normal,build", "--prefix", "none"])
        .args(["--format", "{p}"])
        .args(args)
        .output()
        .expect("cargo tree runs");
    assert!(
        output.status.success(),
        "cargo tree failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| line.split_whitespace().next())
        // Section headers, and this crate itself.
        .filter(|name| !name.starts_with('[') && *name != "newtui")
        .map(str::to_string)
        .collect()
}

/// **Nothing ships, at the two settings a consumer can get without asking.**
///
/// `--no-default-features` is the headless configuration. The bare resolve is
/// what `newtui = "0.1"` means in someone else's manifest, and it is the
/// higher-blast-radius half: moving one string into `default` ships a whole
/// terminal stack to everyone without adding a dependency anywhere.
#[test]
fn the_shipped_closure_is_empty() {
    for (setting, args) in [
        ("--no-default-features", &["--no-default-features"][..]),
        (
            "the default features (what `newtui = \"0.1\"` resolves)",
            &[],
        ),
    ] {
        let shipped = closure(args);
        assert!(
            shipped.is_empty(),
            "at {setting} this crate resolves {shipped:?}. The core — Component, \
             Flow, View, the alphabet, the fingerprint, the explorer — must \
             depend on NOTHING, so a headless consumer pays for the harness and \
             nothing else. If the dependency is real, put it behind a \
             NON-DEFAULT feature."
        );
    }
}

/// **ANTI-VACUOUS TWIN.** The same walker, pointed at the one feature that
/// does pull something in, must come back naming it — and naming its terminal
/// backend, which is the thing the leaf claim exists to keep out of a headless
/// consumer's build.
///
/// An empty result from a walker that always returns empty proves nothing, and
/// "the closure is empty" is precisely the claim a broken reader satisfies for
/// free.
#[test]
fn the_walker_sees_the_dependencies_that_do_exist() {
    let rendering = closure(&["--no-default-features", "--features", "ratatui"]);
    for expected in ["ratatui", "crossterm"] {
        assert!(
            rendering.contains(expected),
            "the `ratatui` closure is missing {expected:?}: {rendering:?}. The \
             walker is not reading the dependency graph, so the emptiness \
             assertions above are vacuous."
        );
    }
}

/// Every runtime dependency is OPTIONAL. A non-optional entry ships to every
/// consumer including the `--no-default-features` one, and would end the
/// empty-closure claim before any feature was even enabled.
///
/// This forbids ADDITION, where the closure walk forbids an outcome: the
/// manifest is pinned even at a moment when a new entry happens to resolve to
/// nothing.
#[test]
fn every_runtime_dependency_is_optional() {
    let package = manifest();

    let mut shipped = Vec::new();
    for dep in package["dependencies"].as_array().expect("dependencies") {
        let kind = dep["kind"].as_str().unwrap_or("normal");
        // Dev-dependencies ship to nobody: they are absent from every
        // consumer's resolved closure. BUILD dependencies are not in that
        // company — they run, with the building user's authority, in every
        // consumer's build — and skipping them here is a hole this used to
        // have, in the same clause, under a comment that was true of `dev`
        // alone.
        if kind == "dev" {
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
///
/// Direct dependencies only, and that is adequate rather than lazy: crates.io
/// refuses to publish a crate carrying a path or git dependency, so a
/// transitive one cannot arrive through a registry edge.
#[test]
fn nothing_comes_from_a_path_or_a_git_url() {
    let package = manifest();

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

/// This crate's own manifest, as cargo parses it.
///
/// `--no-deps`, and no feature flags: it returns the DECLARED dependency table,
/// which is the same array at every feature setting. Reading the resolved
/// closure is the other half's job, and it is a different question.
fn manifest() -> serde_json::Value {
    let output = Command::new(env!("CARGO"))
        .args(["metadata", "--format-version", "1", "--no-deps"])
        .output()
        .expect("cargo metadata runs");
    assert!(
        output.status.success(),
        "cargo metadata failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let meta = String::from_utf8(output.stdout).expect("metadata is UTF-8");
    let value: serde_json::Value = serde_json::from_str(&meta).expect("metadata parses");
    value["packages"]
        .as_array()
        .expect("packages")
        .iter()
        .find(|p| p["name"] == "newtui")
        .expect("this crate is in its own metadata")
        .clone()
}
