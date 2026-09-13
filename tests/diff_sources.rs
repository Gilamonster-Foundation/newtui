use newtui::diff::{from_unified, ChangeSet, DiffLine};
use newtui::{diff, diff_with_sources, ContextRun, DiffData, DiffGeometry, DiffSide};

const PATCH: &str = "--- a/one\n+++ b/one\n@@ -10,4 +20,4 @@\n  <same> \n-old\n+\t中🚀e\u{301}│\r\n \n tail\n@@ -30 +40 @@\n-last\n+ending\n\\ No newline at end of file\n--- /dev/null\n+++ b/two\n@@ -0,0 +1,2 @@\n+next\n+finish\n";

fn payload(line: &DiffLine) -> &str {
    match line {
        DiffLine::Add(text) | DiffLine::Remove(text) | DiffLine::Context(text) => text,
        DiffLine::NoNewline => panic!("an annotation is not source text"),
        _ => panic!("unexpected source variant"),
    }
}

fn projected_scalar(glyph: char) -> char {
    // Independent statement of the closed widget alphabet.
    if glyph == ' ' || glyph.is_ascii_graphic() || matches!(glyph, '░' | '▒' | '█' | '·') {
        glyph
    } else {
        '?'
    }
}

fn check_spans(model: &ChangeSet, data: DiffData<'_>, width: usize, height: usize) {
    let projected = diff_with_sources(data, width, height);
    assert_eq!(projected.output, diff(data, width, height));
    projected.output.validate(width, height).unwrap();
    let mut previous = None;
    for span in &projected.sources {
        assert!(span.output_row < height.saturating_sub(1));
        assert!(!span.output_columns.is_empty());
        assert_eq!(span.output_columns.len(), span.source_codepoints.len());
        assert!(span.output_columns.end <= width);
        if let Some((row, end)) = previous {
            assert!(
                row < span.output_row
                    || (row == span.output_row && end <= span.output_columns.start)
            );
        }
        previous = Some((span.output_row, span.output_columns.end));
        let hunk = &model.files()[span.file].hunks()[span.hunk];
        let line = &hunk.lines()[span.line];
        let text = payload(line);
        assert!(span.source_codepoints.end <= text.chars().count());
        let expected: String = text
            .chars()
            .skip(span.source_codepoints.start)
            .take(span.source_codepoints.len())
            .map(projected_scalar)
            .collect();
        let actual: String = projected.output.lines[span.output_row]
            .text()
            .chars()
            .skip(span.output_columns.start)
            .take(span.output_columns.len())
            .collect();
        assert_eq!(actual, expected, "{span:?}");
        let mut old = hunk.old_range().start;
        let mut new = hunk.new_range().start;
        for entry in &hunk.lines()[..span.line] {
            match entry {
                DiffLine::Remove(_) => old += 1,
                DiffLine::Add(_) => new += 1,
                DiffLine::Context(_) => {
                    old += 1;
                    new += 1;
                }
                DiffLine::NoNewline => {}
                _ => panic!("unexpected source variant"),
            }
        }
        match span.side {
            DiffSide::Old => {
                assert!(!matches!(line, DiffLine::Add(_)));
                assert_eq!(span.line_number, old);
            }
            DiffSide::New => {
                assert!(!matches!(line, DiffLine::Remove(_)));
                assert_eq!(span.line_number, new);
            }
        }
    }
}

// GUARD: visible_diff_source_spans_retain_their_model_and_side_addresses
#[test]
fn visible_diff_source_spans_retain_their_model_and_side_addresses() {
    let model = from_unified(PATCH).unwrap();
    let unified = diff_with_sources(DiffData::new(&model), 200, 40);
    assert_eq!(
        unified.sources.len(),
        8,
        "every nonempty model payload must map"
    );
    assert_eq!(unified.sources[0].source_codepoints, 0..8);
    assert_eq!(
        (unified.sources[0].side, unified.sources[0].line_number),
        (DiffSide::New, 20)
    );
    assert_eq!(
        (
            unified.sources[4].file,
            unified.sources[4].hunk,
            unified.sources[4].line
        ),
        (0, 1, 0)
    );
    assert_eq!(
        (unified.sources[6].file, unified.sources[6].line_number),
        (1, 1)
    );
    let split = diff_with_sources(DiffData::new(&model).geometry(DiffGeometry::Split), 200, 40);
    assert_eq!(split.sources.len(), 10);
    assert_eq!(
        (split.sources[0].side, split.sources[0].line_number),
        (DiffSide::Old, 10)
    );
    assert_eq!(
        (split.sources[1].side, split.sources[1].line_number),
        (DiffSide::New, 20)
    );
    assert_eq!(split.sources[0].output_row, split.sources[1].output_row);
    assert!(split.sources[1].output_columns.start > 100);
    for geometry in [
        DiffGeometry::Unified,
        DiffGeometry::Split,
        DiffGeometry::Stat,
    ] {
        for width in [0, 1, 8, 20, 200] {
            for height in [0, 1, 2, 40] {
                for row in [0, 3, usize::MAX] {
                    for column in [0, 1, 3, usize::MAX] {
                        check_spans(
                            &model,
                            DiffData::new(&model)
                                .geometry(geometry)
                                .row_offset(row)
                                .column_offset(column),
                            width,
                            height,
                        );
                    }
                }
            }
        }
    }
    assert_eq!(model.to_unified(), PATCH);
}

