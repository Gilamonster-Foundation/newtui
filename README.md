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

The north star — a Grafana you can drive from a terminal — and how the crate
gets there is [`docs/PLAN.md`](docs/PLAN.md).

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
satisfies for free. The search walks every reachable state (not path) breadth
first, so a failure's key sequence is always the shortest one, and
`report.verdict()` answers `Clean`, `Violated`, or `Incomplete { reason, .. }`
— because *no violations* and *nothing checked* are not the same claim, and a
boolean cannot tell them apart. How the walk stays exhaustive, why a property
answers three ways and not two, and what the search checks about its own
reconstruction: [`docs/testing-model.md`](docs/testing-model.md).

## Properties are data

An acceptance property is a named check over a state or a transition. The
library ships the ones every component of this shape needs — the selection
stays in range, a non-adjustable row never moves under an arrow, Esc always
leaves without applying. Your component adds its own by writing a closure — one
that returns `PropertyOutcome`, saying whether the observation was even its
business — not a test file.

Every one of them has an observation it REFUSES, and a test in `property.rs`
pins the count to the module, so a property cannot be added without one. A
property that cannot fail is the exact thing this crate exists to refuse — and
the set shipped with one, sold in this paragraph, whose body was `|_| Ok(())`.

That is what makes the corpus portable. A property is a claim about
*observable behaviour*, so it outlives the implementation that first satisfied
it — and a reimplementation in another language, another framework, or another
agent's codebase can be held to exactly the same set.

## Display: styled data, no renderer

Widgets return `WidgetOutput`, a rectangle of lines made from text runs with
semantic tones. The host owns the palette: even the optional ratatui adapter
takes a `Tone -> Style` mapping rather than naming colors here. `Row` remains
the interactive component vocabulary; it is not bent into a chart cell.

```rust
let graph = newtui::sparkline(
    &[10.0, 80.0, 45.0],
    100.0,
    8,
    3,
    newtui::SparkDirection::Up,
);
assert!(graph.validate(8, 3).is_ok());
```

The six builders and their degenerate-width behaviour are catalogued in
[`docs/CATALOG.md`](docs/CATALOG.md).

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
| `src/` | the core: keys, views, component and widget seams, properties, the explorer |
| `demos/` | one recorded terminal demo per component or widget, and the tapes that produce them |
| `examples/python/` | driving the components from Python |
| `docs/` | the plan ([`PLAN.md`](docs/PLAN.md)), the testing model, the catalogue ([`CATALOG.md`](docs/CATALOG.md)), the logo |

## License

Apache-2.0.
