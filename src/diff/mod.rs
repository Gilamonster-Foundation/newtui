//! A change supplied by a host, with unified-diff and Markdown text faces.
//!
//! This module parses and formats; it does not compute a diff, read a file, or
//! apply a patch. It is independent of components, widgets, and terminals.
//! [`from_unified`] documents the accepted interchange domain. Private model
//! fields keep parsed ranges, line counts, and metadata consistent. Hosts can
//! supply their diff engine's unified output without inheriting that engine here.

mod parse;

use std::fmt::Write as _;
use std::ops::Range;

pub use parse::{from_unified, ParseError, ParseErrorKind};

/// An ordered collection of file changes. Empty input is an empty collection.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ChangeSet {
    files: Vec<FileChange>,
}

impl ChangeSet {
    /// The files in interchange order, including metadata-only changes.
    #[must_use]
    pub fn files(&self) -> &[FileChange] {
        &self.files
    }

    /// Format canonical unified text, retaining source bytes and metadata.
    ///
    /// Control lines use LF; single-line ranges omit `,1`; every patch record
    /// has a transport newline. Source CR bytes and explicit no-newline markers
    /// are retained. Parsing this text reconstructs the same model.
    #[must_use]
    pub fn to_unified(&self) -> String {
        let mut output = String::new();
        for file in &self.files {
            for line in &file.headers {
                writeln!(output, "{line}").expect("formatting a String cannot fail");
            }
            for hunk in &file.hunks {
                writeln!(
                    output,
                    "@@ -{} +{} @@{}",
                    range_text(&hunk.old),
                    range_text(&hunk.new),
                    hunk.section
                )
                .expect("formatting a String cannot fail");
                for line in &hunk.lines {
                    let (prefix, text) = match line {
                        DiffLine::Add(text) => ('+', text.as_str()),
                        DiffLine::Remove(text) => ('-', text.as_str()),
                        DiffLine::Context(text) => (' ', text.as_str()),
                        DiffLine::NoNewline => ('\\', " No newline at end of file"),
                    };
                    writeln!(output, "{prefix}{text}").expect("formatting a String cannot fail");
                }
            }
        }
        output
    }

    /// Enclose the canonical patch in a `diff` fence, with no terminal needed.
    ///
    /// The fence is longer than any backtick run in the data, so code or path
    /// content cannot close it and become Markdown outside the change.
    #[must_use]
    pub fn to_markdown(&self) -> String {
        let patch = self.to_unified();
        let mut longest = 0;
        let mut run = 0;
        for byte in patch.bytes() {
            run = if byte == b'`' { run + 1 } else { 0 };
            longest = longest.max(run);
        }
        let fence = "`".repeat(3.max(longest + 1));
        format!("{fence}diff\n{patch}{fence}\n")
    }
}

fn range_text(range: &Range<u32>) -> String {
    let count = range.end - range.start;
    if count == 1 {
        range.start.to_string()
    } else {
        format!("{},{count}", range.start)
    }
}

/// Path labels as spelled in the interchange, not filesystem capabilities.
///
/// Git C-style quotes and escapes remain intact, including octal byte escapes
/// for non-UTF-8 names. `/dev/null` denotes an absent side in ordinary headers.
/// Header timestamps are retained in the interchange but excluded here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FilePath {
    old: String,
    new: String,
}

impl FilePath {
    /// Old path label, or the rename/copy source for a metadata-only change.
    #[must_use]
    pub fn old_path(&self) -> &str {
        &self.old
    }

    /// New path label, or the rename/copy destination for a metadata-only change.
    #[must_use]
    pub fn new_path(&self) -> &str {
        &self.new
    }
}

/// The file operation. Binary is independent: a binary file can also be added.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum FileKind {
    /// A previously absent file.
    Added,
    /// A removed file.
    Deleted,
    /// Changed content or metadata at an existing path.
    Modified,
    /// Explicit Git rename metadata.
    Renamed,
    /// Explicit Git copy metadata.
    Copied,
}

/// A file's paths, metadata, and hunks, without review or terminal state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileChange {
    path: FilePath,
    kind: FileKind,
    headers: Vec<String>,
    metadata: Vec<String>,
    hunks: Vec<Hunk>,
    binary: bool,
}

impl FileChange {
    /// The two path labels supplied by the host.
    #[must_use]
    pub fn path(&self) -> &FilePath {
        &self.path
    }

    /// The declared file operation.
    #[must_use]
    pub fn kind(&self) -> FileKind {
        self.kind
    }

    /// Git extended headers, in source order, without file/path headers.
    #[must_use]
    pub fn metadata(&self) -> &[String] {
        &self.metadata
    }

    /// The ordered text hunks. Binary and metadata-only changes have none.
    #[must_use]
    pub fn hunks(&self) -> &[Hunk] {
        &self.hunks
    }

    /// Whether the input explicitly contained a binary-difference marker.
    #[must_use]
    pub fn is_binary(&self) -> bool {
        self.binary
    }

    /// Added source lines, excluding no-newline annotations.
    #[must_use]
    pub fn additions(&self) -> usize {
        self.hunks
            .iter()
            .flat_map(|hunk| &hunk.lines)
            .filter(|line| matches!(line, DiffLine::Add(_)))
            .count()
    }

    /// Removed source lines, excluding no-newline annotations.
    #[must_use]
    pub fn removals(&self) -> usize {
        self.hunks
            .iter()
            .flat_map(|hunk| &hunk.lines)
            .filter(|line| matches!(line, DiffLine::Remove(_)))
            .count()
    }
}

/// A hunk with validated old/new ranges and exactly their source-line counts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hunk {
    old: Range<u32>,
    new: Range<u32>,
    section: String,
    lines: Vec<DiffLine>,
}

impl Hunk {
    /// Old line addresses; an insertion may have an empty range, including 0..0.
    #[must_use]
    pub fn old_range(&self) -> Range<u32> {
        self.old.clone()
    }

    /// New line addresses; a deletion may have an empty range, including 0..0.
    #[must_use]
    pub fn new_range(&self) -> Range<u32> {
        self.new.clone()
    }

    /// Text following the closing `@@`, including its leading space if present.
    #[must_use]
    pub fn section(&self) -> &str {
        &self.section
    }

    /// Content and no-final-newline annotations in interchange order.
    #[must_use]
    pub fn lines(&self) -> &[DiffLine] {
        &self.lines
    }
}

/// One source line or its no-final-newline annotation.
///
/// Content excludes the diff prefix and transport LF, but keeps source CR.
/// No sanitization or glyph substitution takes place in the data model.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum DiffLine {
    /// Content present on the new side only.
    Add(String),
    /// Content present on the old side only.
    Remove(String),
    /// Content present on both sides.
    Context(String),
    /// The preceding source line lacks a final newline, on its indicated side(s).
    /// This annotation has no line address and does not increment hunk counts.
    NoNewline,
}
