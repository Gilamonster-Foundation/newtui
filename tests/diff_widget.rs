use newtui::diff::from_unified;
use newtui::{diff, ContextRun, DiffData, DiffGeometry, NoticeVisibility, Tone, WidgetNoticeKind};

const PATCH: &str =
    "--- a/code\n+++ b/code\n@@ -10,4 +20,4 @@\n same\n-old1\n-old2\n+new1\n+new2\n tail\n";

fn text(output: &newtui::WidgetOutput) -> String {
    output
        .lines
        .iter()
        .map(|line| line.text())
        .collect::<Vec<_>>()
        .join("\n")
}

// GUARD: diff_geometry_preserves_source_addresses
#[test]
fn diff_geometry_preserves_source_addresses() {
    let model = from_unified(PATCH).unwrap();
    let output = diff(DiffData::new(&model), 80, 12);
    output.validate(80, 12).unwrap();
    let rendered = text(&output);
    assert!(rendered.contains("10 20  same"), "{rendered}");
    assert!(rendered.contains("11    -old1"), "{rendered}");
    assert!(rendered.contains("   21 +new1"), "{rendered}");
    let split = diff(DiffData::new(&model).geometry(DiffGeometry::Split), 80, 12);
    let row = split
        .lines
        .iter()
        .find(|row| row.text().contains("old1"))
        .unwrap()
        .text();
    assert!(
        row.contains("11 -old1") && row.contains("21 +new1"),
        "{row}"
    );
    assert!(split
        .lines
        .iter()
        .any(|row| row.runs.iter().any(|run| run.tone == Tone::Added)));
}

// GUARD: diff_escapes_survive_tiny_rectangles
#[test]
fn diff_escapes_survive_tiny_rectangles() {
    let model =
        from_unified("--- a/code\n+++ b/code\n@@ -1 +1 @@\n-plain\n+\t中🚀e\u{301}│\r\n").unwrap();
    let original = model.to_unified();
    for geometry in [DiffGeometry::Unified, DiffGeometry::Split] {
        for width in [0, 1, 8, 20, 200] {
            for height in [0, 1, 2, 12] {
                let output = diff(DiffData::new(&model).geometry(geometry), width, height);
                output.validate(width, height).unwrap();
                let notice = output
                    .notices
                    .iter()
                    .find(|notice| {
                        matches!(notice.kind, WidgetNoticeKind::GlyphReplacements { .. })
                    })
                    .unwrap();
                assert_eq!(
                    notice.kind,
                    WidgetNoticeKind::GlyphReplacements { count: 6 }
                );
                if width == 0 || height == 0 {
                    assert_eq!(notice.visibility, NoticeVisibility::Hidden);
                } else if width == 1 {
                    assert!(text(&output).contains('!'));
                }
            }
        }
    }
    assert_eq!(model.to_unified(), original);
}

#[test]
fn split_falls_back_and_offsets_and_stat_remain_honest() {
    let model = from_unified(PATCH).unwrap();
    let output = diff(DiffData::new(&model).geometry(DiffGeometry::Split), 8, 4);
    assert!(output.notices.iter().any(|notice| matches!(
        notice.kind,
        WidgetNoticeKind::LayoutFallback {
            requested: "split",
            rendered: "unified"
        }
    )));
    let output = diff(
        DiffData::new(&model)
            .row_offset(usize::MAX)
            .column_offset(usize::MAX),
        20,
        4,
    );
    output.validate(20, 4).unwrap();
    assert!(output.notices.iter().any(
        |notice| matches!(notice.kind, WidgetNoticeKind::OmittedRows { before, .. } if before > 0)
    ));
    let output = diff(DiffData::new(&model).geometry(DiffGeometry::Stat), 80, 12);
    assert!(text(&output).contains("(+2 -2)"));
}

