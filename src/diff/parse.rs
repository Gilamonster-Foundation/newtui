use super::{ChangeSet, DiffLine, FileChange, FileKind, FilePath, Hunk};
use std::fmt;
use std::ops::Range;

/// Why a supplied patch cannot be represented without guessing or losing data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ParseErrorKind {
    /// A file must begin with `---` or a two-way `diff --git` header.
    ExpectedFileHeader,
    /// An old-file header has no matching new-file header.
    MissingNewFileHeader,
    /// A hunk header does not follow the two-way unified grammar.
    InvalidHunkHeader,
    /// A range is malformed, overflows u32, or uses line zero with nonzero count.
    InvalidRange,
    /// Source lines do not match their hunk's declared counts.
    HunkLengthMismatch,
    /// A hunk body has a prefix other than space, plus, or minus.
    InvalidDiffLine,
    /// A no-newline marker has no source line or that side continues afterward.
    InvalidNoNewline,
    /// An extended header or patch format is not in the accepted domain.
    UnsupportedMetadata,
    /// Recognized metadata is malformed, contradictory, or incomplete.
    InvalidMetadata,
    /// A path label is missing, ambiguously separated, or incorrectly quoted.
    InvalidPath,
}

/// A typed parse failure at a one-based input line (including the next EOF line).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    line: usize,
    kind: ParseErrorKind,
}

impl ParseError {
    /// One-based input line where the failure was detected.
    #[must_use]
    pub fn line(&self) -> usize {
        self.line
    }

    /// The failure category, independent of the human-facing wording.
    #[must_use]
    pub fn kind(&self) -> ParseErrorKind {
        self.kind
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "unified diff line {}: {:?}",
            self.line, self.kind
        )
    }
}

impl std::error::Error for ParseError {}

/// Parse ordinary two-way unified text, with optional Git extended headers.
///
/// Accepted metadata: index hashes/mode, old/new/new-file/deleted-file modes,
/// similarity/dissimilarity percentages, rename/copy pairs, and `Binary files
/// … differ` markers. Headers and path spelling are retained. Combined diffs,
/// `GIT binary patch` payloads, mail envelopes, and unknown metadata are errors.
///
/// Canonical output uses LF control lines and omits `,1`. A CRLF first header
/// declares CRLF *transport*; one CR is then removed from every transported line.
/// With LF headers, CR in a content line is source data and is retained. The
/// final transport LF may be absent; source EOF is expressed only by the explicit
/// `\ No newline at end of file` annotation. Input must be UTF-8, while quoted
/// Git path byte escapes need not decode to UTF-8. Work is bounded by input size;
/// a declared hunk length never allocates or loops beyond the available lines.
///
/// # Errors
/// Returns [`ParseError`] for malformed ranges/counts, unsupported metadata,
/// invalid quoting, and impossible no-newline sequences. No partial model escapes.
pub fn from_unified(source: &str) -> Result<ChangeSet, ParseError> {
    let transport_crlf = source
        .split('\n')
        .next()
        .is_some_and(|line| line.ends_with('\r'));
    let lines = source
        .split_terminator('\n')
        .map(|line| {
            if transport_crlf {
                line.strip_suffix('\r').unwrap_or(line)
            } else {
                line
            }
        })
        .collect();
    let mut parser = Parser { lines, index: 0 };
    let mut files = Vec::new();
    while parser.peek().is_some() {
        files.push(parser.file()?);
    }
    Ok(ChangeSet { files })
}

struct Parser<'a> {
    lines: Vec<&'a str>,
    index: usize,
}

