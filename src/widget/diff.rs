use crate::diff::{ChangeSet, DiffLine, FileChange, FileKind, Hunk};

use super::output::{declared_glyph_or_replacement, is_declared_glyph};
use super::{
    bar, NoticeVisibility, Run, Tone, WidgetLine, WidgetNotice, WidgetNoticeKind, WidgetOutput,
};

/// Presentation of a supplied change; no geometry computes or applies a diff.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum DiffGeometry {
    /// One stream with old and new address gutters.
    #[default]
    Unified,
    /// Old and new panes, paired by position within each edit block.
    Split,
    /// File counts and separate addition/removal bars.
    Stat,
}

/// Stable address of the first context entry in a contiguous model run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ContextRun {
    /// Zero-based file index.
    pub file: usize,
    /// Zero-based hunk index.
    pub hunk: usize,
    /// Zero-based index into the hunk's `DiffLine` entries, including annotations.
    pub line: usize,
}

/// Borrowed content and a host-supplied presentation request, with no interaction state.
#[derive(Debug, Clone, Copy)]
pub struct DiffData<'a> {
    changes: &'a ChangeSet,
    geometry: DiffGeometry,
    row: usize,
    column: usize,
    context: usize,
    expanded: &'a [ContextRun],
}

impl<'a> DiffData<'a> {
    /// Unified presentation, zero offsets, three context lines at each run end.
    #[must_use]
    pub fn new(changes: &'a ChangeSet) -> Self {
        Self {
            changes,
            geometry: DiffGeometry::Unified,
            row: 0,
            column: 0,
            context: 3,
            expanded: &[],
        }
    }

    /// Select the requested geometry; narrow split output reports its fallback.
    #[must_use]
    pub fn geometry(mut self, geometry: DiffGeometry) -> Self {
        self.geometry = geometry;
        self
    }

    /// First projected row after folding, excluding the reserved summary footer.
    #[must_use]
    pub fn row_offset(mut self, row: usize) -> Self {
        self.row = row;
        self
    }

    /// First source codepoint in both panes; gutters and notices stay fixed.
    #[must_use]
    pub fn column_offset(mut self, column: usize) -> Self {
        self.column = column;
        self
    }

    /// Context entries retained at each run end; `usize::MAX` disables folding.
    #[must_use]
    pub fn context(mut self, context: usize) -> Self {
        self.context = context;
        self
    }

    /// Individual context runs the host has expanded, independent of screen geometry.
    #[must_use]
    pub fn expanded(mut self, expanded: &'a [ContextRun]) -> Self {
        self.expanded = expanded;
        self
    }
}

#[derive(Clone, Copy)]
struct Source<'a> {
    old: Option<u32>,
    new: Option<u32>,
    text: &'a str,
    tone: Tone,
    eof: bool,
    index: usize,
}

enum Row<'a> {
    Text(Vec<Run>),
    Source(Source<'a>),
    Pair(Option<Source<'a>>, Option<Source<'a>>),
    Annotation(bool, bool),
    Bars(usize, usize, usize),
}

fn text_row(text: impl Into<String>, tone: Tone) -> Row<'static> {
    Row::Text(vec![Run::new(text, tone)])
}

fn replacements(text: &str) -> usize {
    text.chars()
        .filter(|glyph| !is_declared_glyph(*glyph))
        .count()
}

fn notice(kind: WidgetNoticeKind) -> WidgetNotice {
    WidgetNotice {
        kind,
        visibility: NoticeVisibility::Hidden,
    }
}

