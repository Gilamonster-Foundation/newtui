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
dials, Enter by returning an apply or open-backends intent, and Esc (or plain
`q`) by closing without an intent. An unreachable backend is represented by
`Model::new(current, None)`: the active model remains visible, is not
adjustable, and explains why it cannot move.

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
