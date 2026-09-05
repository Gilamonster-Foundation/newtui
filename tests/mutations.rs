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
//! **Adding a guard? Mark it, and add its mutation here.** A guard declares
//! itself with `// GUARD: <the filter cargo test takes>` above its
//! declaration, and `every_registered_guard_is_pinned_by_a_mutation` requires
//! the marked set and the `expect_red` set to MATCH — every registered guard
//! has a mutation, every mutation names a registered guard, both directions
//! failing the build. That replaced a `checked >= 10` count, which closed
//! neither: an eleventh guard with no mutation stayed green, and so did a
//! deleted row.
//!
//! Not a mutation *testing* tool. It does not generate mutants or measure a
//! score; it pins the handful a human argued about, which is the part a
//! generated score cannot tell you.
//!
//! Excluded from `cargo test` by `test = false` in `Cargo.toml` (it shells out
//! to cargo, so running it inside every ordinary test run would nest builds).
//! `just mutations` runs it, and `just check` includes that.

use std::collections::{BTreeMap, BTreeSet};
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
    /// Extra cargo arguments. `--doc` for a doctest guard: `cargo test
    /// <filter>` SKIPS doctests entirely, silently, which would hand this
    /// runner a green from a section that never ran — the class again, in the
    /// tool built to pin the class.
    cargo_args: &'static [&'static str],
}

