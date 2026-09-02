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

The north star, in Shawn's words: **a TUI version of Grafana.** Dashboards of
live panels, keyboard navigable, over data sources you bring. Getting there
needs a chart vocabulary that renders honestly at eight columns wide, an
interaction model that never strands the operator, and components proven
against every state they can reach rather than the three someone demoed.

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

Dependencies are stated. A–B–C are the critical path; D–H can start in
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

**Acceptance:** newt's `/settings` behaves identically (its existing 16 tests
pass unchanged against the adapter), and `newtui` explores the component
exhaustively with zero violations and `exhausted: true`.

---

### Package B — the widget family and its data seam (blocks F, G)

The second family, and the one the Grafana use case needs. `gila-monitor-tui`
has the right shape already: `build_net_butterfly_line` is a **pure builder**
with `draw_net_butterfly_meter` a thin wrapper. Generalise that split.

Donors, in `gilabot/gila-monitor-tui/src/ui/`:

| Widget | Source |
|---|---|
| sparkline / history graph | `metrics.rs::draw_graph`, `draw_graph_inverted` |
| butterfly meter | `swarm.rs::build_net_butterfly_line` (already pure, already tested) |
| heat meter | `metrics.rs::draw_heat_meter` |
| gauge | `budget.rs::draw_gauge` |
| bar line / labelled bar | `metrics.rs::draw_bar_line`, `draw_bar_with_label` |
| core grid | `metrics.rs::draw_cpu_cores` |

A widget is `fn(data, width, height) -> Vec<Row>` (or a cell grid) — **pure, no
`Frame`**. Rendering is the optional `ratatui` adapter.

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

Where the Grafana use case becomes real: a panel is a widget bound to a data
source; a dashboard is a layout of panels plus keyboard navigation. The data
source is a trait the host implements — newtui ships mock sources, never a
client for anybody's database.

Do not start this before B. A dashboard over a chart vocabulary that has not
been proven at eight columns wide is a demo, not a product.

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

## Provenance

Every defect named above as a "shape the harness catches" is one this line
actually hit: a settings panel with an unreachable status arm (found by three
independent harness designs before the harness existed, newt-agent#2031), a
dial step that panics on an empty list, an unguarded index safe only by
accident, and four key tables giving four different answers about control
(newt-agent#2033, #2034). The harness is not speculative; it is the shape of
the last month of bugs, written down.
