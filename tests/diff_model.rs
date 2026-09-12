use newtui::diff::{from_unified, ChangeSet, DiffLine, FileKind, ParseErrorKind};

fn round_trip(source: &str) -> ChangeSet {
    let changes = from_unified(source).expect("fixture parses");
    assert_eq!(changes.to_unified(), source);
    assert_eq!(from_unified(&changes.to_unified()).unwrap(), changes);
    changes
}

#[test]
fn files_hunks_and_line_addresses_are_data_without_a_terminal() {
    let changes = round_trip("--- a/one.rs\n+++ b/one.rs\n@@ -4,2 +4,3 @@ fn example\n context\n-old\n+new\n+extra\n--- /dev/null\n+++ b/new file.py\n@@ -0,0 +1 @@\n+print(1)\n");
    assert_eq!(changes.files().len(), 2);
    let first = &changes.files()[0];
    assert_eq!(first.path().old_path(), "a/one.rs");
    assert_eq!(first.path().new_path(), "b/one.rs");
    assert_eq!(first.kind(), FileKind::Modified);
    assert_eq!(first.hunks()[0].old_range(), 4..6);
    assert_eq!(first.hunks()[0].new_range(), 4..7);
    assert_eq!(first.hunks()[0].section(), " fn example");
    assert_eq!(first.hunks()[0].lines()[1], DiffLine::Remove("old".into()));
    assert_eq!(first.additions(), 2);
    assert_eq!(first.removals(), 1);
    assert_eq!(changes.files()[1].kind(), FileKind::Added);
}

#[test]
fn deleted_file_and_zero_hunks_round_trip() {
    let changes =
        round_trip("--- a/gone\n+++ /dev/null\n@@ -1 +0,0 @@\n-gone\n--- a/empty\n+++ b/empty\n");
    assert_eq!(changes.files()[0].kind(), FileKind::Deleted);
    assert!(changes.files()[1].hunks().is_empty());
    assert_eq!(round_trip("").files().len(), 0);
}

#[test]
fn no_newline_markers_belong_to_the_previous_side_and_survive() {
    let changes = round_trip("--- a/file\n+++ b/file\n@@ -1 +1 @@\n-old\n\\ No newline at end of file\n+new\n\\ No newline at end of file\n");
    assert_eq!(
        changes.files()[0].hunks()[0].lines()[1],
        DiffLine::NoNewline
    );
    assert_eq!(changes.files()[0].additions(), 1);
    assert_eq!(changes.files()[0].removals(), 1);
    round_trip("--- a/file\n+++ b/file\n@@ -1 +1 @@\n same\n\\ No newline at end of file\n");
}

#[test]
fn crlf_source_bytes_are_not_transport_normalization() {
    round_trip("--- a/file\n+++ b/file\n@@ -1 +1 @@\n-old\r\n+new\r\n");
    round_trip("--- a/file\n+++ b/file\n@@ -1 +1 @@\n-old\r\n\\ No newline at end of file\n+new\r\n\\ No newline at end of file\n");
    let wire = "--- a/file\r\n+++ b/file\r\n@@ -1 +1 @@\r\n-old\r\n+new\r\n";
    assert_eq!(
        from_unified(wire).unwrap().to_unified(),
        "--- a/file\n+++ b/file\n@@ -1 +1 @@\n-old\n+new\n"
    );
}

#[test]
fn context_prefixes_and_arbitrary_utf8_are_never_classified_as_controls() {
    let changes = round_trip("--- a/f\n+++ b/f\n@@ -1,5 +1,5 @@\n +addition-looking\n -removal-looking\n \\ marker-looking\n @@ header-looking\n \t你好 e\u{301} 🦎\u{7f}\u{9b}\n");
    assert!(changes.files()[0].hunks()[0]
        .lines()
        .iter()
        .all(|line| matches!(line, DiffLine::Context(_))));
}