/// The table. At least one row per guard in this crate — a guard may have
/// several, when there is more than one way to break the thing it holds.
const MUTATIONS: &[Mutation] = &[
    Mutation {
        defect: "the end-of-search assignment overwrites a depth truncation \
                 (the bug fixed in 5307cd7)",
        file: "src/explore.rs",
        from: "report.exhausted &= queue.is_empty()",
        to: "report.exhausted = queue.is_empty()",
        expect_red: "tests::a_depth_capped_search_is_a_sample_and_admits_it",
        cargo_args: &[],
    },
    Mutation {
        defect: "the fingerprint's identity is a FLATTENED STRING again — the \
                 structural boundary between a view and a caller's seed \
                 removed, so a separator that is content merges two states \
                 and the walk drops everything reachable only through the \
                 second",
        file: "src/component.rs",
        from: "        Self {
            base: FingerprintBase::View(view.clone()),
            extra: Vec::new(),
        }",
        to: "        Self::of(Fingerprint {
            base: FingerprintBase::View(view.clone()),
            extra: Vec::new(),
        })",
        expect_red: "tests::a_state_the_fingerprint_merges_is_a_state_the_walk_drops",
        cargo_args: &[],
    },
    Mutation {
        defect: "`and` is an UNFRAMED ACCUMULATOR again — folds run together, \
                 so `.and(\"y\").and(\"z\")` is the same state as \
                 `.and(\"y\\u{1d}z\")`",
        file: "src/component.rs",
        from: "        self.extra.push(more.to_string());",
        to: "        if let Some(last) = self.extra.last_mut() {
            last.push('\\u{1d}');
            last.push_str(&more.to_string());
        } else {
            self.extra.push(more.to_string());
        }",
        expect_red: "component::tests::structural_fingerprints_keep_their_boundaries",
        cargo_args: &[],
    },
    Mutation {
        defect: "a closing state's view is dropped from the corpus, which then \
                 calls itself complete (bug 3a — what the deleted second walk \
                 shipped)",
        file: "src/explore.rs",
        from: "if recorded.insert(after.clone()) {
                        report.views.push(after.clone());
                    }
                    report.terminal_states += 1;",
        to: "report.terminal_states += 1;",
        expect_red: "tests::the_corpus_holds_the_states_that_closed",
        cargo_args: &[],
    },
    Mutation {
        defect: "the verdict stops reading `exhausted`, so a capped run is \
                 Clean — bug 3c's weak assertion, back inside the crate",
        file: "src/explore.rs",
        from: "if !self.exhausted {",
        to: "if false {",
        expect_red: "tests::a_capped_search_is_a_sample_and_admits_it",
        cargo_args: &[],
    },
    Mutation {
        defect: "a walk that was supplied no property is Clean (P6)",
        file: "src/explore.rs",
        from: "if self.properties.is_empty() {",
        to: "if false {",
        expect_red: "tests::a_run_that_checked_nothing_is_not_a_clean_bill",
        cargo_args: &[],
    },
    Mutation {
        defect: "a SUPPLIED property that was never once in its domain no \
                 longer refuses the walk — the second blocking finding: \
                 `properties_checked` meant supplied, and nonzero was treated \
                 as evidence",
        file: "src/explore.rs",
        from: "if self.properties.iter().any(|p| p.applicable == 0) {",
        to: "if false {",
        expect_red: "tests::a_property_the_alphabet_never_reaches_is_not_a_clean_bill",
        cargo_args: &[],
    },
    Mutation {
        defect: "NotApplicable is counted as applicable, which collapses the \
                 three answers back into the two a `Result` could carry and \
                 makes every supplied property look like it judged something",
        file: "src/explore.rs",
        from: "PropertyOutcome::NotApplicable => {}",
        to: "PropertyOutcome::NotApplicable => coverage.applicable += 1,",
        expect_red: "tests::a_property_the_alphabet_never_reaches_is_not_a_clean_bill",
        cargo_args: &[],
    },
    Mutation {
        defect: "a walk that applied no key is Clean (P6, the empty alphabet)",
        file: "src/explore.rs",
        from: "if self.transitions == 0 {",
        to: "if false {",
        expect_red: "tests::a_walk_over_an_empty_alphabet_is_not_a_clean_bill",
        cargo_args: &[],
    },
    Mutation {
        defect: "the README's worked example stops holding — the shape it \
                 shipped in: a row with dial chrome and no cursor on it",
        file: "README.md",
        from: ".adjustable().selected())",
        to: ".adjustable())",
        expect_red: "TheReadmeIsCompiledAndRun",
        cargo_args: &["--doc"],
    },
    Mutation {
        defect: "a shipped property whose body cannot fail — P7's \
                 `Named::new(name, |_| Ok(()))`, back in the set in the shape \
                 the new outcome type gives it",
        file: "src/property.rs",
        from: "    /// **At most one row is selected, and a non-empty component selects one.**",
        to: "    #[must_use]\n    pub fn a_no_op() -> Named<impl Fn(&Observation<'_>) -> PropertyOutcome> {\n        Named::new(\"a no-op\", |_| PropertyOutcome::Held)\n    }\n\n    /// **At most one row is selected, and a non-empty component selects one.**",
        expect_red: "property::tests::every_shipped_property_rejects_something",
        cargo_args: &[],
    },
    Mutation {
        defect: "a named lookup returns only the FIRST match, hiding the \
                 second behind the very name collision R5 stopped silencing",
        file: "src/explore.rs",
        from: "            .filter(|v| v.property == property)
            .collect()",
        to: "            .find(|v| v.property == property)
            .into_iter()
            .collect()",
        expect_red: "tests::a_named_lookup_returns_every_violation_with_that_name",
        cargo_args: &[],
    },
    Mutation {
        defect: "property retirement keyed on the NAME STRING, so two claims \
                 sharing a name silence each other (R5)",
        file: "src/explore.rs",
        from: "if reported.contains(&index) {",
        to: "if reported.iter().any(|i| properties[*i].name() == property.name()) {",
        expect_red: "tests::two_properties_with_one_name_are_both_checked",
        cargo_args: &[],
    },
    Mutation {
        defect: "a DEFAULT feature drags a dependency into every consumer — \
                 one string moved, and `newtui = \"0.1\"` resolves 50-odd \
                 crates including a terminal backend",
        file: "Cargo.toml",
        from: "default = []",
        to: "default = [\"ratatui\"]",
        expect_red: "the_shipped_closure_is_empty",
        cargo_args: &[],
    },
    Mutation {
        defect: "a BUILD dependency, which runs with the building user's full \
                 authority before a line of this crate compiles",
        file: "Cargo.toml",
        from: "[dev-dependencies]",
        to: "[build-dependencies]\nserde_json = \"1\"\n\n[dev-dependencies]",
        expect_red: "the_shipped_closure_is_empty",
        cargo_args: &[],
    },
    Mutation {
        defect: "the replay comparison is GONE — a reconstruction that lands \
                 in a different state than discovery recorded is accepted, \
                 and the walk judges a machine it never explored",
        file: "src/explore.rs",
        from: "        if departed.is_none() && arrived != departure.expected {",
        to: "        if departed.is_none() && false {",
        expect_red: "tests::a_replay_that_lands_elsewhere_is_not_a_clean_bill",
        cargo_args: &[],
    },
    Mutation {
        defect: "the replay's FLOWS are discarded again — a component that \
                 closes partway through a replay is not noticed, and every \
                 remaining key is delivered to a closed component",
        file: "src/explore.rs",
        from: "            if matches!(component.handle(*replayed), Flow::Close(_)) {
                departed = Some(DivergenceReason::ClosedDuringReplay { at });
                break;
            }",
        to: "            let _ = at;
            component.handle(*replayed);",
        expect_red: "tests::a_replay_that_closes_early_is_caught_as_such",
        cargo_args: &[],
    },
    Mutation {
        defect: "a diverged departure is RECORDED and then judged anyway — \
                 the outgoing keys are applied to the wrong machine, and any \
                 violation is attributed to a path that does not reach it",
        file: "src/explore.rs",
        from: "                        break;
                    }
                };",
        to: "                        let mut carried = factory();
                        for replayed in &departure.path {
                            carried.handle(*replayed);
                        }
                        carried
                    }
                };",
        expect_red: "tests::a_diverged_departure_judges_nothing",
        cargo_args: &[],
    },
    Mutation {
        defect: "the settings panel turns Escape into an accepted close",
        file: "src/components/settings_panel.rs",
        from: "Key::Esc => return self.finish(false, false),",
        to: "Key::Esc => return self.finish(true, false),",
        expect_red: "bounded_settings_panel_is_exhaustively_clean",
        cargo_args: &[],
    },
    Mutation {
        defect: "an accepted intent is installed while the settings panel \
                 remains open, so the default view fingerprint hides state \
                 that can receive another key",
        file: "src/components/settings_panel.rs",
        from: "        Flow::Close(true)\n    }\n}\n\nimpl Component for SettingsPanel",
        to: "        Flow::Stay\n    }\n}\n\nimpl Component for SettingsPanel",
        expect_red: "every_open_state_has_no_intent_hidden_from_view",
        cargo_args: &[],
    },
    Mutation {
        defect: "the catalogue omits the settings panel while the public \
                 export remains",
        file: "docs/CATALOG.md",
        from: "<!-- component: settings_panel -->",
        to: "<!-- omitted component: settings_panel -->",
        expect_red: "catalog_lists_every_component_export",
        cargo_args: &[],
    },
    Mutation {
        defect: "a catalogue snippet calls an API that does not exist, while \
                 the prose inventory remains otherwise intact",
        file: "docs/CATALOG.md",
        from: ".explore(|| SettingsPanel::new(seed.clone()), &refs);",
        to: ".explore_missing(|| SettingsPanel::new(seed.clone()), &refs);",
        expect_red: "TheCatalogIsCompiledAndRun",
        cargo_args: &["--doc"],
    },
];

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
            .args(["test", "--no-default-features"])
            .args(mutation.cargo_args)
            .arg(mutation.expect_red)
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
        // pass a red for a green in disguise. `expect_red` is matched inside
        // the harness's own result line rather than anywhere in the output, so
        // a mention in a panic message does not count — and a doctest, whose
        // printed name carries a line number, still matches.
        let by_the_named_guard = printed.lines().any(|line| {
            line.starts_with("test ")
                && line.contains(mutation.expect_red)
                && line.trim_end().ends_with("... FAILED")
        });
        assert!(
            by_the_named_guard,
            "the mutant for `{}` failed, but not by failing `{}` — so what went \
             red is not the guard:\n\n{printed}",
            mutation.defect, mutation.expect_red,
        );
        checked += 1;
        std::fs::remove_dir_all(&mutant).expect("the mutant is removable");
    }

    assert_eq!(checked, MUTATIONS.len(), "the loop did not reach every row");
}

