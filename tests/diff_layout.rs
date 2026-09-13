use newtui::diff::from_unified;
use newtui::{
    diff_layout, diff_with_sources, ContextRun, DiffData, DiffGeometry, DiffSide, DiffTarget,
};

const PATCH: &str = "--- a/a\n+++ b/a\n@@ -1,3 +1,2 @@\n \n-old\n-gone\n+new\n\\ No newline at end of file\ndiff --git a/script b/script\nold mode 100644\nnew mode 100755\n";

// GUARD: diff_row_targets_survive_blank_clipped_and_metadata_rows
#[test]
fn diff_row_targets_survive_blank_clipped_and_metadata_rows() {
    let model = from_unified(PATCH).unwrap();
    for width in [0, 1, 8, 20, 200] {
        let data = DiffData::new(&model).column_offset(usize::MAX);
        let layout = diff_layout(data, width);
        let projection = diff_with_sources(data, width, 20);
        assert!(projection.sources.is_empty());
        assert!(layout.rows.iter().any(|row| matches!(
            row.old,
            Some(DiffTarget::Line {
                file: 0,
                hunk: 0,
                line: 0,
                old: Some(1),
                new: Some(1)
            })
        )));
        assert!(layout.rows.iter().any(|row| matches!(
            row.new,
            Some(DiffTarget::Annotation {
                file: 0,
                hunk: 0,
                line: 3
            })
        )));
        assert!(layout
            .rows
            .iter()
            .any(|row| matches!(row.old, Some(DiffTarget::File { file: 1 }))));
        // One row per rendered line: seven for `a`, four for `script` (label,
        // two mode lines, "No text hunks"). The footer is not a layout row.
        assert_eq!(layout.rows.len(), 11);
        assert_eq!(
            layout.window(usize::MAX, width, 3),
            if width == 0 { 10..10 } else { 9..11 }
        );
    }
    let selected = DiffData::new(&model).file(1).unwrap();
    let layout = diff_layout(selected, 80);
    assert!(layout
        .rows
        .iter()
        .all(|row| row.old == Some(DiffTarget::File { file: 1 })));
    assert_eq!(DiffData::new(&model).file(2).unwrap_err().file, 2);
}

// GUARD: independent_diff_panes_keep_side_addresses_and_windows
#[test]
fn independent_diff_panes_keep_side_addresses_and_windows() {
    let model = from_unified(PATCH).unwrap();
    let old = DiffData::new(&model).file(0).unwrap().pane(DiffSide::Old);
    let new = DiffData::new(&model).file(0).unwrap().pane(DiffSide::New);
    let old_layout = diff_layout(old, 40);
    let new_layout = diff_layout(new, 40);
    assert_eq!(old_layout.rows.len(), 5);
    assert_eq!(new_layout.rows.len(), 5);
    assert!(old_layout.rows.iter().all(|row| row.new.is_none()));
    assert!(new_layout.rows.iter().all(|row| row.old.is_none()));
    let before = diff_with_sources(new, 40, 3);
    let scrolled = diff_with_sources(old.row_offset(3), 40, 3);
    assert_eq!(diff_with_sources(new, 40, 3), before);
    assert!(scrolled.output.lines[0].text().contains("2 -old"));
    assert!(scrolled
        .sources
        .iter()
        .all(|span| span.side == DiffSide::Old));
    let new_tail = diff_with_sources(new.row_offset(3), 40, 3);
    assert!(new_tail.output.lines[0].text().contains("2 +new"));
    assert!(new_tail
        .sources
        .iter()
        .all(|span| span.side == DiffSide::New));
    assert!(new_tail.output.lines[1].text().contains("No newline"));
    for side in [DiffSide::Old, DiffSide::New] {
        for width in [0, 1, 8, 20, 200] {
            for height in [0, 1, 2, 12] {
                diff_with_sources(DiffData::new(&model).pane(side), width, height)
                    .output
                    .validate(width, height)
                    .unwrap();
            }
        }
    }
}

// GUARD: diff_folds_keep_addresses_without_retaining_source
#[test]
fn diff_folds_keep_addresses_without_retaining_source() {
    let patch = "--- a/f\n+++ b/f\n@@ -1,6 +1,6 @@\n a\n b\n c\n d\n e\n f\n";
    let model = from_unified(patch).unwrap();
    let different = from_unified(&patch.replace(" a\n", " secret payload\n")).unwrap();
    let data = DiffData::new(&model).context(1);
    let run = ContextRun {
        file: 0,
        hunk: 0,
        line: 0,
    };
    let folded = diff_layout(data, 80);
    assert_eq!(folded.rows.len(), 5);
    assert_eq!(
        folded,
        diff_layout(DiffData::new(&different).context(1), 80)
    );
    assert_eq!(
        folded.rows[3].old,
        Some(DiffTarget::Fold { run, entries: 1..5 })
    );
    assert_eq!(diff_layout(data.expanded(&[run]), 80).rows.len(), 8);
    let split = diff_layout(data.geometry(DiffGeometry::Split), 80);
    assert_eq!(split.geometry, DiffGeometry::Split);
    assert_eq!(split.rows[2].old, split.rows[2].new);
    assert_eq!(
        diff_layout(data.geometry(DiffGeometry::Split), 1).geometry,
        DiffGeometry::Unified
    );
    let stat = diff_layout(data.geometry(DiffGeometry::Stat), 80);
    assert_eq!(stat.rows[1].old, Some(DiffTarget::Stat { file: 0 }));
    assert_eq!(
        diff_layout(data.pane(DiffSide::Old).geometry(DiffGeometry::Split), 80).pane,
        None
    );
}
