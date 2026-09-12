# Component and widget catalogue

This is the inventory of what the crate ships. Each entry names the host data
it needs, its input domain, its degenerate edge, and the observable properties
that a reimplementation must satisfy.

The [widget catalog](WIDGETS.md) is the front door — the available / donated /
planned split with donor links. This catalogue goes deeper: the acceptance
properties and runnable examples for each shipped component. How both families
are tested is the [testing model](testing-model.md).

## Diff data and text

`newtui::diff` is a data model and interchange surface, available without
default features. It holds file paths, metadata, hunks, and source lines supplied
by a host. It computes no changes and has no review state, terminal, or effects.
The [interchange contract](diff-model.md) describes the supported domain.

```rust
use newtui::diff::{from_unified, FileKind};

let patch = "--- /dev/null\n+++ b/example.py\n@@ -0,0 +1 @@\n+print('hello')\n";
let changes = from_unified(patch).unwrap();
assert_eq!(changes.files()[0].kind(), FileKind::Added);
assert_eq!(changes.files()[0].additions(), 1);
assert_eq!(changes.to_unified(), patch);
assert_eq!(changes.to_markdown(), format!("```diff\n{patch}```\n"));
```

No widget or interactive demo is claimed by this model-only entry; those are
the remaining #19 surfaces.

<!-- component: settings_panel -->
## `settings_panel`

<!-- demo: settings_panel = settings -->
Demo: [tape](../demos/settings.tape) · [GIF](../demos/settings.gif) · [animated PNG](../demos/settings.png)

A pure settings state machine with choice dials, bounded integer dials, a model
dial, and a door into the host's backend chooser.

The host supplies every setting key, label, current value, accepted choice and
numeric bound through `SettingsSeed`; `newtui` supplies no product vocabulary.
After an accepted close, `intent()` describes changed key/value pairs, an
optional model pick, and whether to open the backend chooser. The host owns all
writes, validation, receipts, network calls, and drawing.

It answers Up/Down by clamping selection, Left/Right by clamping adjustable
dials, Enter by returning an apply or open-backends intent, and Esc by closing
without an intent. A host that wants `q` to quit maps it to `Key::Esc` while
decoding input; printable characters carry no policy here. An unreachable
backend is represented by `Model::new(current, None)`: the active model remains
visible, is not adjustable, and explains why it cannot move.

Its acceptance set requires selection in range, Esc to close without applying,
only adjustable rows to move, and every moved dial value to belong to the
host-supplied vocabulary. Numeric exploration should use a small representative
bound whose states are `{release, min, min + 1, max - 1, max}`; production
bounds remain host data.

```rust
use newtui::components::settings_panel::{
    acceptance, Backend, Choice, Model, Setting, SettingsPanel, SettingsSeed,
};
use newtui::{Explorer, Key, Property};

let seed = SettingsSeed::new(
    vec![
        Setting::choice(
            "tenacity",
            "tenacity",
            "auto",
            vec![Choice::new("auto", "inherit"), Choice::new("steady", "persist")],
        ),
        Setting::number("rounds", "round limit", "auto", "auto", 1, 4),
    ],
    Model::new("qwen", Some(vec![Choice::new("qwen", "active")])),
    Backend::new(Some("sol")),
);
let properties = acceptance(&seed);
let refs: Vec<&dyn Property> = properties.iter().map(AsRef::as_ref).collect();
let report = Explorer::new(Key::navigation())
    .explore(|| SettingsPanel::new(seed.clone()), &refs);
assert!(report.is_clean(), "{report}");
```

The same component is available as plain Python objects; the host still owns
the vocabulary, rendering, persistence, and terminal decoding.

```python
import newtui

panel = newtui.settings_panel(
    settings=[
        newtui.Setting.choice(
            "tenacity",
            "tenacity",
            "auto",
            [
                newtui.Choice("auto", "inherit"),
                newtui.Choice("steady", "persist"),
            ],
        )
    ],
    backend="sol",
    models=["qwen", "nemotron"],
)
panel.handle(newtui.Key.RIGHT)
assert panel.view().rows[0].value == "steady"
```

Every widget builder returns renderer-neutral `WidgetOutput`: lines of text
runs carrying semantic tones. Widths are display columns. Labels and values
that do not fit are clipped, including at widths narrower than the label; zero
width or height returns the corresponding empty rectangle.

Labels and caller-formatted values use the same deliberately closed glyph
alphabet as chart data. Any character outside it is replaced with `?`: this is
lossy, but visible in production and still exactly one display column. Hosts
that need the original spelling retain it in their own data; widgets never
silently drop or mismeasure it.

