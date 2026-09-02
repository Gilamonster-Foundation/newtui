<img src="docs/logos/newtui-logo_source.png" alt="newtui logo" width="256" />

# newtui

> **Terminal UI components you can drive in isolation — interactive ones and
> the charts they sit beside.**

Two families, one crate:

- **Components** are interactive: a state machine over keys. A settings panel,
  a chooser, a form, a pager.
- **Widgets** are display: a pure function from data to cells. A sparkline, a
  butterfly meter, a heat bar, a gauge.

Both are tested the same way — not by scripting the path you thought of, but by
walking **everything reachable** and checking what must always be true.

It is the workbench, not the workshop. It decodes no keys, owns no terminal,
renders nothing by default, and performs no effect. Components describe; hosts
draw and act.

## Where this is going

A **Grafana you can drive from a terminal**: dashboards of live panels, keyboard
navigable, over data sources you bring. The pieces that get there are a chart
vocabulary that renders honestly at eight columns wide, an interaction model
that never strands the operator, and a component suite proven against every
state it can reach rather than the three someone demoed.

## Interactive: three declarations, no I/O

```rust
use newtui::{properties, Component, Explorer, Fingerprint, Flow, Key, Row, View};

struct Volume { level: u8 }

impl Component for Volume {
    fn handle(&mut self, key: Key) -> Flow {
        match key {
            Key::Left => { self.level = self.level.saturating_sub(10); Flow::Stay }
            Key::Right => { self.level = (self.level + 10).min(100); Flow::Stay }
            Key::Enter => Flow::Close(true),
            Key::Esc => Flow::Close(false),
            _ => Flow::Stay,
        }
    }

    fn view(&self) -> View {
        View::titled("volume")
            .row(Row::new("level", self.level.to_string()).adjustable().selected())
    }

    fn fingerprint(&self) -> Fingerprint { Fingerprint::of_view(&self.view()) }
}

let report = Explorer::new(Key::navigation())
    .explore(|| Volume { level: 50 }, &[
        &properties::selection_is_always_in_range(),
        &properties::escape_always_closes_without_applying(),
        &properties::only_adjustable_rows_move(),
    ]);

assert!(report.is_clean(), "{report}");
```

This block is a doctest, so it compiles and runs on every `cargo test`. It is
the only worked example the crate publishes — the idiom a consumer copies as
their first acceptance test — and it once shipped both uncompilable and
failing, which is a good argument for not letting an example be prose.

`is_clean()`, not `violations.is_empty()`: the second is one conjunct of
several, and it is the conjunct a capped, alphabet-less or property-less run
satisfies for free. The violations are reachable through `report.verdict()`,
where the completeness half is not optional — see **Three answers, not two**
below.

### Exhaustive, and the counterexample is minimal for free

Drop the `.adjustable()` from that row — so the renderer promises a plain
value and an arrow moves it anyway — and the same three lines say:

```text
11 states, 66 transitions, 22 terminal, 3 properties
1 violations:

only adjustable rows move
  after: Left
  `level` is not adjustable but Left changed it from `50` to `40`
```

Two properties of the search matter more than the word *exhaustive*:

**It walks states, not paths.** Deduplicating on a fingerprint turns
`keys^depth` sequences into the handful of states a component can actually be
in. An eleven-position dial with four keys is 44 transitions, not a
combinatorial explosion.

**Breadth-first means the first path to a violation is the shortest one.** A
failure reports the minimal key sequence that reaches it, by construction —
there is no shrinking step to trust, tune, or wait for.

### Three answers, not two

`report.verdict()` is `Clean`, `Violated`, or `Incomplete { reason, .. }`. The
third is the one that matters: a search that stopped at a limit, that was
handed no property, or that was handed no key comes back `Incomplete` and says
which, because *no violations* and *nothing checked* are not the same claim and
a boolean cannot tell them apart. `violations` is not a public field, so the
weak assertion is not something a consumer can write by accident.

**The corpus is the same walk.** `report.views` is every distinct view the
search judged — including the view a component closed on, which is where
"escape left the draft alone" lives — and `report.exhausted` is what says
whether that is all of them. One walk and one completeness flag, because a
corpus handed to a reimplementation as "the states it must satisfy" would, if
quietly truncated, pass something that does less. There used to be a second
walk with a second flag; the two disagreed, and the one advertised as the
conformance artifact was the strict subset.

## Properties are data

An acceptance property is a named check over a state or a transition. The
library ships the ones every component of this shape needs — the selection
stays in range, a non-adjustable row never moves under an arrow, Esc always
leaves without applying. Your component adds its own by writing a closure, not
a test file.

Every one of them has an observation it REFUSES, and a test in `property.rs`
pins the count to the module, so a property cannot be added without one. A
property that cannot fail is the exact thing this crate exists to refuse — and
the set shipped with one, sold in this paragraph, whose body was `|_| Ok(())`.

That is what makes the corpus portable. A property is a claim about
*observable behaviour*, so it outlives the implementation that first satisfied
it — and a reimplementation in another language, another framework, or another
agent's codebase can be held to exactly the same set.

## Leaf by construction

At `--no-default-features` this crate's resolved dependency closure is
**empty**, and `tests/leaf.rs` asserts it rather than this paragraph claiming
it. Rendering lives behind the optional `ratatui` feature; a headless consumer
drives components and inspects views without compiling a terminal backend.

That is not tidiness. It is what lets one component suite be shared across
several harnesses — including ones with no terminal at all.

## Layout

| Path | What |
|---|---|
| `src/` | the core: keys, views, the component seam, properties, the explorer |
| `demos/` | one recorded terminal demo per component, and the tapes that produce them |
| `examples/python/` | driving the components from Python |
| `docs/` | the logo, and design notes |

## License

Apache-2.0.
