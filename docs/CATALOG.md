# Component and widget catalogue

This is the inventory of what the crate ships. Each entry names the host data
it needs, its input domain, its degenerate edge, and the observable properties
that a reimplementation must satisfy.

<!-- component: settings_panel -->
## `settings_panel`

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

A multi-row history graph over a caller-declared maximum. Empty and non-finite
samples render as empty signal; `SparkDirection` chooses the growing edge.

```rust
let graph = newtui::sparkline(&[10.0, 80.0], 100.0, 8, 3, newtui::SparkDirection::Up);
assert!(graph.validate(8, 3).is_ok());
```

<!-- widget: butterfly -->
## `butterfly`

Two current values grow away from a stable centre marker. The host supplies
both labels and the shared maximum.

```rust
let net = newtui::butterfly(20.0, 60.0, 100.0, "TX", "RX", 16, 1);
assert!(net.validate(16, 1).is_ok());
```

<!-- widget: heat_meter -->
## `heat_meter`

A current percentage with optional labels around a positional heat ramp.

```rust
let disk = newtui::heat_meter("disk", 72.0, "72%", 16, 1);
assert!(disk.validate(16, 1).is_ok());
```

<!-- widget: gauge -->
## `gauge`

A current value against a maximum. Wide output shows the caption; narrow
output preserves the gauge signal and clips it to the rectangle.

```rust
let daily = newtui::gauge("daily", 7.5, 10.0, 20, 1);
assert!(daily.validate(20, 1).is_ok());
```

<!-- widget: bar -->
## `bar`

A host-formatted value label beside a bar. Units remain host vocabulary.

```rust
let cpu = newtui::bar("cpu", 45.0, 100.0, "45%", 16, 1);
assert!(cpu.validate(16, 1).is_ok());
```

<!-- widget: core_grid -->
## `core_grid`

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
