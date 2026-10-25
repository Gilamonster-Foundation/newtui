# demos

One recorded terminal demo per component, widget, or layout primitive, and the tapes that produce them.

## Why tapes and not screen recordings

A GIF made by pointing a recorder at someone's terminal is a screenshot of one
session: it drifts from the code silently, and nobody can tell whether it still
shows what the component does. A **tape** is a script — the keys, the timing,
the terminal size — so the GIF is a build artifact regenerated from the current
code, and a component whose demo no longer matches is a diff, not a vibe.

That is the same reason the acceptance corpus is data rather than prose.

```
just demos          # regenerate every GIF and APNG from its tape
just demo settings  # both formats for just one
```

Recorded with [VHS](https://github.com/charmbracelet/vhs).

Both commands build the named host once and pass Cargo's reported executable
to VHS, including with `CARGO_TARGET_DIR` or paths containing spaces. Set
`VHS_BIN=/path/to/vhs` to choose a recorder. They record into a fresh temporary
directory, require multiple frames, derive each APNG from its GIF, and decode
all outputs before replacing any existing captures. A missing frame set,
failed conversion or unreadable output leaves the previous set intact.
Per-piece inputs are retained under [`captures/`](captures/); regenerating one
demo leaves the other demos and their input records untouched.

## Live catalog

`just catalog` opens all shipped pieces in one host. `just catalog-capture`
builds that host and records the checked-in `catalog/*.tape` sequences. The
capture script honors `CARGO_TARGET_DIR`; VHS receives Cargo's actual executable
path. The recording commands need `vhs`, `ttyd`, `ffmpeg`, `ffprobe` and a Chromium-compatible browser (VHS
locates or downloads the browser).

The current captures use VHS 0.11.0. Set `VHS_BIN=/path/to/vhs` to select a
recorder explicitly. VHS 0.12.0 can exit successfully without producing output;
the capture script checks fresh staged files and preserves previous captures
if recording fails. The tapes wait for the live catalog before taking a frame.

The catalog recordings show ordinary chart data, the one-line meter family,
a narrow viewport, an unavailable backend and a light palette. Still PNGs live in
`docs/widgets/generated/`; `catalog/catalog.gif` and `catalog/catalog.png`
are the animated walkthrough. Those overview captures supplement the eleven
per-piece behavior demos below. They never substitute for exercising apply,
cancel, overflow and empty input.

The tapes pin the fixture, palette, font, pixel viewport and key sequence, and
disable cursor blinking. Inspect every generated frame before replacing a
documentation image; an empty border is not evidence that a widget rendered.

## What each demo has to show

Not a feature tour — the BEHAVIOUR the acceptance properties pin, so the GIF
and the test are describing the same component:

| Demo | Shows |
|---|---|
| `settings` | ↑↓ through rows, ←→ dialling a value, an unreachable backend explaining itself, a door that does not dial, Esc leaving without applying |
| `sparkline` | a full history scrolling through repeated rises and falls, then a brief width change and pause/single step |
| `butterfly` | a live compact meter above a long mirrored history, with independent TX/RX wings and matching numeric rates |
| `butterfly_history` | newest-at-bottom history whose two sides rise and cool independently around one center; empty and invalid samples |
| `heat_meter` | changing utilization moving along the heat ramp, then label clipping and an exact paused step |
| `gauge` | repeated filling and draining, followed by a narrow bar and pause/single step |
| `bar` | changing latency with current units and bar length agreeing, then a brief narrow view |
| `core_grid` | twelve independently phased busy/cooling cores, full histories and current percentages matching each last sample |
| `diff` | unified, split and stat layouts; folded/expanded context; row windows; narrow fallback; empty and binary changes; long Unicode source with notices outside the preview |
| `bsp` | live numeric widgets inside ratio geometry, then divider/ratio edits, exact shrink/restore, narrow/empty areas and rejected NaN edits |
| `modal` | a modal docked on a 16-row screen growing and shrinking one row at a time from its granted height, zoom filling the screen and a second zoom restoring it, the MIN_ROWS floor, and a request taller than the screen clamped by the host |
| `linked_panes` | host-owned old/new ASCII text in BSP geometry; unequal correspondence, gap fallback and boundary markers; cursor-driven page windows, focus-only Tab, three link modes, empty/rejected inputs and Esc cancellation |

Numeric named demos run a deterministic synthetic stream every 250 ms; their
tapes spend 10–14 seconds showing changing readings before briefly resizing.
Space pauses/resumes, `.` advances one sample and stays paused, and `r` resets
the samples to tick zero. TX and RX have different cycles. The core demo uses
twelve different burst/cooling patterns; each displayed percentage is the
last sample in its own history. A 256-sample window fills wide views.

The catalog opts in with `--animate` or Space; ordinary launches retain their
fixed fixtures. The [activity tape](catalog/activity.tape) records moving
butterfly history and core data, then pauses/reset/steps to fixed screenshot
ticks. `n` switches between data and geometry status in an animated BSP preview.
Every clock and sample generator lives in the executable hosts; the library
builders stay pure. A deadline controls time independently of key frequency.

The diff demo and catalog share the same parsed fixtures and presentation
state. In the catalog, launch `--item diff`; optional `--geometry`,
`--row-offset`, `--column-offset` and `--expanded` arguments select a deterministic
view. Enter focuses its preview: `g` changes layout, `e` expands context, arrows
scroll rows, Shift-left/right scrolls source columns, and `n` cycles complete
notice messages. Plain left/right changes preview width. F1 returns to browsing.
The named `demo diff` uses the same controls, with `f` for fixtures and `q` to exit.
Its tape honors `NEWTUI_DEMO_BIN` when Cargo's target directory is external.

The BSP demo shares geometry and widget samples with `just catalog --item bsp`.
Optional `--ratio 0.8` and `--shrunk` select deterministic initial states. Enter
focuses the catalog preview, Tab selects a divider, up/down changes its ratio,
`s` shrinks/restores, and `x` attempts a rejected NaN edit. Left/right changes
preview width; F1 returns to browsing. The named `demo bsp` uses the same keys,
with `f` for fixtures and `q` to exit. Status compares the last demo edit at the
current viewport size; it never measures pane content or runs reflow timers.

The modal demo shares `ModalPreview` with `just catalog --item modal`. Enter
focuses the catalog preview; Shift-up/down steps the height one row from what
is on screen, and `z` zooms and restores. `+` and `-` step too, because VHS
cannot send a shifted arrow, so the tape uses them. Plain arrows do nothing to the
height, because they belong to the modal's content. The preview is the modal's
screen: its height is what the host grants, and the status reports requested
and granted rows. The named `demo modal` uses the same keys, with `f` for
fixtures and `q` to exit.

The linked-pane demo shares actual `LinkedPanes` navigation and ASCII source
fixtures with `just catalog --item linked_panes --width 88`. Arrow and page
keys move the selected row; Tab changes focus without remapping; `l` cycles
Locked, Proportional and Unlinked modes. An `=` marks a mapped row; an anchor
is a boundary reported outside the source, never a highlighted adjacent row.
`>` and `.` distinguish the active and other stored cursors; `@` gives the
window's first row. The named host uses `f` for fixtures and `r` for reset.
Esc shows cancellation without closing the recorder, and `q` closes it.

A demo that only shows the happy path is advertising, not documentation.
