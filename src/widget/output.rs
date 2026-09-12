use std::fmt;

/// Meaning carried by a run of widget text; the host chooses its palette.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum Tone {
    /// No visual emphasis.
    #[default]
    Plain,
    /// Supporting text or empty capacity.
    Muted,
    /// A label that identifies the data beside it.
    Label,
    /// A structural marker or independently highlighted value.
    Accent,
    /// A value in its ordinary range.
    Healthy,
    /// A value nearing its declared limit.
    Caution,
    /// A value at or beyond its declared limit.
    Critical,
    /// Source present on the new side of a change.
    Added,
    /// Source present on the old side of a change.
    Removed,
    /// Unchanged source shared by both sides.
    Context,
    /// A change's hunk header.
    Hunk,
}

/// How completely a diagnostic is represented inside the requested rectangle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum NoticeVisibility {
    /// The complete diagnostic message fits.
    Full,
    /// An indicator is visible; the host can show the full diagnostic elsewhere.
    Indicator,
    /// No diagnostic cell fits; the host must use the returned data.
    Hidden,
}

/// Renderer-independent facts about a widget's presentation, without source text.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum WidgetNoticeKind {
    /// Undeclared source codepoints replaced before clipping or pane duplication.
    GlyphReplacements {
        /// Number of replaced codepoint occurrences.
        count: usize,
    },
    /// Logical rows outside the visible viewport, excluding deliberately folded rows.
    OmittedRows {
        /// Rows preceding the viewport.
        before: usize,
        /// Rows following the viewport.
        after: usize,
    },
    /// Source rows deliberately summarized by a fold marker.
    FoldedRows {
        /// Number of summarized source rows.
        count: usize,
    },
    /// Columns outside a displayed row's source window, summed across its panes.
    ClippedColumns {
        /// Zero-based output row.
        row: usize,
        /// Columns preceding the source window.
        before: usize,
        /// Columns following the source window.
        after: usize,
    },
    /// A requested layout could not fit; names describe layouts, never content.
    LayoutFallback {
        /// Requested layout name.
        requested: &'static str,
        /// Layout actually used.
        rendered: &'static str,
    },
}

/// A diagnostic survives even when a widget has no display cells.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct WidgetNotice {
    /// Exact presentation fact, independent of any abbreviated visible notice.
    pub kind: WidgetNoticeKind,
    /// Whether its full message, an indicator, or neither fits inside the widget.
    pub visibility: NoticeVisibility,
}

impl WidgetNotice {
    /// Format the complete diagnostic for a host's caption or accessible text.
    #[must_use]
    pub fn message(&self) -> String {
        match self.kind {
            WidgetNoticeKind::GlyphReplacements { count } => format!("{count} escaped"),
            WidgetNoticeKind::OmittedRows { before, after } => {
                format!("{before} rows above, {after} below")
            }
            WidgetNoticeKind::FoldedRows { count } => format!("{count} unchanged folded"),
            WidgetNoticeKind::ClippedColumns { row, before, after } => {
                format!("row {row}: {before} columns left, {after} right")
            }
            WidgetNoticeKind::LayoutFallback {
                requested,
                rendered,
            } => format!("{requested} -> {rendered}"),
        }
    }
}

/// Contiguous text with one semantic tone.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Run {
    /// Text emitted by the widget.
    pub text: String,
    /// Meaning a renderer maps to its own style.
    pub tone: Tone,
}

impl Run {
    /// Construct one styled run.
    pub fn new(text: impl Into<String>, tone: Tone) -> Self {
        Self {
            text: text.into(),
            tone,
        }
    }
}

/// One display line, retained as runs so semantic style boundaries survive.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct WidgetLine {
    /// Runs in display order.
    pub runs: Vec<Run>,
}

impl WidgetLine {
    /// Construct a line from its runs.
    #[must_use]
    pub fn new(runs: Vec<Run>) -> Self {
        Self { runs }
    }

    pub(crate) fn plain(text: impl Into<String>) -> Self {
        Self::new(vec![Run::new(text, Tone::Plain)])
    }

    /// Flatten the line when styling is not needed.
    #[must_use]
    pub fn text(&self) -> String {
        self.runs.iter().map(|run| run.text.as_str()).collect()
    }
}

/// A rectangular renderer-neutral widget result.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct WidgetOutput {
    /// Lines in top-to-bottom display order.
    pub lines: Vec<WidgetLine>,
    /// Exact diagnostics, including those with no room for an on-screen notice.
    pub notices: Vec<WidgetNotice>,
}