#[test]
fn git_metadata_binary_and_rename_only_are_retained() {
    let changes = round_trip("diff --git a/old -> name b/new -> name\nsimilarity index 100%\nrename from old -> name\nrename to new -> name\ndiff --git a/mode b/mode\nold mode 100644\nnew mode 100755\ndiff --git a/image b/image\nindex 1234567..abcdef0 100644\nBinary files a/image and b/image differ\n");
    assert_eq!(changes.files()[0].kind(), FileKind::Renamed);
    assert_eq!(changes.files()[0].path().old_path(), "old -> name");
    assert_eq!(changes.files()[0].path().new_path(), "new -> name");
    assert_eq!(
        changes.files()[1].metadata(),
        &["old mode 100644", "new mode 100755"]
    );
    assert!(changes.files()[2].is_binary());
    assert!(changes.files().iter().all(|file| file.hunks().is_empty()));
}

#[test]
fn quoted_paths_and_spaces_preserve_their_spelling() {
    round_trip("diff --git \"a/weird \\303\\251\" \"b/weird \\303\\251\"\n--- \"a/weird \\303\\251\"\n+++ \"b/weird \\303\\251\"\n@@ -1 +1 @@\n-a\n+b\n");
    let changes = round_trip(
        "diff --git a/path with spaces b/path with spaces\nold mode 100644\nnew mode 100755\n",
    );
    assert_eq!(changes.files()[0].path().old_path(), "a/path with spaces");
    round_trip("--- before name\t2026-01-01\n+++ after name\t2026-01-02\n@@ -1 +1 @@\n-a\n+b\n");
}

#[test]
fn additions_deletions_copies_and_binary_creations_have_honest_kinds() {
    let changes = round_trip("diff --git a/new b/new\nnew file mode 100644\nindex 0000000..1111111\nBinary files /dev/null and b/new differ\ndiff --git a/old b/old\ndeleted file mode 100644\ndiff --git a/old b/copy\nsimilarity index 100%\ncopy from old\ncopy to copy\n");
    assert_eq!(changes.files()[0].kind(), FileKind::Added);
    assert_eq!(changes.files()[1].kind(), FileKind::Deleted);
    assert_eq!(changes.files()[2].kind(), FileKind::Copied);
}

#[test]
fn canonical_format_normalizes_single_counts_and_missing_transport_newline() {
    let canonical = "--- a/f\n+++ b/f\n@@ -1 +1 @@\n-a\n+b\n";
    assert_eq!(
        from_unified("--- a/f\n+++ b/f\n@@ -1,1 +1,1 @@\n-a\n+b")
            .unwrap()
            .to_unified(),
        canonical
    );
    assert_eq!(from_unified(canonical).unwrap().to_unified(), canonical);
}

#[test]
fn markdown_fence_contains_the_exact_canonical_patch() {
    let source = "--- a/f\n+++ b/f\n@@ -1 +1 @@\n-```\n+````diff\n";
    let changes = round_trip(source);
    assert_eq!(changes.to_markdown(), format!("`````diff\n{source}`````\n"));
    assert_eq!(ChangeSet::default().to_markdown(), "```diff\n```\n");
}

#[test]
fn malformed_inputs_fail_with_typed_line_numbers() {
    for (source, kind) in [
        ("not a patch\n", ParseErrorKind::ExpectedFileHeader),
        ("--- a/f\n", ParseErrorKind::MissingNewFileHeader),
        (
            "--- a/f\n+++ b/f\n@@ nonsense\n",
            ParseErrorKind::InvalidHunkHeader,
        ),
        (
            "--- a/f\n+++ b/f\n@@ -4294967295,1 +1 @@\n-a\n+b\n",
            ParseErrorKind::InvalidRange,
        ),
        (
            "--- a/f\n+++ b/f\n@@ -0 +1 @@\n-a\n+b\n",
            ParseErrorKind::InvalidRange,
        ),
        (
            "--- a/f\n+++ b/f\n@@ -1,2 +1 @@\n-a\n+b\n",
            ParseErrorKind::HunkLengthMismatch,
        ),
        (
            "--- a/f\n+++ b/f\n@@ -1 +1 @@\nxinvalid\n",
            ParseErrorKind::InvalidDiffLine,
        ),
        (
            "--- a/f\n+++ b/f\n@@ -1 +1 @@\n\\ No newline at end of file\n-a\n+b\n",
            ParseErrorKind::InvalidNoNewline,
        ),
        (
            "--- a/f\n+++ b/f\n@@ -1,2 +1 @@\n-a\n\\ No newline at end of file\n-b\n+c\n",
            ParseErrorKind::InvalidNoNewline,
        ),
        (
            "diff --git a/f b/f\nunknown future metadata\n",
            ParseErrorKind::UnsupportedMetadata,
        ),
        (
            "diff --git a/f b/f\nGIT binary patch\nliteral 1\n",
            ParseErrorKind::UnsupportedMetadata,
        ),
        ("diff --cc file\n", ParseErrorKind::UnsupportedMetadata),
        (
            "diff --git a/f b/f\nold mode invalid\n",
            ParseErrorKind::InvalidMetadata,
        ),
        (
            "diff --git a/old b/new\nrename from old\n",
            ParseErrorKind::InvalidMetadata,
        ),
        ("--- \"bad\\q\"\n+++ b/f\n", ParseErrorKind::InvalidPath),
        (
            "--- /dev/null\n+++ /dev/null\n",
            ParseErrorKind::InvalidPath,
        ),
    ] {
        let error = from_unified(source).expect_err(source);
        assert_eq!(error.kind(), kind, "{source:?}: {error}");
        assert!(error.line() > 0);
        assert!(error.to_string().contains(&error.line().to_string()));
    }
}