#[test]
fn clipping_maps_original_markers_and_spaces_but_not_injected_markers_or_suffixes() {
    let model = from_unified("--- a/f\n+++ b/f\n@@ -1 +1 @@\n-<ab cd>\n+<ab cd>\n").unwrap();
    let projected = diff_with_sources(DiffData::new(&model).column_offset(1), 10, 8);
    let old = &projected.sources[0];
    assert_eq!(old.output_columns, 6..9);
    assert_eq!(old.source_codepoints, 1..4);
    assert_eq!(projected.output.lines[old.output_row].text(), "1   -<ab >");
    check_spans(&model, DiffData::new(&model).column_offset(1), 10, 8);

    let literal = diff_with_sources(DiffData::new(&model), 40, 8);
    assert_eq!(literal.sources[0].source_codepoints, 0..7);
    let empty = from_unified("--- a/f\n+++ b/f\n@@ -1 +1 @@\n-\n+\n").unwrap();
    assert!(diff_with_sources(DiffData::new(&empty), 40, 8)
        .sources
        .is_empty());
    let escaped = from_unified("--- a/f\n+++ b/f\n@@ -1 +1 @@\n-\t\n+\u{1b}\n").unwrap();
    // The complete escaped-count suffix consumes the source body's remaining cells.
    assert!(diff_with_sources(DiffData::new(&escaped), 15, 8)
        .sources
        .is_empty());
    for (width, height) in [(0, 40), (1, 40), (40, 0), (40, 1)] {
        assert!(diff_with_sources(DiffData::new(&model), width, height)
            .sources
            .is_empty());
    }
    assert!(
        diff_with_sources(DiffData::new(&model).column_offset(usize::MAX), 40, 8)
            .sources
            .is_empty()
    );
    assert!(
        diff_with_sources(DiffData::new(&model).geometry(DiffGeometry::Stat), 40, 8)
            .sources
            .is_empty()
    );
}

#[test]
fn folds_and_windows_keep_model_locations_and_unequal_sides_keep_their_addresses() {
    let model = from_unified("--- a/f\n+++ b/f\n@@ -10,6 +20,7 @@\n a\n b\n c\n d\n-old1\n-old2\n\\ No newline at end of file\n+new1\n+new2\n+new3\n").unwrap();
    let expanded = [ContextRun {
        file: 0,
        hunk: 0,
        line: 0,
    }];
    for geometry in [DiffGeometry::Unified, DiffGeometry::Split] {
        let folded =
            diff_with_sources(DiffData::new(&model).geometry(geometry).context(1), 100, 20);
        let context_lines: Vec<_> = folded
            .sources
            .iter()
            .filter(|span| span.line < 4)
            .map(|span| span.line)
            .collect();
        assert_eq!(
            context_lines,
            if geometry == DiffGeometry::Unified {
                vec![0, 3]
            } else {
                vec![0, 0, 3, 3]
            }
        );
        let unfolded = diff_with_sources(
            DiffData::new(&model)
                .geometry(geometry)
                .context(1)
                .expanded(&expanded),
            100,
            20,
        );
        assert_eq!(
            unfolded.sources.len(),
            if geometry == DiffGeometry::Unified {
                9
            } else {
                13
            }
        );
        let third_addition = unfolded.sources.iter().find(|span| span.line == 9).unwrap();
        assert_eq!(
            (third_addition.side, third_addition.line_number),
            (DiffSide::New, 26)
        );
        for row in [0, 3, usize::MAX] {
            for column in [0, 2, usize::MAX] {
                for width in [0, 1, 8, 20, 100] {
                    check_spans(
                        &model,
                        DiffData::new(&model)
                            .geometry(geometry)
                            .context(1)
                            .row_offset(row)
                            .column_offset(column),
                        width,
                        5,
                    );
                    check_spans(
                        &model,
                        DiffData::new(&model)
                            .geometry(geometry)
                            .context(1)
                            .row_offset(row)
                            .column_offset(column)
                            .expanded(&expanded),
                        width,
                        5,
                    );
                }
            }
        }
    }
}
