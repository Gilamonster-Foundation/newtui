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

/// Which complete file a host uses to style a projected source fragment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DiffSide {
    /// The file before the change.
    Old,
    /// The file after the change.
    New,
}

/// A visible source fragment, with no source text or styling policy attached.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DiffSourceSpan {
    /// Zero-based row in the returned output, after vertical scrolling.
    pub output_row: usize,
    /// Absolute columns in that row, including the split pane's horizontal position.
    pub output_columns: std::ops::Range<usize>,
    /// Zero-based file index in the supplied change set.
    pub file: usize,
    /// Zero-based hunk index in that file.
    pub hunk: usize,
    /// Zero-based `DiffLine` index, including any preceding newline annotations.
    pub line: usize,
    /// Unicode scalar indices in the original line payload, excluding its diff prefix.
    /// Each scalar corresponds to one output cell, including a substituted `?`.
    pub source_codepoints: std::ops::Range<usize>,
    /// Old or new file; unified context uses the new side.
    pub side: DiffSide,
    /// The actual one-based line number in that side's file.
    pub line_number: u32,
}

/// Diff cells and the original source fragments they display.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffProjection {
    /// The same rectangle and notices returned by [`diff`].
    pub output: WidgetOutput,
    /// Nonempty, disjoint fragments in output row/column order.
    /// Gutters, padding, synthetic clip markers, annotations and notices have no span.
    pub sources: Vec<DiffSourceSpan>,
}

/// Project once and retain source locations for a host's optional highlighter.
///
/// Original spaces and literal `<`/`>` characters have locations; synthetic
/// markers do not. Split context maps once per side. The source ranges use
/// Unicode scalars, so a tokenizer using UTF-8 byte offsets must convert them.
/// The host owns multiline syntax state and can apply foreground colors while
/// preserving the widget's semantic background tones. No source enters a `View`.
#[must_use]
pub fn diff_with_sources(data: DiffData<'_>, width: usize, height: usize) -> DiffProjection {
    project_diff(data, width, height)
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

/// The model entry a projected row addresses, with no source text attached.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum DiffTarget {
    /// A file's label, metadata, or binary/empty notice.
    File {
        /// Zero-based file index in the supplied change set.
        file: usize,
    },
    /// A hunk heading.
    Hunk {
        /// Zero-based file index.
        file: usize,
        /// Zero-based hunk index in that file.
        hunk: usize,
    },
    /// A source line and its actual line numbers on each side it exists.
    Line {
        /// Zero-based file index.
        file: usize,
        /// Zero-based hunk index.
        hunk: usize,
        /// Zero-based `DiffLine` index, including annotations.
        line: usize,
        /// One-based line number in the old file, if the line exists there.
        old: Option<u32>,
        /// One-based line number in the new file, if the line exists there.
        new: Option<u32>,
    },
    /// The missing-final-newline marker after the line at `line`.
    Annotation {
        /// Zero-based file index.
        file: usize,
        /// Zero-based hunk index.
        hunk: usize,
        /// The `DiffLine` index of the source line the marker follows.
        line: usize,
    },
    /// A folded context run and the `DiffLine` entries it hides.
    Fold {
        /// The run a host passes to [`DiffData::expanded`] to reveal it.
        run: ContextRun,
        /// Hidden `DiffLine` indices in that hunk.
        entries: std::ops::Range<usize>,
    },
    /// A file's addition/removal bars in the stat geometry.
    Stat {
        /// Zero-based file index.
        file: usize,
    },
}

/// What one projected row addresses on each side; `None` where the side has nothing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffLayoutRow {
    /// Target in the old file or the shared row.
    pub old: Option<DiffTarget>,
    /// Target in the new file or the shared row.
    pub new: Option<DiffTarget>,
}

/// Every row a width produces, before scrolling, with no source text retained.
///
/// Two changes with the same shape and different text lay out identically, so
/// a host can keep this for navigation without keeping a second copy of source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffLayout {
    /// Rows in projection order, excluding the reserved summary footer.
    pub rows: Vec<DiffLayoutRow>,
    /// The geometry actually rendered, after any narrow split fallback.
    pub geometry: DiffGeometry,
    /// The single side rendered, if any; split and stat always show both.
    pub pane: Option<DiffSide>,
}

