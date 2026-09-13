use crate::MermaidDocument;
use newtui::{
    NoticeVisibility, Run, Tone, WidgetLine, WidgetNotice, WidgetNoticeKind, WidgetOutput,
    declared_glyph_or_replacement,
};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

#[derive(Debug, Default)]
pub(crate) struct ProjectedText {
    rows: Vec<Vec<char>>,
    replacements: usize,
}

impl ProjectedText {
    pub(crate) fn new(text: &str) -> Self {
        let mut projected = Self::default();
        for line in text.lines() {
            let mut cells = Vec::new();
            for grapheme in line.graphemes(true) {
                let affected = grapheme
                    .chars()
                    .filter(|&c| declared_glyph_or_replacement(c) != c)
                    .count();
                if affected == 0 {
                    cells.extend(grapheme.chars());
                } else {
                    projected.replacements += affected;
                    // Preserve the backend's Unicode-profile cell positions.
                    // A wide grapheme receives multiple replacement cells;
                    // a combining mark can disappear without moving its base.
                    let width = UnicodeWidthStr::width(grapheme);
                    let retained: Vec<_> = grapheme
                        .chars()
                        .filter(|&c| declared_glyph_or_replacement(c) == c)
                        .take(width)
                        .collect();
                    let missing = width - retained.len();
                    cells.extend(retained);
                    cells.extend(std::iter::repeat_n('?', missing));
                }
            }
            projected.rows.push(cells);
        }
        projected
    }
}

/// A rectangle over prepared diagram cells; offsets never modify the document.
#[derive(Debug, Clone, Copy)]
pub struct MermaidViewport<'a> {
    document: &'a MermaidDocument,
    row: usize,
    column: usize,
}

impl<'a> MermaidViewport<'a> {
    pub(crate) const fn new(document: &'a MermaidDocument) -> Self {
        Self {
            document,
            row: 0,
            column: 0,
        }
    }

    /// Skip this many logical rows, clamped to the complete document height.
    #[must_use]
    pub const fn row_offset(mut self, offset: usize) -> Self {
        self.row = offset;
        self
    }

    /// Skip this many cells per logical row, clamped independently to each row.
    #[must_use]
    pub const fn column_offset(mut self, offset: usize) -> Self {
        self.column = offset;
        self
    }

    /// Produce exactly `width` by `height` closed-glyph cells with typed notices.
    ///
    /// When diagnostics exist and cells are available, the last row is a
    /// caption. Its reserved row is included in omitted-row accounting. A
    /// caption that cannot show every complete message uses `!`; detailed
    /// notices remain returned even at zero width or height.
    #[must_use]
    pub fn render(self, width: usize, height: usize) -> WidgetOutput {
        let initial = self.notices(width, height);
        let caption = !initial.is_empty() && width > 0 && height > 0;
        let body_height = if width == 0 {
            0
        } else {
            height - usize::from(caption)
        };
        let mut notices = if caption {
            self.notices(width, body_height)
        } else {
            initial
        };
        let rows = &self.document.projected.rows;
        let start = self.row.min(rows.len());
        let mut lines = Vec::with_capacity(height);
        for row in rows.iter().skip(start).take(body_height) {
            let text = row
                .iter()
                .copied()
                .skip(self.column.min(row.len()))
                .take(width)
                .chain(std::iter::repeat(' '))
                .take(width)
                .collect::<String>();
            lines.push(line(text, Tone::Plain));
        }
        lines.resize_with(if width == 0 { height } else { body_height }, || {
            line(" ".repeat(width), Tone::Plain)
        });
        if caption {
            lines.push(caption_line(&mut notices, width));
        }
        WidgetOutput { lines, notices }
    }

    fn notices(self, width: usize, height: usize) -> Vec<WidgetNotice> {
        let projected = &self.document.projected;
        let mut notices = Vec::new();
        if let Some(state) = self.document.content_state() {
            notices.push(notice(WidgetNoticeKind::ContentState { state }));
        }
        if projected.replacements > 0 {
            notices.push(notice(WidgetNoticeKind::GlyphReplacements {
                count: projected.replacements,
            }));
        }
        let start = self.row.min(projected.rows.len());
        let visible = if width == 0 {
            0
        } else {
            height.min(projected.rows.len() - start)
        };
        let after = projected.rows.len() - start - visible;
        if start > 0 || after > 0 {
            notices.push(notice(WidgetNoticeKind::OmittedRows {
                before: start,
                after,
            }));
        }
        for (index, row) in projected.rows.iter().skip(start).take(visible).enumerate() {
            let before = self.column.min(row.len());
            let after = row.len().saturating_sub(before).saturating_sub(width);
            if before > 0 || after > 0 {
                notices.push(notice(WidgetNoticeKind::ClippedColumns {
                    row: index,
                    before,
                    after,
                }));
            }
        }
        notices
    }
}

fn notice(kind: WidgetNoticeKind) -> WidgetNotice {
    WidgetNotice {
        kind,
        visibility: NoticeVisibility::Hidden,
    }
}

fn line(text: String, tone: Tone) -> WidgetLine {
    WidgetLine::new(vec![Run::new(text, tone)])
}

fn caption_line(notices: &mut [WidgetNotice], width: usize) -> WidgetLine {
    let message = notices
        .iter()
        .map(WidgetNotice::message)
        .collect::<Vec<_>>()
        .join(" | ");
    let (text, visibility) = if message.len() <= width {
        (message, NoticeVisibility::Full)
    } else {
        ("!".into(), NoticeVisibility::Indicator)
    };
    for notice in notices {
        notice.visibility = visibility;
    }
    let text = text
        .chars()
        .chain(std::iter::repeat(' '))
        .take(width)
        .collect();
    line(text, Tone::Caution)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn closed_projection_preserves_wide_combining_and_emoji_cell_positions() {
        for source in ["|日本語|", "|café|", "|👩‍💻|", "|🇨🇦|", "|x\u{200d}y|"] {
            let projected = ProjectedText::new(source);
            assert_eq!(projected.rows[0].len(), UnicodeWidthStr::width(source));
            assert!(
                projected.rows[0]
                    .iter()
                    .all(|&c| declared_glyph_or_replacement(c) == c)
            );
            assert!(projected.replacements > 0);
            assert_eq!(projected.rows[0][0], '|');
            assert_eq!(projected.rows[0].last(), Some(&'|'));
        }
        assert_eq!(
            ProjectedText::new("|é|").rows[0].iter().collect::<String>(),
            "|e|"
        );
        assert_eq!(
            ProjectedText::new("|日|").rows[0]
                .iter()
                .collect::<String>(),
            "|??|"
        );
    }
}
