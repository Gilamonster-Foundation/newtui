# Diff interchange contract

`newtui::diff::from_unified` reads a host-supplied patch once. The resulting
`ChangeSet` exposes files, paths, file kinds, metadata, hunks, and lines.
`to_unified()` formats its canonical interchange; `to_markdown()` contains that
same text in a `diff` fence for a consumer without a terminal.

This slice implements the model and text surfaces from [#19](https://github.com/Gilamonster-Foundation/newtui/issues/19).
It does not compute a diff, read the filesystem, hash content, run Git, apply a
patch, stage a file, or draw cells. It has no runtime dependencies. The widget
and interactive components remain separate work. A host supplies its existing
diff engine's output; the model fields are private so consumers cannot create
line counts that disagree with the hunk body.

## Accepted input

- Ordinary two-way unified file headers (`---`, `+++`) and `@@` hunks, including
  zero counts, zero-hunk files, multiple files, and multiple ordered hunks.
- Optional `diff --git a/... b/...` headers and recognized extended metadata:
  `index`, old/new modes, new/deleted file modes, similarity/dissimilarity index,
  rename-from/to, and copy-from/to. Extended metadata is preserved in order.
- Binary difference markers (`Binary files ... and ... differ`) following a
  file header, including added/deleted binary files. Binary patch payloads are
  not interpreted as text hunks.
- Path names containing spaces and `->`; Git C-style quoted path labels,
  including escaped quotes, standard C escapes, and octal bytes. Quotes and
  escapes are preserved rather than interpreted as filesystem paths. This also
  retains filenames whose escaped bytes would not decode as UTF-8. Traditional
  unified header timestamps after a tab are retained in interchange, but are
  not included in the path accessors.
- Arbitrary UTF-8 source content, including tabs, Unicode, control characters,
  and a context line whose first content character is `+`, `-`, `\\`, or `@`.
  The first diff prefix alone determines the line kind. A renderer is responsible
  for sanitization and its own glyph/width contract.
- `\ No newline at end of file` immediately following a source line. The model
  stores it as `DiffLine::NoNewline`: it does not consume a source-line address
  or inflate addition/removal counts. The marker applies to the preceding line's
  old side, new side, or both. That side cannot supply later source lines.

The parser rejects unsupported formats and unknown metadata instead of dropping
them: combined diffs (`diff --cc`, `diff --combined`, `@@@`), `GIT binary patch`
payloads, mail envelopes, arbitrary preambles, and custom Git path-prefix modes
are outside this slice. `ParseError` reports a typed category and a one-based
input line. Contradictory metadata, malformed quotes, duplicate extended headers,
overflowing ranges, overlapping hunks, mismatched line counts, and impossible
no-newline sequences are also errors. A failed parse returns no partial model.

Operation metadata and path headers must agree: a rename or copy has both
sides, creation/deletion cannot also declare a mode transition, and similarity
and dissimilarity are alternative declarations. An inline index mode is allowed
only when the mode does not change. When an index header names a definitely
absent side, that side must use the all-zero spelling. These are consistency
checks over supplied fields, following [Git's patch format](https://git-scm.com/docs/diff-format#_generating_patch_text_with_p).
The parser does not verify object IDs against file contents, infer existence
from an abbreviated hash, or recompute similarity percentages from partial hunks.

## Canonical form and CRLF

Formatting retains content, path spelling, metadata order, and hunk section text.
It makes three explicit normalizations: control lines use LF, a one-line hunk
range omits `,1`, and every transported patch record ends in LF. An absent final
transport LF is not evidence that the source file lacked a newline; only the
explicit no-newline annotation expresses that fact.

A CRLF first file header declares CRLF transport, so one trailing CR is removed
from each transported line. With LF file headers, a trailing CR on a source
line is source data and survives. For example, LF headers with `+text\r\n`
describe CRLF source; CRLF headers with `+text\r\n` describe LF source carried
over CRLF transport. CRLF source in CRLF transport carries two CR bytes before
LF. Hosts must select one transport convention; a renderer must not normalize
the source by calling `str::lines()` on the patch body.

For canonical accepted text `t`, `from_unified(t)?.to_unified() == t`.
For every parsed model `m`, parsing `m.to_unified()` reconstructs `m` exactly.
The Markdown fence is longer than every backtick run in the canonical patch,
so path or source content cannot terminate it early.

## What this verifies

The focused data-domain tests exercise parse/format/model round trips, paths,
metadata, degenerate ranges, CRLF, no-newline annotations, invalid input, and
Markdown containment. Declared line counts are checked against available input;
they never control allocation. Parsing has no filesystem or terminal dependency.

These tests do not establish whether a host captured the right edit, whether a
Git index operation applies a hunk correctly, or whether a real PTY renders or
repaints it legibly. Those require host integration and real-resource tests.