/// Render a change into an exact rectangle with diagnostics that survive even 0×0.
///
/// The footer always occupies the last requested row. Source glyph counts are
/// codepoint occurrences in the logical presentation before clipping, folding,
/// or split-pane duplication. Stat counts only its displayed headers/metadata.
/// The model remains untouched. `escaped` means substitution with `?`, not a
/// reversible encoding; use the model's text faces for original source.
#[must_use]
pub fn diff(data: DiffData<'_>, width: usize, height: usize) -> WidgetOutput {
    let (rows, escaped, folded) = project(data);
    let (old_digits, new_digits) = digits(data.changes);
    let split = data.geometry == DiffGeometry::Split
        && width.saturating_sub(3) / 2 >= old_digits + 4
        && width.saturating_sub(3).div_ceil(2) >= new_digits + 4;
    let mut notices = Vec::new();
    if escaped > 0 {
        notices.push(notice(WidgetNoticeKind::GlyphReplacements {
            count: escaped,
        }));
    }
    if folded > 0 {
        notices.push(notice(WidgetNoticeKind::FoldedRows { count: folded }));
    }
    if data.geometry == DiffGeometry::Split && !split {
        notices.push(notice(WidgetNoticeKind::LayoutFallback {
            requested: "split",
            rendered: "unified",
        }));
    }
    let numbered =
        split || data.geometry == DiffGeometry::Stat || width > old_digits + new_digits + 3;
    if !numbered {
        notices.push(notice(WidgetNoticeKind::LayoutFallback {
            requested: "numbered",
            rendered: "compact",
        }));
    }
    let rows = if split { pair(rows) } else { annotate(rows) };
    let capacity = if width == 0 {
        0
    } else {
        height.saturating_sub(1)
    };
    let start = data.row.min(rows.len().saturating_sub(capacity.max(1)));
    let shown = capacity.min(rows.len().saturating_sub(start));
    let after = rows.len().saturating_sub(start + shown);
    if start > 0 || after > 0 {
        notices.push(notice(WidgetNoticeKind::OmittedRows {
            before: start,
            after,
        }));
    }
    let mut lines = Vec::with_capacity(height);
    for (index, row) in rows.iter().skip(start).take(shown).enumerate() {
        let (line, before, after) = render_row(
            row,
            width,
            data.column,
            old_digits,
            new_digits,
            numbered,
            split,
        );
        if before > 0 || after > 0 {
            notices.push(notice(WidgetNoticeKind::ClippedColumns {
                row: index,
                before,
                after,
            }));
        }
        lines.push(line);
    }
    while lines.len() < height.saturating_sub(1) {
        lines.push(WidgetLine::new(vec![Run::new(
            " ".repeat(width),
            Tone::Plain,
        )]));
    }
    if height > 0 {
        lines.push(footer(&mut notices, width, data.changes.files().is_empty()));
    }
    WidgetOutput { lines, notices }
}

fn digits(changes: &ChangeSet) -> (usize, usize) {
    let mut old = 1;
    let mut new = 1;
    for hunk in changes.files().iter().flat_map(FileChange::hunks) {
        old = old.max(hunk.old_range().end.saturating_sub(1));
        new = new.max(hunk.new_range().end.saturating_sub(1));
    }
    (old.to_string().len(), new.to_string().len())
}

fn project(data: DiffData<'_>) -> (Vec<Row<'_>>, usize, usize) {
    let mut rows = Vec::new();
    let mut escaped = 0;
    let mut folded = 0;
    let maximum = data
        .changes
        .files()
        .iter()
        .map(|file| file.additions().max(file.removals()))
        .max()
        .unwrap_or(0);
    for (file_index, file) in data.changes.files().iter().enumerate() {
        let label = match file.kind() {
            FileKind::Added => format!("Added {}", file.path().new_path()),
            FileKind::Deleted => format!("Deleted {}", file.path().old_path()),
            FileKind::Renamed => format!(
                "Renamed {} -> {}",
                file.path().old_path(),
                file.path().new_path()
            ),
            FileKind::Copied => format!(
                "Copied {} -> {}",
                file.path().old_path(),
                file.path().new_path()
            ),
            FileKind::Modified => format!("Modified {}", file.path().new_path()),
        };
        escaped += replacements(&label);
        rows.push(Row::Text(vec![
            Run::new(label, Tone::Label),
            Run::new(format!(" (+{}", file.additions()), Tone::Added),
            Run::new(format!(" -{})", file.removals()), Tone::Removed),
        ]));
        for metadata in file.metadata() {
            escaped += replacements(metadata);
            rows.push(text_row(metadata, Tone::Muted));
        }
        if file.is_binary() {
            rows.push(text_row(
                "Binary files differ (no text-line counts)",
                Tone::Muted,
            ));
        }
        if file.hunks().is_empty() && !file.is_binary() {
            rows.push(text_row("No text hunks", Tone::Muted));
        }
        if data.geometry == DiffGeometry::Stat {
            rows.push(Row::Bars(file.additions(), file.removals(), maximum));
            continue;
        }
        for (hunk_index, hunk) in file.hunks().iter().enumerate() {
            let (hunk_rows, hunk_escaped, hunk_folded) =
                project_hunk(data, file_index, hunk_index, hunk);
            rows.extend(hunk_rows);
            escaped += hunk_escaped;
            folded += hunk_folded;
        }
    }
    (rows, escaped, folded)
}

