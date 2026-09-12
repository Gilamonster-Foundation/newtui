# Diff display

`diff(DiffData::new(&changes), width, height)` renders the parsed change model
into `WidgetOutput`. The builder performs no I/O, diff computation, hashing,
syntax highlighting or patch application. Original source remains in
`ChangeSet`; `to_unified()` and `to_markdown()` retain it independently of the
cell presentation. The widget does not enter a component's `View` or fingerprint.

## Requests and geometry

`DiffData` borrows a `ChangeSet`. Its defaults are unified geometry, zero row
and column offsets, and three context entries retained at each end of a long
context run. Builder methods set `geometry`, `row_offset`, `column_offset`,
`context`, and `expanded` without storing interaction state in the widget.

- `Unified` has old and new address gutters and a separate source prefix.
  Additions advance only the new address, removals only the old, and context
  both. No-newline annotations consume no address and remain attached to their
  source entry. Narrow output drops complete gutters together, with a notice;
  it never presents a truncated line number as a valid address.
- `Split` pairs context and aligns the old/new members of each consecutive edit
  block by position. Missing counterparts are blank gaps. This is display
  alignment, not a claim that two changed lines have equivalent meaning. Hunk
  boundaries and annotations retain their associations. A split needs both
  complete number gutters and at least two source columns per pane; otherwise
  the returned notices identify the unified fallback.
- `Stat` keeps each file's operation, path and exact text-line counts, then
  draws addition/removal bars using the existing `bar` builder. The largest
  addition or removal count in the changeset supplies their common scale.
  Binary and metadata-only changes retain explicit descriptions even at
  `(+0 -0)`. Zero text counts do not mean unchanged binary content.

Folds use the declared alphabet's ASCII `... N unchanged`. `ContextRun` names
the zero-based `(file, hunk, line)` of the first model entry in a context run;
its line index includes no-newline annotations. Expand that address through
`expanded(&runs)`, or set `context(usize::MAX)` to show all context. Annotated
context entries are excluded from folding. Vertical offsets refer to the
projected rows after folding, and clamp to the final available page. Horizontal
offsets refer to source codepoints; gutters remain stationary.

## Source locations for host styling

`diff_with_sources(data, width, height)` returns `DiffProjection`: the same
`WidgetOutput` plus ordered `DiffSourceSpan` entries for visible source cells.
Projection happens once. Every span identifies its output row and absolute
column range, the file/hunk/model-line location, the original payload's Unicode
scalar range, and the old/new side with its actual one-based file line number.
Rows are relative to the returned viewport; the right pane's columns include
the left pane and separator. Unified context uses the new side; split context
has one span on each side.

Only nonempty source fragments have spans. Original spaces and literal `<`/`>`
characters map normally, as do original codepoints displayed as `?`. Gutters,
synthetic clip markers, padding, escaped-count suffixes, folded summaries,
newline annotations, stat bars and footer text do not map. Output and source
ranges have equal lengths, and spans never overlap. A fully clipped or empty
line, zero-cell viewport or footer-only view has no source span.

A host can convert the scalar range to UTF-8 tokenizer offsets and apply its
syntax foreground colors while preserving the diff's semantic backgrounds.
The library does not tokenize or reset syntax state at each displayed line;
multiline highlighting and access to complete old/new files remain host work.
The metadata contains no source text and does not enter a component `View`.

## Diagnostics are part of the result

`WidgetOutput` now has a public `notices` field. `WidgetOutput::new(lines)`
continues to construct an empty notice list, leaving other widgets' existing
cells unchanged. Consumers using struct literals must initialize `notices`.
`Tone` adds `Added`, `Removed`, `Context`, and `Hunk`, and is now non-exhaustive;
external palette matches must include a fallback. Hosts still own all colors.

`WidgetNoticeKind` describes glyph substitutions, rows outside the viewport,
deliberately folded rows, clipped columns, and layout fallbacks. `message()`
formats its exact facts for a host's caption or accessible text. It contains no
original content or filesystem capability.

The glyph alphabet remains space, ASCII graphics, and `░▒█·`. Each unsupported
codepoint becomes `?`; tabs, source CR, CJK, emoji, combining marks and unsupported
box drawing are counted separately. The display wording `N escaped` denotes
this substitution, **not** a reversible escape encoding. Counts use occurrences
in the logical presentation before scrolling, clipping, folding or split-pane
duplication. A shared context line counts once, although both panes display it.
File labels, metadata and hunk headings also contribute. Stat considers its
headers and metadata, since it does not present source lines.

An affected line reserves its `N escaped` suffix, or a compact `!` when the
suffix does not fit. The last requested row is always the summary footer;
messages that cannot fit whole are represented by `!`. Every notice reports
`Full`, `Indicator`, or `Hidden` visibility. At zero width or height, exact
counts remain in `notices` with `Hidden`; the host must project them outside
the unavailable preview. At height one, the summary takes the single row.

Row omission counts exclude deliberate folding. Column clipping counts source
columns omitted before/after the displayed window, summed across split panes;
the row is a zero-based output row. Full-width headers have their own clipping
notices. `<` and `>` mark horizontal omissions where source cells permit.

## Verification boundary

The domain tests cover every geometry at widths 0, 1, 8, 20 and 200, zero and
tiny heights, extreme offsets, arbitrary source glyphs, EOF annotations,
insertion/deletion, gaps, large line numbers, folding, binary and metadata-only
changes. Registered mutations break source numbering and replacement counts;
the catalog's existing blank-preview mutation protects actual host projection.
Source-span tests cover substitutions, original spaces, synthetic markers,
unequal sides, folding, both scroll offsets and EOF annotations. Mutations drop
all locations, corrupt a file address or omit the right pane's column origin.

These checks do not establish real-PTY repainting, correct host journal keys,
or coordination between two live prompts. Interactive review components,
staging, discarding and syntax highlighting remain separate work under #19.