impl<'a> Parser<'a> {
    fn peek(&self) -> Option<&'a str> {
        self.lines.get(self.index).copied()
    }

    fn error(&self, kind: ParseErrorKind) -> ParseError {
        ParseError {
            line: self.index + 1,
            kind,
        }
    }

    fn take(&mut self) -> &'a str {
        let line = self.lines[self.index];
        self.index += 1;
        line
    }

    fn file(&mut self) -> Result<FileChange, ParseError> {
        let mut headers = Vec::new();
        let git = self
            .peek()
            .and_then(|line| line.strip_prefix("diff --git "));
        if git.is_some() {
            headers.push(self.take().to_owned());
        } else if self.peek().is_some_and(|line| line.starts_with("diff ")) {
            return Err(self.error(ParseErrorKind::UnsupportedMetadata));
        } else if !self.peek().is_some_and(|line| line.starts_with("--- ")) {
            return Err(self.error(ParseErrorKind::ExpectedFileHeader));
        }
        let mut metadata = Vec::new();
        if git.is_some() {
            while let Some(line) = self.peek() {
                if line.starts_with("--- ")
                    || line.starts_with("diff ")
                    || line.starts_with("Binary files ")
                    || line.starts_with("@@")
                {
                    break;
                }
                validate_metadata(line).map_err(|kind| self.error(kind))?;
                metadata.push(self.take().to_owned());
            }
            headers.extend(metadata.iter().cloned());
        }
        let path = self.file_labels(&mut headers)?;
        let (kind, renamed) = metadata_kind(&metadata).map_err(|kind| self.error(kind))?;
        let kind = resolve_file_kind(kind, path.as_ref()).map_err(|kind| self.error(kind))?;
        validate_metadata_state(&metadata, kind).map_err(|kind| self.error(kind))?;
        let git_paths = git
            .map(|header| git_paths(header, path.as_ref(), renamed.as_ref()))
            .transpose()
            .map_err(|kind| self.error(kind))?;
        let path = path
            .or(renamed)
            .or_else(|| git_paths.clone())
            .ok_or_else(|| self.error(ParseErrorKind::InvalidPath))?;
        let binary = self.binary_marker(&mut headers, git_paths.as_ref().unwrap_or(&path), kind)?;
        let mut hunks: Vec<Hunk> = Vec::new();
        let mut eof = (false, false);
        while self.peek().is_some_and(|line| line.starts_with("@@")) {
            if binary {
                return Err(self.error(ParseErrorKind::InvalidMetadata));
            }
            let hunk = self.hunk(&mut eof)?;
            if (kind == FileKind::Added && !hunk.old.is_empty())
                || (kind == FileKind::Deleted && !hunk.new.is_empty())
            {
                return Err(self.error(ParseErrorKind::InvalidMetadata));
            }
            if hunks
                .last()
                .is_some_and(|last| last.old.end > hunk.old.start || last.new.end > hunk.new.start)
            {
                return Err(self.error(ParseErrorKind::InvalidRange));
            }
            hunks.push(hunk);
        }
        if self
            .peek()
            .is_some_and(|line| !line.starts_with("--- ") && !line.starts_with("diff "))
        {
            return Err(self.error(ParseErrorKind::HunkLengthMismatch));
        }
        Ok(FileChange {
            path,
            kind,
            headers,
            metadata,
            hunks,
            binary,
        })
    }

    fn file_labels(&mut self, headers: &mut Vec<String>) -> Result<Option<FilePath>, ParseError> {
        let Some(old) = self.peek().and_then(|line| line.strip_prefix("--- ")) else {
            return Ok(None);
        };
        let old = path_label(old).map_err(|kind| self.error(kind))?.to_owned();
        headers.push(self.take().to_owned());
        let Some(new) = self.peek().and_then(|line| line.strip_prefix("+++ ")) else {
            return Err(self.error(ParseErrorKind::MissingNewFileHeader));
        };
        let new = path_label(new).map_err(|kind| self.error(kind))?.to_owned();
        if old == "/dev/null" && new == "/dev/null" {
            return Err(self.error(ParseErrorKind::InvalidPath));
        }
        headers.push(self.take().to_owned());
        Ok(Some(FilePath { old, new }))
    }

    fn binary_marker(
        &mut self,
        headers: &mut Vec<String>,
        labels: &FilePath,
        kind: FileKind,
    ) -> Result<bool, ParseError> {
        let Some(line) = self.peek().filter(|line| line.starts_with("Binary files ")) else {
            return Ok(false);
        };
        let old = if kind == FileKind::Added {
            "/dev/null"
        } else {
            &labels.old
        };
        let new = if kind == FileKind::Deleted {
            "/dev/null"
        } else {
            &labels.new
        };
        if line != format!("Binary files {old} and {new} differ") {
            return Err(self.error(ParseErrorKind::InvalidMetadata));
        }
        headers.push(self.take().to_owned());
        Ok(true)
    }

    fn hunk(&mut self, eof: &mut (bool, bool)) -> Result<Hunk, ParseError> {
        let header = self.peek().expect("caller saw a hunk");
        let (old, new, section) = hunk_header(header).map_err(|kind| self.error(kind))?;
        self.take();
        let mut remaining = (old.end - old.start, new.end - new.start);
        let mut lines = Vec::new();
        loop {
            if self
                .peek()
                .is_some_and(|line| line.trim_end_matches('\r') == "\\ No newline at end of file")
            {
                let sides = match lines.last() {
                    Some(DiffLine::Add(_)) => (false, true),
                    Some(DiffLine::Remove(_)) => (true, false),
                    Some(DiffLine::Context(_)) => (true, true),
                    _ => return Err(self.error(ParseErrorKind::InvalidNoNewline)),
                };
                eof.0 |= sides.0;
                eof.1 |= sides.1;
                lines.push(DiffLine::NoNewline);
                self.take();
                continue;
            }
            if remaining == (0, 0) {
                break;
            }
            let line = self
                .peek()
                .ok_or_else(|| self.error(ParseErrorKind::HunkLengthMismatch))?;
            let (old_line, new_line, parsed) = if let Some(text) = line.strip_prefix('+') {
                (false, true, DiffLine::Add(text.to_owned()))
            } else if let Some(text) = line.strip_prefix('-') {
                (true, false, DiffLine::Remove(text.to_owned()))
            } else if let Some(text) = line.strip_prefix(' ') {
                (true, true, DiffLine::Context(text.to_owned()))
            } else {
                return Err(self.error(ParseErrorKind::InvalidDiffLine));
            };
            if (old_line && eof.0) || (new_line && eof.1) {
                return Err(self.error(ParseErrorKind::InvalidNoNewline));
            }
            if (old_line && remaining.0 == 0) || (new_line && remaining.1 == 0) {
                return Err(self.error(ParseErrorKind::HunkLengthMismatch));
            }
            remaining.0 -= u32::from(old_line);
            remaining.1 -= u32::from(new_line);
            lines.push(parsed);
            self.take();
        }
        Ok(Hunk {
            old,
            new,
            section: section.to_owned(),
            lines,
        })
    }
}