#[test]
fn a_no_newline_side_cannot_reappear_in_a_later_hunk() {
    let source = "--- a/f\n+++ b/f\n@@ -1 +1 @@\n-a\n\\ No newline at end of file\n+b\n@@ -3 +3 @@\n-c\n+d\n";
    assert_eq!(
        from_unified(source).unwrap_err().kind(),
        ParseErrorKind::InvalidNoNewline
    );
}

#[test]
fn contradictory_metadata_and_headers_are_errors_not_lossy_models() {
    for source in [
        "diff --git a/f b/f\nold mode 100644\nold mode 100755\nnew mode 100755\n",
        "diff --git a/f b/f\nnew file mode 100644\ndeleted file mode 100644\n",
        "diff --git a/f b/f\nnew file mode 100644\n--- a/f\n+++ b/f\n",
        "diff --git a/f b/f\nold mode 100644\n",
        "--- /dev/null\n+++ b/f\n@@ -1 +1 @@\n-old\n+new\n",
        "--- a/f\n+++ /dev/null\n@@ -1 +1 @@\n-old\n+new\n",
    ] {
        assert!(from_unified(source).is_err(), "accepted {source:?}");
    }
    assert_eq!(
        from_unified("diff --git a/f b/f\n--- a/other\n+++ b/other\n")
            .unwrap_err()
            .kind(),
        ParseErrorKind::InvalidPath
    );
}

#[test]
fn binary_markers_cannot_name_a_different_file_than_the_model() {
    assert_eq!(
        from_unified("diff --git a/f b/f\nBinary files a/other and b/other differ\n")
            .unwrap_err()
            .kind(),
        ParseErrorKind::InvalidMetadata
    );
    round_trip("diff --git a/gone b/gone\ndeleted file mode 100644\nBinary files a/gone and /dev/null differ\n");
    round_trip("diff --git a/old b/new\nsimilarity index 80%\nrename from old\nrename to new\nBinary files a/old and b/new differ\n");
}

#[test]
fn empty_ranges_boundary_numbers_and_quoted_byte_paths_are_explicit() {
    round_trip("--- a/f\n+++ b/f\n@@ -0,0 +0,0 @@\n@@ -4294967294 +4294967294 @@\n same\n");
    round_trip("diff --git \"a/\\377\\t\\\\\\\"\" \"b/\\377\\t\\\\\\\"\"\nold mode 100644\nnew mode 100755\n");
    round_trip("diff --git a/contains b/separator b/contains b/separator\nold mode 100644\nnew mode 100755\n");
    let wire = "--- a/f\r\n+++ b/f\r\n@@ -1 +1 @@\r\n-a\r\r\n+b\r\r\n";
    assert_eq!(
        from_unified(wire).unwrap().to_unified(),
        "--- a/f\n+++ b/f\n@@ -1 +1 @@\n-a\r\n+b\r\n"
    );
    for source in [
        "diff --git a/f b/other\nold mode 100644\nnew mode 100755\n",
        "diff --git b/f a/f\n--- b/f\n+++ a/f\n",
        "--- \"a/\\777\"\n+++ b/f\n",
        "--- \"a/unclosed\n+++ b/f\n",
        "--- a/f\n+++ b/f\n@@ -1 +1 @@\n-a\n+b\n@@ -1 +1 @@\n-c\n+d\n",
    ] {
        assert!(from_unified(source).is_err(), "accepted {source:?}");
    }
}