impl DiffLayout {
    /// Rows visible for a row offset in a `width` x `height` rectangle.
    ///
    /// Matches [`diff`]: the footer takes the last row, and a zero-width
    /// rectangle shows none.
    #[must_use]
    pub fn window(&self, row: usize, width: usize, height: usize) -> std::ops::Range<usize> {
        let capacity = if width == 0 {
            0
        } else {
            height.saturating_sub(1)
        };
        let start = row.min(self.rows.len().saturating_sub(capacity.max(1)));
        start..start + capacity.min(self.rows.len().saturating_sub(start))
    }
}

/// A requested file index that the change set does not contain.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DiffFileError {
    /// The requested zero-based index.
    pub file: usize,
    /// How many files the change set has.
    pub files: usize,
}

impl std::fmt::Display for DiffFileError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "file {} is outside a change set of {} files",
            self.file, self.files
        )
    }
}

impl std::error::Error for DiffFileError {}

/// Borrowed content and a host-supplied presentation request, with no interaction state.
#[derive(Debug, Clone, Copy)]
pub struct DiffData<'a> {
    changes: &'a ChangeSet,
    geometry: DiffGeometry,
    row: usize,
    column: usize,
    context: usize,
    expanded: &'a [ContextRun],
    file: Option<usize>,
    pane: Option<DiffSide>,
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
            file: None,
            pane: None,
        }
    }

    /// Show one file; every address keeps its index in the whole change set.
    ///
    /// # Errors
    ///
    /// Returns [`DiffFileError`] when the change set has no such file.
    pub fn file(mut self, file: usize) -> Result<Self, DiffFileError> {
        let files = self.changes.files().len();
        if file >= files {
            return Err(DiffFileError { file, files });
        }
        self.file = Some(file);
        Ok(self)
    }

    /// Show only one side as a pane; split and stat geometries ignore it.
    #[must_use]
    pub fn pane(mut self, side: DiffSide) -> Self {
        self.pane = Some(side);
        self
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
    file: usize,
    hunk: usize,
    index: usize,
}

struct RenderedRow {
    line: WidgetLine,
    before: usize,
    after: usize,
    sources: Vec<DiffSourceSpan>,
}

impl RenderedRow {
    fn without_source((line, before, after): (WidgetLine, usize, usize)) -> Self {
        Self {
            line,
            before,
            after,
            sources: Vec::new(),
        }
    }
}

/// Model address of a source line: file, hunk, and `DiffLine` index.
type Address = (usize, usize, usize);

enum Row<'a> {
    Text(Vec<Run>, DiffTarget),
    Source(Source<'a>),
    Pair(Option<Source<'a>>, Option<Source<'a>>),
    Annotation(Option<Address>, Option<Address>),
    Bars(usize, usize, usize, usize),
}

fn text_row(text: impl Into<String>, tone: Tone, target: DiffTarget) -> Row<'static> {
    Row::Text(vec![Run::new(text, tone)], target)
}

impl Source<'_> {
    fn address(self) -> Address {
        (self.file, self.hunk, self.index)
    }

    fn target(self) -> DiffTarget {
        DiffTarget::Line {
            file: self.file,
            hunk: self.hunk,
            line: self.index,
            old: self.old,
            new: self.new,
        }
    }
}

impl Row<'_> {
    fn layout(&self, pane: Option<DiffSide>) -> DiffLayoutRow {
        let annotation = |address: Option<Address>| {
            address.map(|(file, hunk, line)| DiffTarget::Annotation { file, hunk, line })
        };
        let (old, new) = match self {
            Row::Text(_, target) => (Some(target.clone()), Some(target.clone())),
            Row::Source(source) => (
                source.old.map(|_| source.target()),
                source.new.map(|_| source.target()),
            ),
            Row::Pair(old, new) => (old.map(Source::target), new.map(Source::target)),
            Row::Annotation(old, new) => (annotation(*old), annotation(*new)),
            Row::Bars(file, ..) => (
                Some(DiffTarget::Stat { file: *file }),
                Some(DiffTarget::Stat { file: *file }),
            ),
        };
        match pane {
            None => DiffLayoutRow { old, new },
            Some(DiffSide::Old) => DiffLayoutRow { old, new: None },
            Some(DiffSide::New) => DiffLayoutRow { old: None, new },
        }
    }
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
    diff_with_sources(data, width, height).output
}