<!-- widget: sparkline -->
## `sparkline`

<!-- demo: sparkline = sparkline -->
Demo: [tape](../demos/sparkline.tape) · [GIF](../demos/sparkline.gif) · [animated PNG](../demos/sparkline.png)

A multi-row history graph over a caller-declared maximum. Empty and non-finite
samples render as empty signal; `SparkDirection` chooses the growing edge.

```rust
let graph = newtui::sparkline(&[10.0, 80.0], 100.0, 8, 3, newtui::SparkDirection::Up);
assert!(graph.validate(8, 3).is_ok());
```

<!-- widget: butterfly -->
## `butterfly`

<!-- demo: butterfly = butterfly -->
Demo: [tape](../demos/butterfly.tape) · [GIF](../demos/butterfly.gif) · [animated PNG](../demos/butterfly.png)

Two current values grow away from a stable centre marker. The host supplies
both labels and the shared maximum.

```rust
let net = newtui::butterfly(20.0, 60.0, 100.0, "TX", "RX", 16, 1);
assert!(net.validate(16, 1).is_ok());
```

<!-- widget: heat_meter -->
## `heat_meter`

<!-- demo: heat_meter = heat_meter -->
Demo: [tape](../demos/heat_meter.tape) · [GIF](../demos/heat_meter.gif) · [animated PNG](../demos/heat_meter.png)

A current percentage with optional labels around a positional heat ramp.

```rust
let disk = newtui::heat_meter("disk", 72.0, "72%", 16, 1);
assert!(disk.validate(16, 1).is_ok());
```

<!-- widget: gauge -->
## `gauge`

<!-- demo: gauge = gauge -->
Demo: [tape](../demos/gauge.tape) · [GIF](../demos/gauge.gif) · [animated PNG](../demos/gauge.png)

A current value against a maximum. Wide output shows the caption; narrow
output preserves the gauge signal and clips it to the rectangle.

```rust
let daily = newtui::gauge("daily", 7.5, 10.0, 20, 1);
assert!(daily.validate(20, 1).is_ok());
```

<!-- widget: bar -->
## `bar`

<!-- demo: bar = bar -->
Demo: [tape](../demos/bar.tape) · [GIF](../demos/bar.gif) · [animated PNG](../demos/bar.png)

A host-formatted value label beside a bar. Units remain host vocabulary.

```rust
let cpu = newtui::bar("cpu", 45.0, 100.0, "45%", 16, 1);
assert!(cpu.validate(16, 1).is_ok());
```

<!-- widget: core_grid -->
## `core_grid`

<!-- demo: core_grid = core_grid -->
Demo: [tape](../demos/core_grid.tape) · [GIF](../demos/core_grid.gif) · [animated PNG](../demos/core_grid.png)

One compact current-value and history row per visible core. Missing cores fill
their rows with blanks, so the result always occupies the requested height.

```rust
let cores = [newtui::CoreSeries {
    label: "0",
    current: 45.0,
    history: &[20.0, 45.0],
    maximum: 100.0,
}];
let grid = newtui::core_grid(&cores, 16, 4);
assert!(grid.validate(16, 4).is_ok());
```

## Python exploration cost

The Rust explorer can judge a component implemented in Python. It holds the
GIL throughout the walk: replay constructs a fresh Python component and every
`handle`, `view`, and optional `fingerprint` call returns to Python, so there is
no sound `allow_threads` boundary to take.

The reproducible benchmark in `newtui-py/benchmarks/gil_walk.py` explores a
real Python mixer with four five-position dials. A release build on CPython
3.12.3, x86-64, an Intel Core i7-11700B measured 2,500 states and 15,000
transitions in a median 0.313794 seconds across five walks: 20.920 microseconds
per transition. That number includes factory replay, Python callbacks, Rust
property checks, and report construction; it is the cost callers actually pay,
not an isolated crossing microbenchmark.

A Python callback exception becomes a structured `report.errors` entry with
the callback name and key path. The affected branch closes, `report.is_clean`
is false, `report.verdict` is `incomplete` with a reason, and no exception or
panic crosses the FFI boundary. Otherwise the verdict preserves the Rust
report's three answers — `clean`, `violated`, and `incomplete` — rather than
asking callers to infer completeness from `is_clean` and `exhausted`.

`report.properties` carries each supplied claim's observation, applicable,
and held counts with a derived `not_applicable`, `held`, or `violated` outcome.
This belongs to the in-memory Python explorer because it says whether the
acceptance claims were exercised at all. The views and a portable corpus do
not cross this face; those require #13's separately governed wire format.