fn hunk_header(header: &str) -> Result<(Range<u32>, Range<u32>, &str), ParseErrorKind> {
    let (ranges, section) = header
        .strip_prefix("@@ -")
        .and_then(|rest| rest.split_once(" @@"))
        .ok_or(ParseErrorKind::InvalidHunkHeader)?;
    if !section.is_empty() && !section.starts_with(' ') {
        return Err(ParseErrorKind::InvalidHunkHeader);
    }
    let (old, new) = ranges
        .split_once(" +")
        .ok_or(ParseErrorKind::InvalidHunkHeader)?;
    Ok((parse_range(old)?, parse_range(new)?, section))
}

fn parse_range(value: &str) -> Result<Range<u32>, ParseErrorKind> {
    let (start, count) = value.split_once(',').unwrap_or((value, "1"));
    let decimal = |text: &str| {
        if text.is_empty() || !text.bytes().all(|byte| byte.is_ascii_digit()) {
            return Err(ParseErrorKind::InvalidRange);
        }
        text.parse::<u32>()
            .map_err(|_| ParseErrorKind::InvalidRange)
    };
    let start = decimal(start)?;
    let count = decimal(count)?;
    if start == 0 && count != 0 {
        return Err(ParseErrorKind::InvalidRange);
    }
    let end = start
        .checked_add(count)
        .ok_or(ParseErrorKind::InvalidRange)?;
    Ok(start..end)
}

fn validate_metadata(line: &str) -> Result<(), ParseErrorKind> {
    for prefix in [
        "old mode ",
        "new mode ",
        "new file mode ",
        "deleted file mode ",
    ] {
        if let Some(mode) = line.strip_prefix(prefix) {
            return if valid_mode(mode) {
                Ok(())
            } else {
                Err(ParseErrorKind::InvalidMetadata)
            };
        }
    }
    for prefix in ["rename from ", "rename to ", "copy from ", "copy to "] {
        if let Some(path) = line.strip_prefix(prefix) {
            return validate_path(path);
        }
    }
    for prefix in ["similarity index ", "dissimilarity index "] {
        if let Some(percent) = line.strip_prefix(prefix) {
            return if percent
                .strip_suffix('%')
                .and_then(|text| text.parse::<u8>().ok())
                .is_some_and(|value| value <= 100)
            {
                Ok(())
            } else {
                Err(ParseErrorKind::InvalidMetadata)
            };
        }
    }
    if let Some(index) = line.strip_prefix("index ") {
        let (hashes, mode) = index
            .split_once(' ')
            .map_or((index, None), |(hashes, mode)| (hashes, Some(mode)));
        let valid = hashes.split_once("..").is_some_and(|(old, new)| {
            [old, new]
                .iter()
                .all(|hash| !hash.is_empty() && hash.bytes().all(|byte| byte.is_ascii_hexdigit()))
        });
        return if valid && mode.is_none_or(valid_mode) {
            Ok(())
        } else {
            Err(ParseErrorKind::InvalidMetadata)
        };
    }
    Err(ParseErrorKind::UnsupportedMetadata)
}