/// **Every registered guard has a mutation, and every mutation names a
/// registered guard.** Both directions, both failing the build.
///
/// What this replaces was `assert!(checked >= 10)`, and a count is exactly the
/// shape this crate exists to refuse: a developer can add an eleventh guard
/// with no mutation behind it and stay green, which is the claim — *every
/// guard here has a defect it is known to catch* — passing unchanged with the
/// thing it claims absent. The identifiers have to MATCH, not be counted.
///
/// A guard declares itself with `// GUARD: <the filter cargo test takes>`
/// above its declaration, and this compares that set against the `expect_red`
/// of every row. A guard whose mutation was deleted fails here; a mutation
/// naming a guard that was renamed or removed fails here; and a marker whose
/// filter has drifted from the name below it fails here too, so a rename
/// cannot rot the registry quietly.
///
/// **What it does not close, plainly:** the marker is opt-in, so a new guard
/// whose author writes no marker is still invisible. That is the residue, and
/// it is smaller than the count's — a count closed neither direction. The
/// module comment states the discipline; this test enforces it for everything
/// that has ever been declared.
///
/// It cannot fail open. The scan feeds one side of an equality whose other
/// side is a non-empty table, so a scan that reads nothing fails with every
/// mutation unregistered rather than passing vacuously — and it asserts it read
/// a non-empty file anyway.
#[test]
fn every_registered_guard_is_pinned_by_a_mutation() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let registered = registered_guards(&root);
    let pinned: BTreeSet<&str> = MUTATIONS.iter().map(|m| m.expect_red).collect();

    let unpinned: Vec<&str> = registered
        .iter()
        .filter(|guard| !pinned.contains(guard.as_str()))
        .map(String::as_str)
        .collect();
    assert!(
        unpinned.is_empty(),
        "registered as guards, with no mutation that shows them failing: \
         {unpinned:?}. A guard nobody has seen fail is a claim about a defect \
         that is not present, which is the shape a broken guard satisfies for \
         free. Add the defect it catches to MUTATIONS."
    );

    let unregistered: Vec<&str> = pinned
        .iter()
        .filter(|guard| !registered.contains(**guard))
        .copied()
        .collect();
    assert!(
        unregistered.is_empty(),
        "named by a mutation and not registered in any source file: \
         {unregistered:?}. Either the guard was renamed or removed — in which \
         case the row is pinning nothing — or its `// GUARD:` marker is \
         missing."
    );
}

