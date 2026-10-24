//! **The published crate carries code, not demo media.**
//!
//! crates.io rejects uploads over 10 MiB. The 0.1.0 publish failed on exactly
//! that (`413 Payload Too Large`), because `demos/` and the generated widget
//! screenshots rode along. `exclude` in Cargo.toml fixes it; this keeps a new
//! image directory from undoing it.

use std::process::Command;

#[test]
fn the_package_ships_no_images() {
    let out = Command::new(env!("CARGO"))
        .args(["package", "--list", "--allow-dirty"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("run cargo package --list");
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let list = String::from_utf8(out.stdout).expect("utf-8 file list");
    // Anti-vacuous: an empty listing would pass the image check.
    assert!(
        list.lines().any(|f| f == "src/lib.rs"),
        "listing is missing src/lib.rs:\n{list}"
    );
    let images: Vec<_> = list
        .lines()
        .filter(|f| [".png", ".gif", ".apng"].iter().any(|ext| f.ends_with(ext)))
        .collect();
    assert!(
        images.is_empty(),
        "images would ship in the crate: {images:?}"
    );
}