fn valid_mode(mode: &str) -> bool {
    mode.len() == 6 && mode.bytes().all(|byte| (b'0'..=b'7').contains(&byte))
}

fn metadata_kind(metadata: &[String]) -> Result<(FileKind, Option<FilePath>), ParseErrorKind> {
    for prefix in [
        "old mode ",
        "new mode ",
        "new file mode ",
        "deleted file mode ",
        "rename from ",
        "rename to ",
        "copy from ",
        "copy to ",
        "index ",
        "similarity index ",
        "dissimilarity index ",
    ] {
        if metadata
            .iter()
            .filter(|line| line.starts_with(prefix))
            .count()
            > 1
        {
            return Err(ParseErrorKind::InvalidMetadata);
        }
    }
    if metadata.iter().any(|line| line.starts_with("old mode "))
        != metadata.iter().any(|line| line.starts_with("new mode "))
    {
        return Err(ParseErrorKind::InvalidMetadata);
    }
    let mut kind = FileKind::Modified;
    let mut pair = None;
    let mut operation = None;
    for (prefix, candidate) in [
        ("new file mode ", FileKind::Added),
        ("deleted file mode ", FileKind::Deleted),
        ("rename from ", FileKind::Renamed),
        ("copy from ", FileKind::Copied),
    ] {
        if let Some(value) = metadata.iter().find_map(|line| line.strip_prefix(prefix)) {
            if operation.is_some() {
                return Err(ParseErrorKind::InvalidMetadata);
            }
            operation = Some(candidate);
            kind = candidate;
            if matches!(candidate, FileKind::Renamed | FileKind::Copied) {
                let target = if candidate == FileKind::Renamed {
                    "rename to "
                } else {
                    "copy to "
                };
                let new = metadata
                    .iter()
                    .find_map(|line| line.strip_prefix(target))
                    .ok_or(ParseErrorKind::InvalidMetadata)?;
                pair = Some(FilePath {
                    old: value.to_owned(),
                    new: new.to_owned(),
                });
            }
        }
    }
    for (from, to) in [("rename from ", "rename to "), ("copy from ", "copy to ")] {
        if metadata.iter().any(|line| line.starts_with(from))
            != metadata.iter().any(|line| line.starts_with(to))
        {
            return Err(ParseErrorKind::InvalidMetadata);
        }
    }
    Ok((kind, pair))
}

/// A path header can infer an operation, but cannot replace one explicitly
/// declared by Git metadata. Resolve once before inspecting ranges or markers.
fn resolve_file_kind(kind: FileKind, path: Option<&FilePath>) -> Result<FileKind, ParseErrorKind> {
    let Some(path) = path else {
        return Ok(kind);
    };
    let inferred = if path.old == "/dev/null" {
        Some(FileKind::Added)
    } else if path.new == "/dev/null" {
        Some(FileKind::Deleted)
    } else {
        None
    };
    match kind {
        FileKind::Modified => Ok(inferred.unwrap_or(kind)),
        FileKind::Added | FileKind::Deleted if inferred == Some(kind) => Ok(kind),
        FileKind::Renamed | FileKind::Copied if inferred.is_none() => Ok(kind),
        _ => Err(ParseErrorKind::InvalidMetadata),
    }
}

/// Check mutually exclusive declarations and the index spelling of an absent
/// side. The viewer never verifies object IDs or recomputes similarity scores.
fn validate_metadata_state(metadata: &[String], kind: FileKind) -> Result<(), ParseErrorKind> {
    let value = |prefix| metadata.iter().find_map(|line| line.strip_prefix(prefix));
    let mode_transition = value("old mode ").is_some();
    let similarity = value("similarity index ").is_some();
    let dissimilarity = value("dissimilarity index ").is_some();
    let absent_side = matches!(kind, FileKind::Added | FileKind::Deleted);
    if (similarity && dissimilarity)
        || (absent_side && (mode_transition || similarity || dissimilarity))
    {
        return Err(ParseErrorKind::InvalidMetadata);
    }
    if let Some(index) = value("index ") {
        let (hashes, mode) = index
            .split_once(' ')
            .map_or((index, None), |(hashes, mode)| (hashes, Some(mode)));
        let (old, new) = hashes
            .split_once("..")
            .ok_or(ParseErrorKind::InvalidMetadata)?;
        if (mode.is_some() && (mode_transition || absent_side))
            || (kind == FileKind::Added && !old.bytes().all(|byte| byte == b'0'))
            || (kind == FileKind::Deleted && !new.bytes().all(|byte| byte == b'0'))
        {
            return Err(ParseErrorKind::InvalidMetadata);
        }
    }
    Ok(())
}

