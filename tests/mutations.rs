//! **Every guard here has a defect it is known to catch.**
//!
//! A guard is a claim about a defect that is not present. That claim is exactly
//! the shape a broken guard satisfies for free — the whole false-completeness
//! class this crate exists to refuse, living in the crate's own tests. Prose
//! mutation tables (a review's, a design doc's, a PR body's) do not close it:
//! nobody re-runs a table, and the one bug this crate has already shipped
//! landed ONE COMMIT after the fix that created the flag it broke.
//!
//! So: each entry below is a real edit to a real source file, applied to a
//! copy of this crate, with the name of the test that must go RED. If the
//! mutation stops applying (the code moved) or the test stops failing (the
//! guard rotted), this runner fails and says which.
//!
//! **Adding a guard? Add its mutation here.** That is the acceptance criterion
//! for a new guard in this crate, not a nice-to-have: a guard with no entry is
//! a claim nobody has ever seen fail.
//!
//! Not a mutation *testing* tool. It does not generate mutants or measure a
//! score; it pins the handful a human argued about, which is the part a
//! generated score cannot tell you.
//!
//! Excluded from `cargo test` by `test = false` in `Cargo.toml` (it shells out
//! to cargo, so running it inside every ordinary test run would nest builds).
//! `just mutations` runs it, and `just check` includes that.

use std::path::{Path, PathBuf};
use std::process::Command;

/// One defect, and the guard that must notice it.
struct Mutation {
    /// The defect, in the words a PR would use.
    defect: &'static str,
    /// File to edit, relative to the crate root.
    file: &'static str,
    /// Text to replace. Must occur EXACTLY ONCE — a stale needle that matches
    /// nothing would make the mutant identical to the original, and a guard
    /// that goes green on an unmutated copy proves nothing.
    from: &'static str,
    /// What to put there instead.
    to: &'static str,
    /// The test that must FAIL, as `cargo test` prints it.
    expect_red: &'static str,
}

/// The table. One row per guard in this crate.
const MUTATIONS: &[Mutation] = &[Mutation {
    defect: "the end-of-search assignment overwrites a depth truncation \
                 (the bug fixed in 5307cd7)",
    file: "src/explore.rs",
    from: "report.exhausted &= queue.is_empty()",
    to: "report.exhausted = queue.is_empty()",
    expect_red: "tests::a_depth_capped_search_is_a_sample_and_admits_it",
}];

#[test]
fn every_guard_has_a_defect_it_provably_catches() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let arena = root.join("target/mutations");
    // One target directory for every mutant: the dependency builds are shared,
    // and it is not the outer `cargo test`'s, so the two never contend for a
    // build lock.
    let shared_target = arena.join(".target");

    let mut checked = 0;
    for mutation in MUTATIONS {
        let mutant = arena.join(mutation.expect_red.replace("::", "-"));
        let _ = std::fs::remove_dir_all(&mutant);
        copy_crate(&root, &mutant);

        let path = mutant.join(mutation.file);
        let original = std::fs::read_to_string(&path).expect("the file to mutate is readable");
        assert_eq!(
            original.matches(mutation.from).count(),
            1,
            "the mutation for `{}` does not apply to {} any more — it matched \
             {} times, not once. The code moved; move the needle with it, \
             because a mutant identical to the original goes green for free.",
            mutation.defect,
            mutation.file,
            original.matches(mutation.from).count(),
        );
        std::fs::write(&path, original.replacen(mutation.from, mutation.to, 1))
            .expect("the mutant is writable");

        // `--no-default-features` for every mutant: nothing in `src/` is
        // feature-gated, and it keeps the default-feature mutation from
        // compiling a whole terminal backend to run a test that shells out to
        // `cargo tree` anyway.
        let output = Command::new(env!("CARGO"))
            .args(["test", "--no-default-features", mutation.expect_red])
            .current_dir(&mutant)
            .env("CARGO_TARGET_DIR", &shared_target)
            .output()
            .expect("cargo test runs in the mutant");
        let printed = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );

        // A filter that matches no test runs zero tests and exits 0, so this
        // also catches a renamed guard.
        assert!(
            !output.status.success(),
            "`{}` is GREEN under this defect:\n  {}\nIt does not catch what it \
             claims to. Either the guard is decoration or the mutation is \
             wrong.\n\n{printed}",
            mutation.expect_red,
            mutation.defect,
        );
        // Non-zero is not enough: a compile error is also non-zero, and would
        // pass a red for a green in disguise.
        assert!(
            printed.contains(&format!("{} ... FAILED", mutation.expect_red)),
            "the mutant for `{}` failed, but not by failing `{}` — so what went \
             red is not the guard:\n\n{printed}",
            mutation.defect,
            mutation.expect_red,
        );
        checked += 1;
        std::fs::remove_dir_all(&mutant).expect("the mutant is removable");
    }

    assert!(
        checked >= 1,
        "only {checked} mutations ran. This table may only GROW: a guard \
         removed from it is a guard nobody has seen fail."
    );
}

/// Copy the crate into `dest` — sources, manifest and lockfile, and nothing
/// else. Explicitly not `target/` (recursive, enormous) and not this file (a
/// mutant that ran its own mutation runner would nest without end).
fn copy_crate(root: &Path, dest: &Path) {
    std::fs::create_dir_all(dest.join("src")).expect("the mutant arena is writable");
    std::fs::create_dir_all(dest.join("tests")).expect("the mutant arena is writable");
    for file in ["Cargo.toml", "Cargo.lock", "README.md", "tests/leaf.rs"] {
        std::fs::copy(root.join(file), dest.join(file)).expect("crate file is copyable");
    }
    for entry in std::fs::read_dir(root.join("src")).expect("src/ is readable") {
        let entry = entry.expect("a src/ entry");
        std::fs::copy(entry.path(), dest.join("src").join(entry.file_name()))
            .expect("a source file is copyable");
    }
}