/// Every `.rs` file beneath `src/` and `tests/`, in a deterministic order.
///
/// **Recursive, and that is load-bearing.** This used to be one `read_dir` per
/// directory, which reads only the immediate children: a module in
/// `src/component/vi.rs` — the shape this crate is heading for as components
/// are extracted — was not scanned, and every guard in it was silently
/// unregistered. The scan would still pass, because an unregistered guard is
/// invisible in exactly the direction the registry cannot see. A discovery
/// walk that quietly covers less than it says is the class this file exists to
/// pin, in the file that pins it.
///
/// `target/` is excluded: a mutant copy of this crate lives there, and scanning
/// it would register every guard twice under a path that is not source.
///
/// Symlinks are not followed — a link pointing at an ancestor is the ordinary
/// way a recursive walk becomes an infinite one, and no source file in this
/// crate needs to be reached through one.
///
/// `tests/mutations.rs` is skipped: its `expect_red` strings are the other side
/// of the comparison, and a scan that read them would agree with itself.
fn rust_sources(root: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let mut pending: Vec<PathBuf> = ["src", "tests"].iter().map(|dir| root.join(dir)).collect();

    while let Some(dir) = pending.pop() {
        let entries = std::fs::read_dir(&dir)
            .unwrap_or_else(|err| panic!("{} is readable: {err}", dir.display()));
        for entry in entries {
            let path = entry.expect("a source entry").path();
            // `symlink_metadata` does NOT traverse the link, so a link to an
            // ancestor is seen as a link and skipped rather than descended.
            let kind = std::fs::symlink_metadata(&path)
                .unwrap_or_else(|err| panic!("{} is stat-able: {err}", path.display()))
                .file_type();
            if kind.is_symlink() {
                continue;
            }
            let name = path.file_name().and_then(std::ffi::OsStr::to_str);
            if kind.is_dir() {
                if name != Some("target") {
                    pending.push(path);
                }
            } else if path.extension().and_then(std::ffi::OsStr::to_str) == Some("rs")
                && name != Some("mutations.rs")
            {
                found.push(path);
            }
        }
    }

    // The walk order depends on the filesystem; the result must not.
    found.sort();
    found
}

/// Every `// GUARD:` marker in the crate's own sources.
///
/// The marker names the guard as `cargo test` takes it, and must sit directly
/// above the declaration it names — checked, so that renaming the test without
/// the marker fails here instead of silently deregistering the guard.
fn registered_guards(root: &Path) -> BTreeSet<String> {
    const MARKER: &str = "// GUARD: ";
    // Keyed by filter, valued by where it was registered, so that the same
    // guard claimed from two files is a named collision rather than a silent
    // set-insert that drops one of them.
    let mut registered: BTreeMap<String, PathBuf> = BTreeMap::new();
    let mut files = 0;

    for path in rust_sources(root) {
        let text = std::fs::read_to_string(&path).expect("a source file is readable");
        assert!(
            !text.trim().is_empty(),
            "{} is empty, so anything this scan concludes from it is vacuous",
            path.display()
        );
        files += 1;

        let lines: Vec<&str> = text.lines().collect();
        for (at, line) in lines.iter().enumerate() {
            let Some((_, rest)) = line.split_once(MARKER) else {
                continue;
            };
            let filter = rest
                .split_whitespace()
                .next()
                .expect("a guard marker names the test it registers");
            // The declaration it sits above, past any attributes.
            let declared = lines[at + 1..]
                .iter()
                .take(4)
                .find_map(|line| declared_item(line))
                .unwrap_or_else(|| {
                    panic!(
                        "the marker for `{filter}` in {} is not above a \
                         declaration",
                        path.display()
                    )
                });
            assert_eq!(
                filter.rsplit("::").next().expect("a non-empty filter"),
                declared,
                "the marker in {} registers `{filter}` but sits above \
                 `{declared}` — a rename must not deregister a guard quietly",
                path.display()
            );
            if let Some(first) = registered.insert(filter.to_string(), path.clone()) {
                panic!(
                    "`{filter}` is registered twice, in {} and in {} — one \
                     mutation cannot pin two guards, and a set-insert would \
                     have dropped one of them without a word",
                    first.display(),
                    path.display()
                );
            }
        }
    }

    assert!(files > 0, "no source file was scanned");
    assert!(
        !registered.is_empty(),
        "no guard is registered anywhere, so this scan means nothing"
    );
    registered.into_keys().collect()
}