fn path_label(label: &str) -> Result<&str, ParseErrorKind> {
    let path = label.split_once('\t').map_or(label, |(path, _)| path);
    validate_path(path)?;
    Ok(path)
}

fn validate_path(path: &str) -> Result<(), ParseErrorKind> {
    if path.is_empty() || path.chars().any(char::is_control) {
        return Err(ParseErrorKind::InvalidPath);
    }
    if path.starts_with('"') {
        if quoted_end(path) != Some(path.len()) {
            return Err(ParseErrorKind::InvalidPath);
        }
    } else if path.contains('"') {
        return Err(ParseErrorKind::InvalidPath);
    }
    Ok(())
}

fn quoted_end(path: &str) -> Option<usize> {
    let bytes = path.as_bytes();
    let mut index = 1;
    while index < bytes.len() {
        match bytes[index] {
            b'"' => return (index > 1).then_some(index + 1),
            b'\\' => {
                index += 1;
                match *bytes.get(index)? {
                    b'a' | b'b' | b't' | b'n' | b'v' | b'f' | b'r' | b'\\' | b'"' => index += 1,
                    b'0'..=b'7' => {
                        let mut value = 0_u16;
                        for _ in 0..3 {
                            let Some(byte @ b'0'..=b'7') = bytes.get(index) else {
                                break;
                            };
                            value = value * 8 + u16::from(byte - b'0');
                            index += 1;
                        }
                        if value > 255 {
                            return None;
                        }
                    }
                    _ => return None,
                }
            }
            _ => index += 1,
        }
    }
    None
}

fn git_paths(
    header: &str,
    labels: Option<&FilePath>,
    renamed: Option<&FilePath>,
) -> Result<FilePath, ParseErrorKind> {
    // Supplied file/rename labels determine the separator even when a filename
    // contains spaces or " b/". A metadata-only modification has equal names,
    // hence equal byte lengths on each side. No quadratic split-and-guess scan.
    let expected = if let Some(paths) = renamed {
        FilePath {
            old: git_prefix(&paths.old, 'a'),
            new: git_prefix(&paths.new, 'b'),
        }
    } else if let Some(labels) = labels {
        let old = if labels.old == "/dev/null" {
            git_prefix(&without_git_prefix(&labels.new)?, 'a')
        } else {
            labels.old.clone()
        };
        let new = if labels.new == "/dev/null" {
            git_prefix(&without_git_prefix(&labels.old)?, 'b')
        } else {
            labels.new.clone()
        };
        FilePath { old, new }
    } else {
        let midpoint = header.len() / 2;
        if header.as_bytes().get(midpoint) != Some(&b' ') {
            return Err(ParseErrorKind::InvalidPath);
        }
        let old = header.get(..midpoint).ok_or(ParseErrorKind::InvalidPath)?;
        let new = header
            .get(midpoint + 1..)
            .ok_or(ParseErrorKind::InvalidPath)?;
        if without_git_prefix(old)? != without_git_prefix(new)? {
            return Err(ParseErrorKind::InvalidPath);
        }
        FilePath {
            old: old.to_owned(),
            new: new.to_owned(),
        }
    };
    validate_path(&expected.old)?;
    validate_path(&expected.new)?;
    if !(expected.old.starts_with("a/") || expected.old.starts_with("\"a/"))
        || !(expected.new.starts_with("b/") || expected.new.starts_with("\"b/"))
    {
        return Err(ParseErrorKind::InvalidPath);
    }
    if header != format!("{} {}", expected.old, expected.new) {
        return Err(ParseErrorKind::InvalidPath);
    }
    if let Some(labels) = labels {
        if (labels.old != "/dev/null" && labels.old != expected.old)
            || (labels.new != "/dev/null" && labels.new != expected.new)
        {
            return Err(ParseErrorKind::InvalidPath);
        }
    }
    Ok(expected)
}

fn without_git_prefix(path: &str) -> Result<String, ParseErrorKind> {
    if path.starts_with("\"a/") || path.starts_with("\"b/") {
        Ok(format!("\"{}", &path[3..]))
    } else if path.starts_with("a/") || path.starts_with("b/") {
        Ok(path[2..].to_owned())
    } else {
        Err(ParseErrorKind::InvalidPath)
    }
}

fn git_prefix(path: &str, prefix: char) -> String {
    if let Some(quoted) = path.strip_prefix('"') {
        format!("\"{prefix}/{quoted}")
    } else {
        format!("{prefix}/{path}")
    }
}
