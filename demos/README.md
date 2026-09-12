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
are the animated walkthrough. Those overview captures supplement the nine
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
| `sparkline` | a series at several widths, including narrower than its label |
| `butterfly` | two directions against a stable midline, at the width where the midline is all that fits |
| `heat_meter` | the positional heat ramp retaining a signal while its labels are clipped at narrow widths |
| `gauge` | the wide caption switching to an honest bar when the caption no longer fits |
| `bar` | a host-formatted value and units yielding space to the bar as width contracts |
| `core_grid` | two histories retaining their row order while missing core rows stay visibly empty |
| `diff` | unified, split and stat layouts; folded/expanded context; row windows; narrow fallback; empty and binary changes; long Unicode source with notices outside the preview |
| `bsp` | real widget panes in ratio geometry; selected dividers; ratio edits; exact shrink/restore; narrow and empty areas; rejected NaN edits with geometry status outside the preview |

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

A demo that only shows the happy path is advertising, not documentation.