#[test]
fn full_domain_obeys_rectangles_and_preserves_the_model() {
    let cases = [
        "",
        PATCH,
        "--- /dev/null\n+++ b/new\n@@ -0,0 +1,2 @@\n+first\n+last\n\\ No newline at end of file\n",
        "--- a/old\n+++ /dev/null\n@@ -1,2 +0,0 @@\n-first\n-last\n\\ No newline at end of file\n",
        "--- a/high\n+++ b/high\n@@ -4294967294 +4294967294 @@\n-old\n+new\n",
        "--- a/c\n+++ b/c\n@@ -1,4 +1,4 @@\n +context\n -context\n \\context\n @context\n",
        "--- a/c\n+++ b/c\n@@ -1 +1 @@\n \t中🚀e\u{301}│\r\n\\ No newline at end of file\n",
        "diff --git a/image b/image\nindex abc123..fed321 100644\nBinary files a/image and b/image differ\n",
        "diff --git a/old b/new\nsimilarity index 100%\nrename from old\nrename to new\n",
        "diff --git a/script b/script\nold mode 100644\nnew mode 100755\n",
        "--- \"a/weird \\303\\251\"\n+++ \"b/weird \\303\\251\"\n",
    ];
    for patch in cases {
        let model = from_unified(patch).unwrap();
        let original = model.to_unified();
        for geometry in [
            DiffGeometry::Unified,
            DiffGeometry::Split,
            DiffGeometry::Stat,
        ] {
            for width in [0, 1, 8, 20, 200] {
                for height in [0, 1, 2, 12] {
                    for offset in [0, 1, usize::MAX] {
                        let output = diff(
                            DiffData::new(&model)
                                .geometry(geometry)
                                .row_offset(offset)
                                .column_offset(offset),
                            width,
                            height,
                        );
                        output.validate(width, height).unwrap_or_else(|error| {
                            panic!("{geometry:?} {width}x{height} {offset}: {error} {output:?}")
                        });
                    }
                }
            }
        }
        assert_eq!(model.to_unified(), original);
    }
}

#[test]
fn folds_are_addressed_in_model_space_and_annotations_keep_their_side() {
    let model = from_unified("--- a/code\n+++ b/code\n@@ -1,9 +1,9 @@\n a\n b\n c\n d\n e\n f\n g\n h\n-old\n\\ No newline at end of file\n+new\n\\ No newline at end of file\n").unwrap();
    let folded = diff(DiffData::new(&model).context(1), 80, 20);
    assert!(text(&folded).contains("... 6 unchanged"));
    assert!(text(&folded).contains("8 8  h"));
    assert!(text(&folded).contains("9   -old"));
    assert_eq!(
        text(&folded)
            .matches("\\ No newline at end of file")
            .count(),
        2
    );
    let expanded = [ContextRun {
        file: 0,
        hunk: 0,
        line: 0,
    }];
    let full = diff(DiffData::new(&model).context(1).expanded(&expanded), 80, 20);
    assert!(!text(&full).contains("unchanged"));
    assert!(text(&full).contains("4 4  d"));
    assert!(!full
        .notices
        .iter()
        .any(|notice| matches!(notice.kind, WidgetNoticeKind::FoldedRows { .. })));
    let split = diff(
        DiffData::new(&model)
            .geometry(DiffGeometry::Split)
            .context(0),
        80,
        20,
    );
    let marker = split
        .lines
        .iter()
        .find(|line| line.text().contains("No newline"))
        .unwrap()
        .text();
    assert_eq!(marker.matches("No newline").count(), 2);
}

#[test]
fn split_has_explicit_gaps_and_does_not_pair_across_hunks() {
    let model = from_unified(
        "--- a/code\n+++ b/code\n@@ -1,2 +1 @@\n-one\n-two\n+first\n@@ -5 +4 @@\n-last\n+final\n",
    )
    .unwrap();
    let output = diff(DiffData::new(&model).geometry(DiffGeometry::Split), 80, 20);
    let row = output
        .lines
        .iter()
        .find(|line| line.text().contains("two"))
        .unwrap()
        .text();
    assert!(row.split_once(" | ").unwrap().1.trim().is_empty(), "{row}");
    let row = output
        .lines
        .iter()
        .find(|line| line.text().contains("last"))
        .unwrap()
        .text();
    assert!(row.contains("5 -last") && row.contains("4 +final"), "{row}");
}