/// Lay out every row a width produces, addressing the model without copying source.
///
/// This is the same projection [`diff`] renders: the same folding, split
/// fallback, and single-side pane, so `rows[i]` describes output row `i` before
/// scrolling. Use [`DiffLayout::window`] for the rows a rectangle shows.
#[must_use]
pub fn diff_layout(data: DiffData<'_>, width: usize) -> DiffLayout {
    let arranged = arrange(data, width);
    DiffLayout {
        rows: arranged
            .rows
            .iter()
            .map(|row| row.layout(arranged.pane))
            .collect(),
        geometry: arranged.geometry,
        pane: arranged.pane,
    }
}

struct Arranged<'a> {
    rows: Vec<Row<'a>>,
    escaped: usize,
    folded: usize,
    geometry: DiffGeometry,
    pane: Option<DiffSide>,
    old_digits: usize,
    new_digits: usize,
}

fn arrange(data: DiffData<'_>, width: usize) -> Arranged<'_> {
    let (rows, escaped, folded) = project(data);
    let (old_digits, new_digits) = digits(data.changes);
    let split = data.geometry == DiffGeometry::Split
        && width.saturating_sub(3) / 2 >= old_digits + 4
        && width.saturating_sub(3).div_ceil(2) >= new_digits + 4;
    let geometry = match data.geometry {
        DiffGeometry::Stat => DiffGeometry::Stat,
        _ if split => DiffGeometry::Split,
        _ => DiffGeometry::Unified,
    };
    let pane = if data.geometry == DiffGeometry::Unified {
        data.pane
    } else {
        None
    };
    let rows = if split {
        pair(rows)
    } else {
        side(annotate(rows), pane)
    };
    Arranged {
        rows,
        escaped,
        folded,
        geometry,
        pane,
        old_digits,
        new_digits,
    }
}

fn project_diff(data: DiffData<'_>, width: usize, height: usize) -> DiffProjection {
    let Arranged {
        rows,
        escaped,
        folded,
        geometry,
        pane,
        old_digits,
        new_digits,
    } = arrange(data, width);
    let split = geometry == DiffGeometry::Split;
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
    let numbered = match pane {
        _ if split || data.geometry == DiffGeometry::Stat => true,
        Some(DiffSide::Old) => width > old_digits + 2,
        Some(DiffSide::New) => width > new_digits + 2,
        None => width > old_digits + new_digits + 3,
    };
    if !numbered {
        notices.push(notice(WidgetNoticeKind::LayoutFallback {
            requested: "numbered",
            rendered: "compact",
        }));
    }
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
    let mut sources = Vec::new();
    for (index, row) in rows.iter().skip(start).take(shown).enumerate() {
        let mut rendered = render_row(
            row,
            Frame {
                width,
                offset: data.column,
                old_digits,
                new_digits,
                numbered,
                split,
                pane,
            },
        );
        if rendered.before > 0 || rendered.after > 0 {
            notices.push(notice(WidgetNoticeKind::ClippedColumns {
                row: index,
                before: rendered.before,
                after: rendered.after,
            }));
        }
        for span in &mut rendered.sources {
            span.output_row = index;
        }
        sources.extend(rendered.sources);
        lines.push(rendered.line);
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
    DiffProjection {
        output: WidgetOutput { lines, notices },
        sources,
    }
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
        if data.file.is_some_and(|selected| selected != file_index) {
            continue;
        }
        let target = || DiffTarget::File { file: file_index };
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
        rows.push(Row::Text(
            vec![
                Run::new(label, Tone::Label),
                Run::new(format!(" (+{}", file.additions()), Tone::Added),
                Run::new(format!(" -{})", file.removals()), Tone::Removed),
            ],
            target(),
        ));
        for metadata in file.metadata() {
            escaped += replacements(metadata);
            rows.push(text_row(metadata, Tone::Muted, target()));
        }
        if file.is_binary() {
            rows.push(text_row(
                "Binary files differ (no text-line counts)",
                Tone::Muted,
                target(),
            ));
        }
        if file.hunks().is_empty() && !file.is_binary() {
            rows.push(text_row("No text hunks", Tone::Muted, target()));
        }
        if data.geometry == DiffGeometry::Stat {
            rows.push(Row::Bars(
                file_index,
                file.additions(),
                file.removals(),
                maximum,
            ));
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
    rows.push(text_row(
        heading,
        Tone::Hunk,
        DiffTarget::Hunk {
            file: file_index,
            hunk: hunk_index,
        },
    ));
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
            file: file_index,
            hunk: hunk_index,
            index,
        });
    }
    folded += fold_context(data, file_index, hunk_index, &sources, &mut rows);
    (rows, escaped, folded)
}