fn project_hunk<'a>(
    data: DiffData<'a>,
    file_index: usize,
    hunk_index: usize,
    hunk: &'a Hunk,
) -> (Vec<Row<'a>>, usize, usize) {
    let mut rows = Vec::new();
    let mut escaped = 0;
    let mut folded = 0;
    let old = hunk.old_range();
    let new = hunk.new_range();
    let heading = format!(
        "@@ -{},{} +{},{} @@{}",
        old.start,
        old.end - old.start,
        new.start,
        new.end - new.start,
        hunk.section()
    );
    escaped += replacements(&heading);
    rows.push(text_row(heading, Tone::Hunk));
    let mut old = old.start;
    let mut new = new.start;
    let mut sources = Vec::new();
    for (index, line) in hunk.lines().iter().enumerate() {
        let (text, tone, old_address, new_address) = match line {
            DiffLine::Add(text) => {
                let address = new;
                new += 1;
                (text, Tone::Added, None, Some(address))
            }
            DiffLine::Remove(text) => {
                let address = old;
                old += 1;
                (text, Tone::Removed, Some(address), None)
            }
            DiffLine::Context(text) => {
                let addresses = (old, new);
                old += 1;
                new += 1;
                (text, Tone::Context, Some(addresses.0), Some(addresses.1))
            }
            DiffLine::NoNewline => continue,
        };
        escaped += replacements(text);
        sources.push(Source {
            old: old_address,
            new: new_address,
            text,
            tone,
            eof: matches!(hunk.lines().get(index + 1), Some(DiffLine::NoNewline)),
            index,
        });
    }
    let mut at = 0;
    while at < sources.len() {
        let source = sources[at];
        if source.tone != Tone::Context || source.eof {
            rows.push(Row::Source(source));
            at += 1;
            continue;
        }
        let end = at
            + sources[at..]
                .iter()
                .take_while(|source| source.tone == Tone::Context && !source.eof)
                .count();
        let count = end - at;
        let expanded = data.expanded.contains(&ContextRun {
            file: file_index,
            hunk: hunk_index,
            line: source.index,
        });
        if !expanded && count > data.context.saturating_mul(2).saturating_add(1) {
            rows.extend(
                sources[at..at + data.context]
                    .iter()
                    .copied()
                    .map(Row::Source),
            );
            let hidden = count - data.context * 2;
            rows.push(text_row(format!("... {hidden} unchanged"), Tone::Muted));
            folded += hidden;
            rows.extend(
                sources[end - data.context..end]
                    .iter()
                    .copied()
                    .map(Row::Source),
            );
        } else {
            rows.extend(sources[at..end].iter().copied().map(Row::Source));
        }
        at = end;
    }
    (rows, escaped, folded)
}

