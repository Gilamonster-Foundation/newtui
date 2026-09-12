<!--
The plan lives HERE, with the code, rather than on the board — the board is a
pointer system, and a plan that is versioned alongside what it plans stays true
by being edited in the same PR as the thing it describes.

Board cards that hand these packages out live in `knowledge/board/newtui/` and
point back at this file.
-->
# newtui — the plan, and how to split it

`newtui` is a public repo at `Gilamonster-Foundation/newtui`. First commit is
in and CI is green on all six jobs. This note is the whole plan so the rest can
be handed out.

## What it is

> **A TUI component is a state machine over keys. Drive it in isolation, and
> say exactly what it must do.**

Two families under one roof:

- **Components** are interactive — a state machine over keys. A settings panel,
  a chooser, a form, a pager.
- **Widgets** are display — a pure function from data to cells. A sparkline, a
  butterfly meter, a heat bar, a gauge.

The public focus is **Shawn's custom TUI widgets**, developed for his TUI
harnesses and collected into a reusable library. The [widget catalog](WIDGETS.md)
is the entry point: controls, compact charts, and workspace panels. Dashboards
are one composition of those pieces. Each piece should work independently,
handle narrow terminals, and ship with examples and behavioral checks.

## Why a separate repo, not a newt-agent crate

Because three agents need it and none of them should inherit newt's release
train. This follows `precedence-ladder` exactly — the line's established
pattern for carving out a concern — including its hardest rule:

**The leaf invariant.** At `--no-default-features` the resolved runtime
dependency closure is EMPTY, asserted by `tests/leaf.rs` reading
`cargo metadata`, with its own CI job so a break names itself. Rendering lives
behind an optional `ratatui` feature. That is what lets newt, wyvern,
gilamonster and a foreign TUI all depend on one component suite — including
headless consumers that drive components and never draw a frame.

**Do not break this.** A component that drags a renderer into every consumer
ends the reason the repo exists.

## What is already there (~1,455 lines, all green)

| Piece | What it is |
|---|---|
| `Key` | the key vocabulary an acceptance corpus is written in — deliberately not `crossterm::KeyCode`, so the corpus survives a terminal-library swap |
| `View` / `Row` | what a component SHOWS, as plain comparable data — no terminal type anywhere |
| `Component` | the seam: `handle(key) -> Flow`, `view()`, `fingerprint()` |
| `Property` / `properties` | named claims over a state or transition, reading only the view |
| `Explorer` | walks every reachable state, checking properties at each state and transition |
| `tests/leaf.rs` | the empty-closure guard |

Three of the 19 tests prove the harness CATCHES defect shapes rather than
merely running: a door that dials (minimal path `Right`), an Esc swallowed
below the first row (`Down Esc`), and an unbounded dial that must report itself
as a sample rather than a proof.

## The design decision, and what it costs

A ten-agent fan-out weighed three harness designs. **All three judges picked
property-based testing with shrinking** over exhaustive state-graph BFS, on CI
cost — roughly 1–3s per component versus 10–15s under coverage instrumentation,
and the state-graph cost grows multiplicatively as components and seeds are
added.

The shipped `Explorer` is the state-graph design. **That is deliberate and it
is not a contradiction**: exhaustive BFS is what a component's OWN test suite
should run once, in its own repo, where 10s is affordable and the counterexample
is minimal by construction. The property-based layer (package D) is what newt's
per-PR gate runs. Same properties, two harnesses, different budgets.

**State this honestly to anyone who asks**: BFS deduplicates on a fingerprint,
so it walks states rather than paths — but a fingerprint that is too coarse
skips real states, and one too fine never terminates. `Report::exhausted` is
false when a search hit a limit, and `is_clean()` refuses to call a capped run
clean. "No violations in the part I looked at" is not "no violations."

### What no harness in this family will catch

Say this out loud in any PR that claims coverage:

- **A panel drawing into a captured pty** (newt-agent#2020's class). Every
  harness driver is the real driver MINUS the terminal, so no fd is involved.
  That needs the real-PTY tier, which is exactly why that tier exists.
- **A write journalled under the wrong key** (newt-agent#2026). The seam sits
  at the component's door; that bug was one level below it.
- **Two live prompts on one screen** (newt-agent#1959). A property ACROSS
  writers, and no design models more than one component at a time.

## The work packages

Dependencies are stated. A–B–C are the critical path; D–I can start in
parallel once A is in.

---

### Package A — move `settings_panel` across (FIRST; blocks C, E)

The pilot extraction. Chosen because it has **no injected writers** — the other
two panels hold `persist`/`remove` closures that do filesystem I/O, and the
pilot should not also be solving that.

- Port `newt-tui/src/settings_panel.rs`'s state machine into `newtui`, behind
  the `Component` seam. The rows become `View`/`Row`; `Flow` is already
  identical.
- The settings VOCABULARY (what `tenacity` accepts) stays in newt — the
  component takes it as data through its constructor. **A component that knew
  newt's fields would not be reusable, and would not be a leaf.**
- newt keeps a thin adapter: build the seed, hand it to the component, apply
  the outcome. Its local `panel::Key` (newt-agent#2034) is **deleted**, not
  translated — that type exists to be replaced by this crate's.
- Ship the acceptance set with it: selection in range, escape always closes
  without applying, only adjustable rows move, plus its own — a dial only ever
  produces a value the vocabulary accepts.

### The numeric dial is part of the vocabulary, and must arrive bounded

`Field::Rounds` is `1..=10_000` (`RELENTLESS_TOOL_ROUND_TARGET`), stepped one
integer per keypress, and every value is a distinct fingerprint. So the real
panel has ~10,000 states on that row alone, and roughly 29 million in the
cross-product with the other seven — an exhaustive walk is not merely slow, it
is not a thing.

Measured, against the real explorer: the true vocabulary hits `max_states` at
50,000 with `exhausted: false`; a bounded rounds space of five values gives
17,280 states, `exhausted: true`, in 232 ms.

So the numeric bound arrives as data like every other vocabulary, and newtui
explores it over `{release, min, min+1, max-1, max}`. **The 10,000-step walk
proves nothing the five boundary values do not** — newt's own
`settings_panel.rs` already reasons this way, probing release/min/max rather
than walking the range.

**Acceptance:** newt's `/settings` behaves identically (its existing 16 tests
pass unchanged against the adapter), and `newtui` explores the component over a
bounded vocabulary with zero violations and `exhausted: true`.

---

### Package B — the widget family and its data seam (blocks F, G)

The display family, extracted for reuse across harnesses. `gila-monitor-tui`
has the right shape already: `build_net_butterfly_line` is a **pure builder**
with `draw_net_butterfly_meter` a thin wrapper. Generalise that split.

Donors, in [gilabot/gila-monitor-tui/src/ui](https://github.com/hartsock/gilabot/tree/main/gila-monitor-tui/src/ui).
The [catalog](WIDGETS.md) links each donor function and distinguishes existing
implementations from planned library variants.

| Widget | Source |
|---|---|
| heat graph / mirrored history | `metrics.rs::draw_graph`, `draw_graph_inverted` |
| butterfly meter | `swarm.rs::build_net_butterfly_line` (already pure, already tested) |
| heat meter | `metrics.rs::draw_heat_meter` |
| gauge | `budget.rs::draw_gauge` |
| bar line / labelled bar | `metrics.rs::draw_bar_line`, `draw_bar_with_label` |
| per-core / named-series history | `metrics.rs::draw_cpu_cores` |
| activity heat row / status history | `swarm.rs::build_heatrow_commits`, `build_heatrow_status` |
| animated character | `character.rs::draw` |
| machine / GPU machine card | `metrics.rs::draw_machine_cell`, the GPU-machine cell |
| adaptive metrics list / scrollbar | `metrics.rs::summary_layout`, `draw_scrollbar` |

A widget is `fn(data, width, height) -> WidgetOutput` — a rectangle of
`WidgetLine`s composed from text `Run`s carrying a semantic `Tone`. It is
**pure, with no `Frame`**. Rendering is the optional `ratatui` adapter, and the
host supplies the `Tone -> Style` map so it keeps ownership of its palette.

**Not `Vec<Row>`.** This said `Row` until #5, and `Row` is
`label / value / note / selected / adjustable` — a settings row. A sparkline has
no label-value pair and cannot be selected, so three of those five fields would
be permanently false, which is the definition of the wrong type. `src/view.rs`'s
own doc states the rule this broke: *"A component that needs a genuinely
different shape should say so rather than bend into this one."* The donor
settles it — `build_net_butterfly_line` returns styled spans, a run of text plus
a colour. The line type is #5's first deliverable, and the colour in it is a
SEMANTIC tone, not an RGB value: `metrics.rs::value_color(ratio) -> Color` is
already a meaning-to-colour map, and every host has its own palette.

A display widget builds a cell grid from data and dimensions — **pure, no
`Frame`**. Preserve cell positions, glyphs, and styles; the metrics graphs use
color to encode sample intensity, while heat meters use a positional gradient.
Plain label/value `Row` data cannot preserve that distinction. Keep the display
grid separate from the interactive `View` and independent of terminal-library
types. Rendering is the optional `ratatui` adapter; the grid API is still to be
implemented.

Extract small builders first: heat graphs, heat meters, labeled bars, the
butterfly meter, and activity/status rows. The host supplies samples, limits,
labels, units, palettes, and thresholds. App state, metric collection,
Prometheus clients, and machine names stay with the host. Preserve the existing
`░▒█` glyph vocabulary as a console-oriented option.

#### Machine cards describe capabilities

The donor's GPU-machine cell is a GPU-equipped machine composition, not a
reusable name.
Generalize it as a machine card with host-supplied GPU capabilities and memory
topology. A `draw_gpu_machine_cell` adapter would describe its purpose more
clearly during extraction.

| Composition | Memory presentation |
|---|---|
| Machine without GPU metrics | System-memory usage and capacity |
| GPU machine with separate memory | System-memory pool plus separate device-memory pools |
| GPU machine with unified memory | One shared pool; optional CPU/GPU attribution when measurements support it |

A DGX-specific card is a preset of the GPU composition, selecting the topology
for that machine. Do not infer shared memory from a hostname or the DGX name.
Represent shared pool identity explicitly so CPU and GPU views cannot count
the same capacity twice. Missing memory metrics remain unavailable rather than
appearing as zero usage. The unified-memory composition is new work; the
existing GPU-machine cell is the extraction source, not evidence it is already
implemented.

**Card acceptance:** the same generic card accepts different names and data
sources; separate and unified memory fixtures produce distinct layouts; a
shared pool appears exactly once; and missing GPU metrics have a defined
presentation. Host-specific CPU, GPU, storage, and network collection remains
outside newtui.

Widgets are tested over DATA DOMAINS the way components are tested over key
sequences: empty series, one point, all-equal, all-zero, a single spike, values
above the declared max, width narrower than the label, height 1, NaN and
infinity. Every one of those is a real terminal-chart bug.

**Acceptance:** each widget renders without panicking across the full domain
set, and `gila-monitor-tui` can adopt at least one from the crate with its
existing tests passing.

---

### Package C — the property-based layer for newt's gate (needs A)

The judges' winner, as the CHEAP tier. Generated key sequences over the same
`Property` set, with shrinking, wired into newt's per-PR run at a budget that
does not move the 5-minute suite. The exhaustive `Explorer` stays as newtui's
own gate.

Graft the two things the judges said it needs or it is the weakest of the
three: the state-graph's self-test (a harness that cannot fail is worse than
none), and an alphabet guard (a key the component ignores must be IN the
alphabet, or the search proves nothing about it).

---

### Package D — demos as tapes (independent)

One VHS tape per component in `demos/`, so a GIF is a **build artifact
regenerated from current code**, not a screenshot that drifts. `just demos`
already exists.

Each demo must show the behaviour the acceptance properties pin — not a feature
tour. **A demo that only shows the happy path is advertising, not
documentation.** The `demos/README.md` states what each one owes.

---

### Package E — the Python face (needs A)

`newtui-py`, PyO3, a NON-default workspace member so a plain `cargo build`
never compiles it — the arrangement `precedence-ladder` uses.

Two uses, and the second is the one that earns the binding:

1. Build a TUI in Python: drive a component, get a view back, render it with
   Textual / Rich / your own writer.
2. **Hold a Python reimplementation to the same corpus.** The properties are
   claims about observable behaviour, so a component written in Python can be
   explored and judged by exactly the set that judges the Rust one. A shared
   corpus is how two implementations of one component stay one component.

`examples/python/README.md` has the sketch.

---

### Package F — the dashboard layer (needs B)

One way to compose the widgets: a panel is a widget bound to a data
source; a dashboard is a layout of panels plus keyboard navigation. The data
source is a trait the host implements — newtui ships mock sources, never a
client for anybody's database.

Do not start this before B. A dashboard over a chart vocabulary that has not
been proven at eight columns wide is a demo, not a product.

#### F.1 — the panel geometry is a BSP of ratios, not a grid of cells

Panels tile a BSP tree. Every split holds an `f32` ratio for its first child,
clamped to a sane band, and rects are DERIVED at render from the area the host
passes in. Sizes are never stored in cells.

This is the one design decision in F that cannot be deferred, because the
alternative was measured and it loses. tmux's model — absolute integer cell
sizes, re-fitted to a new terminal by round-robin +/-1 nudging — is what
`gilamonster-agent/src/layout.rs` ports today. On an 80/20 split of a 200-column
terminal, halving the terminal gives:

```text
  start            [159,  40]   ratios 0.799 / 0.201
  round-robin      [107,   1]   ratios 1.081 / 0.010   <- overflows 100 cells
  ratio            [ 79,  20]   ratios 0.798 / 0.202

  then grow back to 200:
  round-robin      [153,  46]   started at [159, 40]; never returns
  ratio            [159,  40]   exact round-trip
```

Absolute-plus-clamp is lossy: once a panel floors at its minimum the original
proportion is gone, so a shrink/grow cycle silently rewrites the operator's
layout. Ratios round-trip by construction. **A dashboard whose panels drift
every time the terminal changes is not a dashboard the operator can trust.**

The donor is `gilamonster-agent/src/layout.rs` plus `keys.rs` — both already
import neither `newt-*` nor `ratatui` (std and crate only, verified), so they
satisfy the leaf invariant TODAY. The resize model swaps to ratios **during**
the extraction, not after: three consumers are about to depend on this, and
fixing it later means fixing it in three places.

#### F.2 — mouse resize, without newtui knowing what a mouse is

The layout exposes its dividers as data:

```rust
pub struct SplitBorder {
    pub pos: u16,            // divider line: x for a horizontal split, y for vertical
    pub direction: Direction,
    pub ratio: f32,          // the split's current first-child ratio
    pub area: Rect,          // the split node's area
    pub path: Vec<bool>,     // root -> this split (false = first, true = second)
}

fn splits(&self, area: Rect) -> Vec<SplitBorder>;
fn set_ratio_at(&mut self, path: &[bool], ratio: f32) -> bool;
```

The HOST decodes the pointer, hit-tests it against `pos`, and on a drag sends
`set_ratio_at(path, (x - area.x) / area.width)`. The component never sees a
mouse event, newtui never imports a terminal library, and the leaf invariant is
untouched. This is rule 2 — components describe, hosts draw and act — applied
to a pointer instead of a key. (The shape is herdr's; it is the reason herdr's
panes drag cleanly and ours do not.)

A drag is continuous and the `Explorer` walks a finite alphabet, so a ratio
enters the vocabulary **bounded**, exactly as `Field::Rounds` does in package A:
explore `{min, 0.5, max}` and the boundary neighbours. The 10,000-step walk
proved nothing the five boundary values did not, and neither does a
pixel-by-pixel drag.

#### F.3 — smoothness is the host's job, and newtui must not make it impossible

Crush stays fluid under a resize drag by *deferring* the expensive work:
`BeginResize()` marks the view as resizing so draws skip the full-height scan
and reflow only visible items, then a settle timer warms the cache and clears
the suppression. That belongs in the host — newtui owns no terminal and runs no
timer.

What newtui owes the host is the ability to do it: layout recomputation must be
cheap (it is, once sizes are derived rather than nudged), and a resize must be
able to say WHICH panels actually changed, so a host can reflow those and defer
the rest. A layout that forces a full recompute on every pointer event has made
crush's trick unavailable.

**Acceptance:** an 80/20 split survives a shrink-and-restore with identical
ratios; `splits()` returns one boundary per split with a path that
`set_ratio_at` accepts; the explorer covers the bounded ratio vocabulary with
zero violations and `exhausted: true`; and gilamonster-agent's cockpit runs on
the crate with its existing layout tests passing (the package G discipline —
if adoption hurts, the seam is wrong).

**What this does not cover:** panel CONTENT under resize. F proves the geometry;
whether a widget renders honestly at the width the geometry hands it is B's
claim, and whether a real pty repaints correctly inside one is the real-PTY
tier's. Do not let a green F imply either.

---

### Package G — adopt back into gila-monitor-tui (needs B)

The proof that extraction worked: `gila-monitor-tui` deletes its local copy of
a widget and takes the crate's. If that is painful, the seam is wrong and it is
cheaper to learn it now than after three consumers.

---

### Package H — the remaining two panels (needs A)

`backend_panel` and `config_panel`, which is where the injected-writer question
gets answered: a component that must WRITE takes an injected sink and returns
an intent, so the component stays pure and the host performs the effect. That
rule is already how both panels work; the package is making it the crate's
rule.

---

---

### Package I — the mermaid widget (needs B)

A diagram is display, so a mermaid renderer is a **widget**: mermaid source in,
cells out, no interaction. It is the first widget whose data domain is a
LANGUAGE rather than a series, which is why it is worth naming separately.

Do not write a mermaid parser. `merman` (Latias94) is a headless Rust
implementation — no Node, no Chromium — that parses to a typed semantic model,
computes layout, and renders; `merman-ascii` is its terminal renderer, and its
`render_model_report` returns text PLUS display metrics including width, height
and **overflow state**. That last field is precisely what this crate's
eight-columns-wide bar needs: a widget that can say "this did not fit" instead
of silently clipping.

The costs, stated up front:

- It is `0.8.0-alpha.6`. The API will move, and a pre-1.0 dependency in the
  default graph would be a liability.
- `merman-ascii` is ~9.5 MB and ~336K SLoC — roughly two hundred times the
  newtui core. It is not going anywhere near the leaf.

So it lands as a **separate, non-default workspace member** (`newtui-mermaid`),
the arrangement this repo already uses, and the core never learns it exists. A
consumer that wants diagrams opts in; `cargo build` never compiles it.

**Acceptance:** the widget renders the flowchart / sequence / state / class /
ER families without panicking across the widget domain set (width narrower than
the longest label, height 1, empty diagram, a parse error, a diagram far larger
than the viewport), reports overflow rather than clipping silently, and
`tests/leaf.rs` still passes with an empty closure at
`--no-default-features`.

---

### Package J — the linked two-pane (needs F)

Three things on the wish list are one component: markdown **edit/preview**, a
diff's **old/new**, and a **commit preview**. Each is two surfaces over one
subject with a mapping between them — scroll the left, the right follows the
corresponding region; select a line, its counterpart highlights. Build it once
and scrybe, mattatui, and code review all get it.

The mapping is the component; the surfaces are not. It owns: which side has
focus, the scroll offset of the primary side, the selected region, the link
mode (locked / proportional / unlinked), and the correspondence table it was
GIVEN. It does not own the text, does not parse markdown, and does not diff
anything — a host hands it a correspondence (source line ranges to rendered
regions, or old-line to new-line) as data, the way package A hands the settings
panel its vocabulary.

**Acceptance:** with a correspondence in hand, scrolling either side leaves the
other on the corresponding region and never off the end; an unlinked mode moves
one side only; a correspondence with gaps (a block that exists on one side
only) still yields a defined position on both; explored over a bounded document
of a handful of regions with `exhausted: true`.

---

### Package K — the tree (needs A)

A folder explorer and a markdown outline are one component over different node
sources. It is a state machine over keys — expand, collapse, move, select — and
the node source is data the host supplies, lazily: the component asks for a
node's children, it never reads a filesystem.

It owns: the expansion set, the cursor, and the scroll window. Not the nodes.

**Acceptance:** the cursor is always on a visible node (collapsing an ancestor
of the cursor moves the cursor to that ancestor, never strands it off-screen or
onto a hidden node); expand-then-collapse restores the prior visible set;
a node source that returns no children never renders an expandable node;
explored over a small fixed tree with `exhausted: true`.

**The strandable cursor is the defect this package exists to catch** — it is
the tree-shaped version of the Esc swallowed below the first row.

---

### Package L — the tabbed document container (needs F)

Tabs of open documents, the centre of the PyCharm-shaped composition. A state
machine over: which tab is active, tab order, and the dirty flag per tab. The
documents are the host's.

Closing the active tab must land focus somewhere defined, and the rule is the
operator's expectation, not the implementation's convenience — the neighbour,
not "index 0". (gilamonster-agent's cockpit shipped exactly that bug: a
background pane exiting refocused `panes()[0]` and stole the operator's
keyboard.)

**Acceptance:** closing a tab focuses its neighbour; closing the last tab
leaves a defined empty state rather than an out-of-range index; a dirty tab
cannot be closed without the host being told; tab order survives
activation changes; explored with `exhausted: true`.

---

### Package M — the changeset review surface (needs J, K)

#### Diff data and text face (#19, independent of J and K)

`newtui::diff` supplies the shared `ChangeSet` / file / hunk / line model,
`from_unified`, canonical `to_unified`, and fenced `to_markdown`. This is the
first #19 slice: a host can supply a diff and show it in chat without a terminal
or runtime dependencies. The [interchange contract](diff-model.md) names its
accepted metadata, canonicalization, and error domain. Source stays in this
model, outside the component `View` and fingerprint.

The display widget, linked `diff_view`, and `changeset` review component remain
subsequent slices. The host still computes diffs, captures edit preimages and
postimages, supplies identity and retention, and fulfills Git/index/working-tree
intents. Parsing a patch does not grant permission to apply it.

The reason the IDE composition exists: **a human reviewing an agent's changes
before they land.** Everything else in the PyCharm shape is scaffolding around
this.

Three components, one subject:

- **the changeset view** — changed files with a per-file state. A tree
  (package K) over a change source instead of a directory source.
- **the diff view** — old/new as a linked two-pane (package J) over a hunk
  model: hunks with per-line add / remove / context.
- **the stage editor** — the interactive half. Stage, unstage, and discard at
  file, hunk, or line granularity, with per-file tri-state (staged / unstaged /
  partially staged), plus the commit message form.

**It never runs git.** This is package H's rule at full size: the component
takes the changeset as data and returns INTENTS — `Stage(hunk)`,
`Unstage(file)`, `Discard(range)`, `Commit(message)` — and the host performs
them. A component that shells out is not explorable and not reusable, and a
component that can `git checkout --` a file on its own is one bad transition
away from destroying an operator's work.

**Discard is not the inverse of unstage and must not share its key or its
intent.** Unstage is recoverable; discard destroys the working-tree change.
Give it its own intent so a host can confirm it.

**Acceptance:** staging every hunk of a file leaves that file fully staged, and
unstaging the last one returns it to unstaged; a file with both staged and
unstaged hunks reports partial and never a boolean; discard emits an intent
distinct from unstage; no transition emits an intent the host did not ask to be
possible; the component performs no I/O (asserted the way `tests/leaf.rs`
asserts the closure); explored over a small fixed changeset — two files, three
hunks, one of them partially staged — with `exhausted: true`.

**What this does not cover:** whether the host's git plumbing applies the
intent correctly. That is the host's test, and it needs a real repository.

---

## The donor list above is a third of the real one

Packages A and B name two donors because they are the two that are *ready* — a
panel with no injected writers, and a widget family with a pure builder already
factored out. They are not the extent of the duplication. The operator's scope
is every custom TUI on this line: newt-agent, gilabot's `gila-monitor-tui`,
herdr's panes and tabs, shea, gilamonster-agent, and crush.

Counted across `newt-tui/src`, `gila-monitor-tui/src`, `herdr/src` and
`gilamonster-agent/src` — **files that mention the shape**, which is a smell and
not yet a duplication count; each one needs a read before it becomes a package:

| Shape | newt-tui | gila-monitor | herdr | gilamonster-agent |
|---|---|---|---|---|
| theme / palette | 13 | – | 62 | 1 |
| modal / popup | 27 | 1 | 36 | 1 |
| scrollbar | – | 2 | 26 | 1 |
| spinner | 24 | – | 4 | – |
| status bar / line | 9 | 1 | 1 | 1 |
| tab strip | 4 | 2 | 17 | 2 |
| list cursor / clamp | 6 | – | 3 | 1 |
| help overlay / key hints | – | 3 | 5 | – |

Every row is in three or four surfaces. Three of them are worth naming now
because they are what a component IS, not what it draws:

- **The list cursor.** `newt-tui/src/list_cursor.rs` is 213 lines of clamping
  and it is the thing `properties::selection_is_always_in_range` already checks
  from the outside. Extracting it makes the property and the implementation the
  same claim in two places, which is the point.
- **The key table.** Four key tables gave four different answers about control
  (newt-agent#2033, #2034). `Key` exists so there is one; a component that
  brings its own is the defect re-entering.
- **Theme.** The single biggest count, and the one thing a leaf crate must NOT
  own concretely — see the tone rule under Package B. newtui names meanings;
  hosts own colours. If newtui ever ships an RGB value the leaf argument is
  over.

Ordering: nothing here jumps A or B. B settles the tone question and A settles
the vocabulary question, and every shape in that table needs one or both
answered first. Extracting a popup before the seam it draws into exists is how
you get a second seam.

### crush is Go, and that is a real limit

605 `.go` files, zero `.rs`. A Rust crate does not drop into it, and the honest
answer is not a `cdylib` and cgo inside a bubbletea app — it is that crush
consumes the **corpus**, not the code.

That is not a consolation prize; it is the thing the properties were built to
be. README: *"a property is a claim about observable behaviour, so it outlives
the implementation that first satisfied it — and a reimplementation in another
language, another framework, or another agent's codebase can be held to exactly
the same set."* Package E makes the same argument for Python. What both need is
the corpus **exportable as data** — `report.views` and the property set, written
out in a form a non-Rust runner can read — which is one deliverable serving two
consumers, and it should be built once, in E, rather than twice.

So: crush is not a Package G-style adoption. It is a conformance target, and it
does not gate the release.

## Rules for anyone picking one up

1. **The leaf invariant is not negotiable.** If a package seems to need a
   non-optional dependency, that is a design signal, not a paperwork problem.
2. **Components describe; hosts draw and act.** A component that performs an
   effect has stopped being testable in isolation, which is the whole product.
3. **The vocabulary stays in the host.** newtui knows what a Choice IS; it does
   not know what `tenacity` accepts.
4. **A property is a claim about observable behaviour.** If it needs to reach
   inside a component, it is a unit test, and it belongs with the component.
5. **Say what a harness does not cover.** See the list above; repeat it in the
   PR rather than letting a green run imply more than it proved.
6. **Interaction state is explorable; content is not.** `fingerprint()` defaults
   to the view, so a component whose `view()` carried a document would make the
   state space the space of all documents — unbounded, and the explorer would
   report a sample forever. Split it: the COMPONENT owns cursor, scroll,
   selection, expansion, active tab (bounded, comparable, explored); a WIDGET
   owns rendering the content (`fn(data, width, height) -> cells`, tested over
   data domains). A cursor moving through a document is a small state machine
   over a large data domain, and those are two different tests. This is why
   `View` is rows of plain data and must stay that way — widening it to carry
   spans and buffers would put the document back inside the fingerprint.

## Provenance

Every defect named above as a "shape the harness catches" is one this line
actually hit: a settings panel with an unreachable status arm (found by three
independent harness designs before the harness existed, newt-agent#2031), a
dial step that panics on an empty list, an unguarded index safe only by
accident, and four key tables giving four different answers about control
(newt-agent#2033, #2034). The harness is not speculative; it is the shape of
the last month of bugs, written down.