impl WidgetOutput {
    /// Construct output from lines.
    #[must_use]
    pub fn new(lines: Vec<WidgetLine>) -> Self {
        Self {
            lines,
            notices: Vec::new(),
        }
    }

    /// Check the shape and the closed single-column glyph vocabulary.
    ///
    /// # Errors
    ///
    /// Returns the first line-count, width, or undeclared-glyph defect.
    pub fn validate(&self, width: usize, height: usize) -> Result<(), WidgetOutputError> {
        if self.lines.len() != height {
            return Err(WidgetOutputError::Height {
                expected: height,
                actual: self.lines.len(),
            });
        }
        for (line, rendered) in self.lines.iter().enumerate() {
            let text = rendered.text();
            let actual = text.chars().count();
            if actual != width {
                return Err(WidgetOutputError::Width {
                    line,
                    expected: width,
                    actual,
                });
            }
            if let Some(glyph) = text.chars().find(|glyph| !is_declared_glyph(*glyph)) {
                return Err(WidgetOutputError::Glyph { line, glyph });
            }
        }
        Ok(())
    }
}

/// Why widget output cannot occupy its declared rectangle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WidgetOutputError {
    /// The widget emitted the wrong number of lines.
    Height {
        /// Height requested by the caller.
        expected: usize,
        /// Lines actually emitted.
        actual: usize,
    },
    /// A line occupies the wrong number of display columns.
    Width {
        /// Zero-based line containing the defect.
        line: usize,
        /// Width requested by the caller.
        expected: usize,
        /// Columns actually emitted.
        actual: usize,
    },
    /// A glyph is outside the closed, known-single-column vocabulary.
    Glyph {
        /// Zero-based line containing the defect.
        line: usize,
        /// Undeclared glyph that made column counting unknowable.
        glyph: char,
    },
}

impl fmt::Display for WidgetOutputError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for WidgetOutputError {}

pub(crate) fn is_declared_glyph(glyph: char) -> bool {
    glyph == ' '
        || glyph.is_ascii_graphic()
        || matches!(glyph, '\u{2591}' | '\u{2592}' | '\u{2588}' | '\u{00b7}')
}

pub(crate) fn declared_glyph_or_replacement(glyph: char) -> char {
    if is_declared_glyph(glyph) {
        glyph
    } else {
        '?'
    }
}

/// Convert widget data without fixing a palette in the reusable crate.
#[cfg(feature = "ratatui")]
pub fn ratatui_lines(
    output: &WidgetOutput,
    style: impl Fn(Tone) -> ratatui::style::Style,
) -> Vec<ratatui::text::Line<'static>> {
    output
        .lines
        .iter()
        .map(|line| {
            ratatui::text::Line::from(
                line.runs
                    .iter()
                    .map(|run| ratatui::text::Span::styled(run.text.clone(), style(run.tone)))
                    .collect::<Vec<_>>(),
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    // GUARD: widget::output::tests::an_empty_result_cannot_pass_a_nonempty_height — this is a guard; tests/mutations.rs must show it red.
    #[test]
    fn an_empty_result_cannot_pass_a_nonempty_height() {
        let empty = WidgetOutput::new(vec![]);
        assert_eq!(
            empty.validate(8, 1),
            Err(WidgetOutputError::Height {
                expected: 1,
                actual: 0
            })
        );
    }

    // GUARD: widget::output::tests::every_line_must_use_the_declared_width — this is a guard; tests/mutations.rs must show it red.
    #[test]
    fn every_line_must_use_the_declared_width() {
        let short = WidgetOutput::new(vec![WidgetLine::plain("seven")]);
        assert_eq!(
            short.validate(8, 1),
            Err(WidgetOutputError::Width {
                line: 0,
                expected: 8,
                actual: 5
            })
        );
    }

    // GUARD: widget::output::tests::the_single_column_alphabet_is_closed — this is a guard; tests/mutations.rs must show it red.
    #[test]
    fn the_single_column_alphabet_is_closed() {
        let emoji = WidgetOutput::new(vec![WidgetLine::plain("1234567\u{1f680}")]);
        assert_eq!(
            emoji.validate(8, 1),
            Err(WidgetOutputError::Glyph {
                line: 0,
                glyph: '\u{1f680}'
            })
        );
    }
}