#[test]
fn counters_are_before_split_duplication_and_clipping_is_exact() {
    let model = from_unified("--- a/code\n+++ b/code\n@@ -1 +1 @@\n \t中\n").unwrap();
    let output = diff(DiffData::new(&model).geometry(DiffGeometry::Split), 200, 8);
    assert_eq!(output.notices.len(), 1);
    assert_eq!(
        output.notices[0].kind,
        WidgetNoticeKind::GlyphReplacements { count: 2 }
    );
    assert_eq!(output.notices[0].visibility, NoticeVisibility::Full);
    let model = from_unified("--- a/code\n+++ b/code\n@@ -1 +1 @@\n 0123456789\n").unwrap();
    let output = diff(DiffData::new(&model).column_offset(3), 20, 8);
    assert!(text(&output).contains("<3456789"));
    assert!(output.notices.iter().any(|notice| notice.kind
        == WidgetNoticeKind::ClippedColumns {
            row: 2,
            before: 3,
            after: 0
        }));
    let ordinary = newtui::bar("x", 1.0, 2.0, "1", 20, 1);
    assert!(ordinary.notices.is_empty());
}

#[test]
fn metadata_only_and_binary_stats_never_imply_no_change() {
    for (patch, expected) in [
        (
            "diff --git a/image b/image\nBinary files a/image and b/image differ\n",
            "Binary files",
        ),
        (
            "diff --git a/script b/script\nold mode 100644\nnew mode 100755\n",
            "old mode 100644",
        ),
        (
            "diff --git a/old b/new\nsimilarity index 100%\nrename from old\nrename to new\n",
            "Renamed old -> new",
        ),
        (
            "diff --git a/old b/new\nsimilarity index 100%\ncopy from old\ncopy to new\n",
            "Copied old -> new",
        ),
    ] {
        let model = from_unified(patch).unwrap();
        let output = diff(DiffData::new(&model).geometry(DiffGeometry::Stat), 80, 12);
        assert!(text(&output).contains(expected), "{}", text(&output));
        assert!(text(&output).contains("(+0 -0)"));
    }
}

#[test]
fn fallback_boundary_keeps_whole_gutters_and_header_escapes_have_notices() {
    let model = from_unified(PATCH).unwrap();
    for (width, fallback) in [(14, true), (15, false)] {
        let output = diff(
            DiffData::new(&model).geometry(DiffGeometry::Split),
            width,
            12,
        );
        output.validate(width, 12).unwrap();
        assert_eq!(
            output.notices.iter().any(|notice| matches!(
                notice.kind,
                WidgetNoticeKind::LayoutFallback {
                    requested: "split",
                    ..
                }
            )),
            fallback
        );
    }
    let model = from_unified("--- a/中\n+++ b/中\n").unwrap();
    let output = diff(DiffData::new(&model), 80, 8);
    assert!(output.lines[0].text().contains("b/? (+0 -0)"));
    assert!(output.lines[0].text().contains("1 escaped"));
    assert_eq!(
        output.notices[0].kind,
        WidgetNoticeKind::GlyphReplacements { count: 1 }
    );
    let model = from_unified(
        "--- a/old\n+++ /dev/null\n@@ -1 +0,0 @@\n-old\n\\ No newline at end of file\n",
    )
    .unwrap();
    let output = diff(DiffData::new(&model).geometry(DiffGeometry::Split), 80, 8);
    let row = output
        .lines
        .iter()
        .find(|line| line.text().contains("No newline"))
        .unwrap()
        .text();
    assert!(row.split_once(" | ").unwrap().1.trim().is_empty());
}

#[test]
fn zero_hunk_files_do_not_invent_metadata() {
    for patch in [
        "--- a/empty\n+++ b/empty\n",
        "--- /dev/null\n+++ b/empty\n",
        "--- a/empty\n+++ /dev/null\n",
    ] {
        let model = from_unified(patch).unwrap();
        for width in [1, 8, 20, 80] {
            let output = diff(DiffData::new(&model), width, 6);
            output.validate(width, 6).unwrap();
            assert!(!text(&output).contains("metadata-only"));
            if width == 80 {
                assert_eq!(output.lines[1].text().trim(), "No text hunks");
            }
        }
    }
}