fn annotate(rows: Vec<Row<'_>>) -> Vec<Row<'_>> {
    let mut output = Vec::new();
    for row in rows {
        let annotation = match &row {
            Row::Source(source) if source.eof => Some((source.old.is_some(), source.new.is_some())),
            _ => None,
        };
        output.push(row);
        if let Some((old, new)) = annotation {
            output.push(Row::Annotation(old, new));
        }
    }
    output
}

fn pair(rows: Vec<Row<'_>>) -> Vec<Row<'_>> {
    let mut output = Vec::new();
    let mut rows = rows.into_iter().peekable();
    while let Some(row) = rows.next() {
        match row {
            Row::Source(source) if source.tone == Tone::Context => {
                push_pair(&mut output, Some(source), Some(source));
            }
            Row::Source(source) => {
                let mut old = Vec::new();
                let mut new = Vec::new();
                if source.old.is_some() {
                    old.push(source);
                } else {
                    new.push(source);
                }
                while matches!(rows.peek(), Some(Row::Source(source)) if source.tone != Tone::Context)
                {
                    if let Some(Row::Source(source)) = rows.next() {
                        if source.old.is_some() {
                            old.push(source);
                        } else {
                            new.push(source);
                        }
                    }
                }
                for index in 0..old.len().max(new.len()) {
                    push_pair(
                        &mut output,
                        old.get(index).copied(),
                        new.get(index).copied(),
                    );
                }
            }
            row => output.push(row),
        }
    }
    output
}

fn push_pair<'a>(rows: &mut Vec<Row<'a>>, old: Option<Source<'a>>, new: Option<Source<'a>>) {
    rows.push(Row::Pair(old, new));
    let old_eof = old.is_some_and(|source| source.eof);
    let new_eof = new.is_some_and(|source| source.eof);
    if old_eof || new_eof {
        rows.push(Row::Annotation(old_eof, new_eof));
    }
}

fn address(number: Option<u32>, digits: usize) -> String {
    number.map_or_else(|| " ".repeat(digits), |number| format!("{number:>digits$}"))
}

fn prefix(source: Source<'_>) -> char {
    match source.tone {
        Tone::Added => '+',
        Tone::Removed => '-',
        _ => ' ',
    }
}

fn source_line(
    source: Source<'_>,
    width: usize,
    offset: usize,
    gutter: String,
) -> (WidgetLine, usize, usize) {
    let remaining = width.saturating_sub(gutter.len());
    let escaped = replacements(source.text);
    let suffix = if escaped > 0 {
        format!(" {escaped} escaped")
    } else {
        String::new()
    };
    let suffix = if suffix.len() > remaining {
        "!".repeat(remaining.min(1))
    } else {
        suffix
    };
    let body_width = remaining.saturating_sub(suffix.len());
    let (body, before, after) = window(source.text, body_width, offset);
    let mut runs = vec![Run::new(gutter, Tone::Muted), Run::new(body, source.tone)];
    if !suffix.is_empty() {
        runs.push(Run::new(suffix, Tone::Muted));
    }
    (WidgetLine::new(runs), before, after)
}

fn window(text: &str, width: usize, offset: usize) -> (String, usize, usize) {
    let count = text.chars().count();
    let before = offset.min(count);
    let leading = usize::from(before > 0 && width > 0);
    let capacity = width.saturating_sub(leading);
    let trailing = usize::from(count - before > capacity && capacity > 0);
    let shown = (count - before).min(capacity.saturating_sub(trailing));
    let mut output = String::new();
    if leading > 0 {
        output.push('<');
    }
    output.extend(
        text.chars()
            .skip(before)
            .take(shown)
            .map(declared_glyph_or_replacement),
    );
    if trailing > 0 {
        output.push('>');
    }
    output.extend(std::iter::repeat_n(' ', width - output.chars().count()));
    (output, before, count - before - shown)
}

fn render_row(
    row: &Row<'_>,
    width: usize,
    offset: usize,
    old_digits: usize,
    new_digits: usize,
    numbered: bool,
    split: bool,
) -> (WidgetLine, usize, usize) {
    match row {
        Row::Text(runs) => styled_window(runs, width),
        Row::Source(source) => {
            let gutter = if numbered {
                format!(
                    "{} {} {}",
                    address(source.old, old_digits),
                    address(source.new, new_digits),
                    prefix(*source)
                )
            } else {
                prefix(*source).to_string().chars().take(width).collect()
            };
            source_line(*source, width, offset, gutter)
        }
        Row::Pair(old, new) => {
            let left_width = (width - 3) / 2;
            let right_width = width - 3 - left_width;
            let pane = |source: Option<Source<'_>>, width, digits, old_side| {
                source.map_or_else(
                    || {
                        (
                            WidgetLine::new(vec![Run::new(" ".repeat(width), Tone::Plain)]),
                            0,
                            0,
                        )
                    },
                    |source| {
                        source_line(
                            source,
                            width,
                            offset,
                            format!(
                                "{} {}",
                                address(if old_side { source.old } else { source.new }, digits),
                                prefix(source)
                            ),
                        )
                    },
                )
            };
            let (mut left, before_left, after_left) = pane(*old, left_width, old_digits, true);
            let (right, before_right, after_right) = pane(*new, right_width, new_digits, false);
            left.runs.push(Run::new(" | ", Tone::Accent));
            left.runs.extend(right.runs);
            (left, before_left + before_right, after_left + after_right)
        }
        Row::Annotation(old, new) => {
            let marker = "\\ No newline at end of file";
            if split {
                let left_width = (width - 3) / 2;
                let right_width = width - 3 - left_width;
                let (left, _, lost_left) = window(if *old { marker } else { "" }, left_width, 0);
                let (right, _, lost_right) = window(if *new { marker } else { "" }, right_width, 0);
                (
                    WidgetLine::new(vec![
                        Run::new(left, Tone::Muted),
                        Run::new(" | ", Tone::Accent),
                        Run::new(right, Tone::Muted),
                    ]),
                    0,
                    lost_left + lost_right,
                )
            } else {
                styled_window(&[Run::new(marker, Tone::Muted)], width)
            }
        }
        Row::Bars(added, removed, maximum) => {
            let left_width = width / 2;
            let right_width = width - left_width;
            let (mut left, lost_left) = stat_bar(*added, *maximum, "+", left_width, Tone::Added);
            let (right, lost_right) = stat_bar(*removed, *maximum, "-", right_width, Tone::Removed);
            left.runs.extend(right.runs);
            (left, 0, lost_left + lost_right)
        }
    }
}