#[test]
fn explicit_operations_cannot_be_reclassified_by_absent_side_headers() {
    for operation in ["rename", "copy"] {
        for (old, new, hunk) in [
            ("a/old", "/dev/null", "@@ -1 +0,0 @@\n-old\n"),
            ("/dev/null", "b/new", "@@ -0,0 +1 @@\n+new\n"),
        ] {
            let patch = format!("diff --git a/old b/new\n{operation} from old\n{operation} to new\n--- {old}\n+++ {new}\n{hunk}");
            assert_eq!(
                from_unified(&patch).unwrap_err().kind(),
                ParseErrorKind::InvalidMetadata,
                "{patch}"
            );
        }
    }
}

#[test]
fn operation_modes_scores_and_absent_index_sides_must_agree() {
    for patch in [
        "diff --git a/f b/f\nnew file mode 100644\nold mode 100644\nnew mode 100755\n",
        "diff --git a/f b/f\ndeleted file mode 100644\nold mode 100644\nnew mode 100755\n",
        "diff --git a/f b/f\nold mode 100644\nnew mode 100755\n--- /dev/null\n+++ b/f\n",
        "diff --git a/f b/f\nsimilarity index 100%\ndissimilarity index 100%\n",
        "diff --git a/f b/f\nnew file mode 100644\nindex 1234567..abcdef0\n--- /dev/null\n+++ b/f\n@@ -0,0 +1 @@\n+new\n",
        "diff --git a/f b/f\ndeleted file mode 100644\nindex abcdef0..1234567\n--- a/f\n+++ /dev/null\n@@ -1 +0,0 @@\n-old\n",
        "diff --git a/f b/f\nindex 1234567..abcdef0\n--- /dev/null\n+++ b/f\n",
        "diff --git a/f b/f\nold mode 100644\nnew mode 100755\nindex 1234567..abcdef0 100644\n",
        "diff --git a/f b/f\nnew file mode 100644\nindex 0000000..abcdef0 100644\n",
        "diff --git a/f b/f\nnew file mode 100644\nsimilarity index 50%\n",
        "diff --git a/f b/f\ndeleted file mode 100644\ndissimilarity index 50%\n",
    ] {
        assert_eq!(from_unified(patch).unwrap_err().kind(), ParseErrorKind::InvalidMetadata, "{patch}");
    }
}

#[test]
fn rename_copy_and_rewrite_can_change_mode_and_content_together() {
    for operation in ["rename", "copy"] {
        let patch = format!("diff --git a/old b/new\nold mode 100644\nnew mode 100755\nsimilarity index 66%\n{operation} from old\n{operation} to new\nindex 1234567..abcdef0\n--- a/old\n+++ b/new\n@@ -1,3 +1,3 @@\n same\n-old\n+new\n same\n");
        let changes = round_trip(&patch);
        let expected = if operation == "rename" {
            FileKind::Renamed
        } else {
            FileKind::Copied
        };
        assert_eq!(changes.files()[0].kind(), expected);
    }
    round_trip("diff --git a/f b/f\nold mode 100644\nnew mode 100755\ndissimilarity index 100%\nindex 1234567..abcdef0\n--- a/f\n+++ b/f\n@@ -1 +1 @@\n-old\n+new\n");
    // Prefixes are not full object identities: a present blob's abbreviated
    // spelling can consist of zeroes. Only a definitely absent side is checked.
    round_trip("diff --git a/f b/f\nindex 0000000..abcdef0 100644\n--- a/f\n+++ b/f\n@@ -1 +1 @@\n-old\n+new\n");
    round_trip("diff --git a/f b/f\nnew file mode 100644\nindex 0000000..abcdef0\n");
    round_trip("diff --git a/f b/f\ndeleted file mode 100644\nindex abcdef0..0000000\n");
}