/// Push a hunk's source rows, folding long context runs the host has not expanded.
/// Returns how many context entries were hidden.
fn fold_context<'a>(
    data: DiffData<'a>,
    file_index: usize,
    hunk_index: usize,
    sources: &[Source<'a>],
    rows: &mut Vec<Row<'a>>,
) -> usize {
    let mut folded = 0;
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
        let run = ContextRun {
            file: file_index,
            hunk: hunk_index,
            line: source.index,
        };
        let expanded = data.expanded.contains(&run);
        if !expanded && count > data.context.saturating_mul(2).saturating_add(1) {
            rows.extend(
                sources[at..at + data.context]
                    .iter()
                    .copied()
                    .map(Row::Source),
            );
            let hidden = count - data.context * 2;
            rows.push(text_row(
                format!("... {hidden} unchanged"),
                Tone::Muted,
                DiffTarget::Fold {
                    run,
                    entries: sources[at + data.context].index
                        ..sources[end - data.context - 1].index + 1,
                },
            ));
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
    folded
}

fn annotate(rows: Vec<Row<'_>>) -> Vec<Row<'_>> {
    let mut output = Vec::new();
    for row in rows {
        let annotation = match &row {
            Row::Source(source) if source.eof => Some((
                source.old.map(|_| source.address()),
                source.new.map(|_| source.address()),
            )),
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
    let old_eof = old.filter(|source| source.eof).map(Source::address);
    let new_eof = new.filter(|source| source.eof).map(Source::address);
    if old_eof.is_some() || new_eof.is_some() {
        rows.push(Row::Annotation(old_eof, new_eof));
    }
}

/// Keep the rows that exist on one side; shared text rows stay on both.
fn side(rows: Vec<Row<'_>>, pane: Option<DiffSide>) -> Vec<Row<'_>> {
    let Some(pane) = pane else {
        return rows;
    };
    rows.into_iter()
        .filter(|row| match (row, pane) {
            (Row::Source(source), DiffSide::Old) => source.old.is_some(),
            (Row::Source(source), DiffSide::New) => source.new.is_some(),
            (Row::Annotation(old, _), DiffSide::Old) => old.is_some(),
            (Row::Annotation(_, new), DiffSide::New) => new.is_some(),
            _ => true,
        })
        .collect()
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
    side: DiffSide,
) -> RenderedRow {
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
    let shown = source.text.chars().count() - before - after;
    let start = gutter.len() + usize::from(before > 0 && body_width > 0);
    let sources = if shown == 0 {
        Vec::new()
    } else {
        vec![DiffSourceSpan {
            output_row: 0,
            output_columns: start..start + shown,
            file: source.file,
            hunk: source.hunk,
            line: source.index,
            source_codepoints: before..before + shown,
            side,
            line_number: match side {
                DiffSide::Old => source.old,
                DiffSide::New => source.new,
            }
            .expect("a projected source belongs to its displayed side"),
        }]
    };
    let mut runs = vec![Run::new(gutter, Tone::Muted), Run::new(body, source.tone)];
    if !suffix.is_empty() {
        runs.push(Run::new(suffix, Tone::Muted));
    }
    RenderedRow {
        line: WidgetLine::new(runs),
        before,
        after,
        sources,
    }
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

/// Per-projection settings every rendered row shares.
#[derive(Clone, Copy)]
struct Frame {
    width: usize,
    offset: usize,
    old_digits: usize,
    new_digits: usize,
    numbered: bool,
    split: bool,
    pane: Option<DiffSide>,
}

fn render_row(row: &Row<'_>, frame: Frame) -> RenderedRow {
    let Frame {
        width,
        offset,
        old_digits,
        new_digits,
        numbered,
        split,
        pane,
    } = frame;
    match row {
        Row::Text(runs, _) => RenderedRow::without_source(styled_window(runs, width)),
        Row::Source(source) if pane.is_some() => {
            let (number, digits, side) = match pane {
                Some(DiffSide::Old) => (source.old, old_digits, DiffSide::Old),
                _ => (source.new, new_digits, DiffSide::New),
            };
            let gutter = if numbered {
                format!("{} {}", address(number, digits), prefix(*source))
            } else {
                prefix(*source).to_string().chars().take(width).collect()
            };
            source_line(*source, width, offset, gutter, side)
        }
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
            source_line(
                *source,
                width,
                offset,
                gutter,
                if source.tone == Tone::Removed {
                    DiffSide::Old
                } else {
                    DiffSide::New
                },
            )
        }
        Row::Pair(old, new) => render_pair(*old, *new, frame),
        Row::Annotation(old, new) => {
            let marker = "\\ No newline at end of file";
            if split {
                let left_width = (width - 3) / 2;
                let right_width = width - 3 - left_width;
                let (left, _, lost_left) =
                    window(if old.is_some() { marker } else { "" }, left_width, 0);
                let (right, _, lost_right) =
                    window(if new.is_some() { marker } else { "" }, right_width, 0);
                RenderedRow::without_source((
                    WidgetLine::new(vec![
                        Run::new(left, Tone::Muted),
                        Run::new(" | ", Tone::Accent),
                        Run::new(right, Tone::Muted),
                    ]),
                    0,
                    lost_left + lost_right,
                ))
            } else {
                RenderedRow::without_source(styled_window(&[Run::new(marker, Tone::Muted)], width))
            }
        }
        Row::Bars(_, added, removed, maximum) => {
            let left_width = width / 2;
            let right_width = width - left_width;
            let (mut left, lost_left) = stat_bar(*added, *maximum, "+", left_width, Tone::Added);
            let (right, lost_right) = stat_bar(*removed, *maximum, "-", right_width, Tone::Removed);
            left.runs.extend(right.runs);
            RenderedRow::without_source((left, 0, lost_left + lost_right))
        }
    }
}

/// Old and new panes side by side, with right-pane spans in full-row columns.
fn render_pair(old: Option<Source<'_>>, new: Option<Source<'_>>, frame: Frame) -> RenderedRow {
    let Frame {
        width,
        offset,
        old_digits,
        new_digits,
        ..
    } = frame;
    let left_width = (width - 3) / 2;
    let right_width = width - 3 - left_width;
    let pane = |source: Option<Source<'_>>, width, digits, old_side| {
        source.map_or_else(
            || {
                RenderedRow::without_source((
                    WidgetLine::new(vec![Run::new(" ".repeat(width), Tone::Plain)]),
                    0,
                    0,
                ))
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
                    if old_side {
                        DiffSide::Old
                    } else {
                        DiffSide::New
                    },
                )
            },
        )
    };
    let mut left = pane(old, left_width, old_digits, true);
    let mut right = pane(new, right_width, new_digits, false);
    for span in &mut right.sources {
        span.output_columns.start += left_width + 3;
        span.output_columns.end += left_width + 3;
    }
    left.line.runs.push(Run::new(" | ", Tone::Accent));
    left.line.runs.extend(right.line.runs);
    left.before += right.before;
    left.after += right.after;
    left.sources.extend(right.sources);
    left
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