fn styled_window(runs: &[Run], width: usize) -> (WidgetLine, usize, usize) {
    let count: usize = runs.iter().map(|run| run.text.chars().count()).sum();
    let escaped: usize = runs.iter().map(|run| replacements(&run.text)).sum();
    let suffix = if escaped > 0 {
        format!(" {escaped} escaped")
    } else {
        String::new()
    };
    let suffix = if suffix.len() > width {
        "!".repeat(width.min(1))
    } else {
        suffix
    };
    let body_width = width - suffix.len();
    let clipped = count > body_width;
    let mut remaining = body_width.saturating_sub(usize::from(clipped && body_width > 0));
    let shown = count.min(remaining);
    let mut output = Vec::new();
    for run in runs {
        let text: String = run
            .text
            .chars()
            .map(declared_glyph_or_replacement)
            .take(remaining)
            .collect();
        remaining -= text.chars().count();
        if !text.is_empty() {
            output.push(Run::new(text, run.tone));
        }
    }
    if clipped && body_width > 0 {
        output.push(Run::new(">", Tone::Muted));
    }
    if remaining > 0 {
        output.push(Run::new(" ".repeat(remaining), Tone::Plain));
    }
    if !suffix.is_empty() {
        output.push(Run::new(suffix, Tone::Muted));
    }
    (WidgetLine::new(output), 0, count - shown)
}

fn stat_bar(
    value: usize,
    maximum: usize,
    label: &str,
    width: usize,
    tone: Tone,
) -> (WidgetLine, usize) {
    // Counts are bounded by in-memory source entries; terminal bar proportions
    // need no exact integer representation above f64's mantissa range.
    #[allow(clippy::cast_precision_loss)]
    let mut output = bar(label, value as f64, maximum as f64, "", width, 1);
    for run in &mut output.lines[0].runs {
        run.tone = tone;
    }
    (output.lines.remove(0), 3usize.saturating_sub(width))
}

fn footer(notices: &mut [WidgetNotice], width: usize, empty: bool) -> WidgetLine {
    if width == 0 {
        return WidgetLine::default();
    }
    if notices.is_empty() {
        return styled_window(
            &[Run::new(
                if empty { "No changes" } else { "End of change" },
                Tone::Muted,
            )],
            width,
        )
        .0;
    }
    for notice in notices.iter_mut() {
        notice.visibility = NoticeVisibility::Indicator;
    }
    let mut caption = String::new();
    for index in 0..notices.len() {
        let message = notices[index].message();
        let separator = if caption.is_empty() { "" } else { "; " };
        let reserve = usize::from(index + 1 < notices.len()) * 2;
        if caption.len() + separator.len() + message.len() + reserve > width {
            if !caption.is_empty() && caption.len() + 2 <= width {
                caption.push(' ');
            }
            caption.push('!');
            break;
        }
        caption.push_str(separator);
        caption.push_str(&message);
        notices[index].visibility = NoticeVisibility::Full;
    }
    caption.push_str(&" ".repeat(width - caption.len()));
    WidgetLine::new(vec![Run::new(caption, Tone::Muted)])
}