/// A guard in a nested module is discovered.
///
/// The regression for a non-recursive `read_dir`. Under the old scan a
/// directory beneath `src/` failed the `.rs` extension test and was skipped
/// whole, so `src/component/vi.rs` — the shape this crate takes as soon as a
/// component is extracted into its own module — could carry any number of
/// `// GUARD:` markers and register none of them.
///
/// It is checked against a FIXTURE rather than this crate's own tree, because
/// a scan of a flat tree cannot tell the two implementations apart: with no
/// nested source file in the repository, the recursive and non-recursive walks
/// return the same set, and a test over the real tree would pass under the bug
/// it exists to catch.
#[test]
fn a_guard_in_a_nested_module_is_registered() {
    let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/guard-scan");
    let _ = std::fs::remove_dir_all(&fixture);
    std::fs::create_dir_all(fixture.join("src/component")).expect("the fixture is writable");
    std::fs::create_dir_all(fixture.join("tests")).expect("the fixture is writable");

    // A flat file with a marker, so the fixture is not vacuous in the other
    // direction: a walk that found NOTHING would trip the empty-scan assert
    // rather than the one this test is about.
    std::fs::write(
        fixture.join("src/lib.rs"),
        "// GUARD: tests::a_flat_guard\n#[test]\nfn a_flat_guard() {}\n",
    )
    .expect("the fixture is writable");
    std::fs::write(
        fixture.join("src/component/vi.rs"),
        "// GUARD: tests::a_nested_guard\n#[test]\nfn a_nested_guard() {}\n",
    )
    .expect("the fixture is writable");
    std::fs::write(fixture.join("tests/leaf.rs"), "// nothing to register\n")
        .expect("the fixture is writable");

    let registered = registered_guards(&fixture);

    assert!(
        registered.contains("tests::a_nested_guard"),
        "a `// GUARD:` marker in src/component/vi.rs was not registered — the \
         scan is not walking nested directories, so every guard in an \
         extracted component is invisible to the registry. Found: \
         {registered:?}"
    );
    assert!(
        registered.contains("tests::a_flat_guard"),
        "the flat marker was not registered either, so this fixture proves \
         nothing about nesting: {registered:?}"
    );

    std::fs::remove_dir_all(&fixture).expect("the fixture is removable");
}

/// The name declared by a `fn` or `struct` line, if it is one.
fn declared_item(line: &str) -> Option<&str> {
    let line = line.trim();
    let line = line
        .strip_prefix("pub(crate) ")
        .or_else(|| line.strip_prefix("pub "))
        .unwrap_or(line);
    let rest = line
        .strip_prefix("fn ")
        .or_else(|| line.strip_prefix("struct "))?;
    Some(
        rest.split(|c: char| !c.is_alphanumeric() && c != '_')
            .next()
            .unwrap_or(rest),
    )
}

/// Copy the crate into `dest` — sources, manifest and lockfile, and nothing
/// else. Explicitly not `target/` (recursive, enormous) and not this file (a
/// mutant that ran its own mutation runner would nest without end).
fn copy_crate(root: &Path, dest: &Path) {
    for file in ["Cargo.toml", "Cargo.lock", "README.md", "docs/CATALOG.md"] {
        let parent = dest
            .join(file)
            .parent()
            .expect("every copied file has a parent")
            .to_path_buf();
        std::fs::create_dir_all(parent).expect("the mutant arena is writable");
        std::fs::copy(root.join(file), dest.join(file)).expect("crate file is copyable");
    }
    for source in rust_sources(root) {
        let relative = source
            .strip_prefix(root)
            .expect("a discovered source belongs to this crate");
        let target = dest.join(relative);
        std::fs::create_dir_all(target.parent().expect("a source has a parent"))
            .expect("the mutant arena is writable");
        std::fs::copy(source, target).expect("a source file is copyable");
    }
}
