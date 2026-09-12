# Settings panel host adoption

`SettingsPanel` keeps the current edits relative to the seed supplied when it
opened. `changes()` returns changed settings in seed order; `picked_model()`
returns the current model only when it differs from that opening model. Both
methods only read state. Returning a value to its opening value removes it from
these results.

`intent()` has a different lifetime: Enter stores an `Apply` or `OpenBackends`
snapshot and returns `Flow::Close(true)`. Esc clears that snapshot and returns
`Flow::Close(false)`. Neither key resets the pending values or changes the opening
baseline. If a host retains and revisits the same panel, later edits appear in
the pending reads while the accepted snapshot remains unchanged until the next
Enter or Esc. A fresh `SettingsPanel::new(seed)` starts a fresh baseline.

The host decides which edits it accepts and applies. A containing settings shell
may retain an accepted-section flag across visits, or offer a separate backend
link that accepts pending edits. Those host policies can read current changes
after a section returns with Esc; reading pending state does not itself create
an accepted component intent. The explorer ends an individual run at
`Flow::Close`, so these retained-instance host lifetimes do not change its state
fingerprinting contract.

## Numeric values from the host

Opening values are displayed unchanged, including a value above the configured
ceiling. With valid bounds, the first Left or Right from a value above `max`
lands on `max`. For example, a detail-like row seeded with `100002` and bounds
`0..=100000` displays `100002`, then either horizontal key produces `100000`.
Left at or below the floor selects the release token. Right from that token
enters at the floor; ordinary in-range stepping and ceiling clamping remain the
same. Arithmetic retains the full `usize` range without narrowing through `u32`.

## Measured Newt adaptation

The next consumer slice targets `newt-tui/src/settings_panel.rs`. Its local
`DrillIn`, `ModelRow`, `SettingRow`, `Row`, selection, and dial handling duplicate
the crate state machine. Keep the existing host boundaries:

| Newt input or responsibility | Crate adaptation or retained host path |
| --- | --- |
| `Field::ALL`, `Field::current()`, `Field::value_space()` | Build one `SettingsSeed` in field order; keep the vocabulary in Newt. |
| `ValueSpace::Choice` | `Setting::choice` with the existing value/description pairs. |
| `ValueSpace::Number` | `Setting::number` with the existing release/min/max. |
| `ValueSpace::Text` | `Setting::fixed`, preserving the raw value and existing `/settings` text-form hint. |
| `ModelChoice { name, tag }` and active model | `Model::new` with mapped `Choice`s; missing lists, single choices, and an omitted active model already match. |
| Active backend | `Backend::new`; retain the existing chooser effect after `OpenBackends`. |
| Pending setting writes | Resolve `SettingChange.key` through `Field::from_token`; call existing `settings_form::apply_and_record`. |
| Model switch | Return the pending model to the existing validated caller path in `chat.rs`. |
| Plain `q` cancellation | Host maps `Key::Char('q')` to `Key::Esc`; Ctrl-Q remains ignored. |
| Drawing and cursor | Project `View.rows` into existing `config_panel::RowView`; keep `render_panel`, theme, modal chrome, and `ListCursor`. |

Newt's `Shell` retains its per-section applied flag across re-entry. Its final
settings outcome reads current pending values, and its index Backends link does
so without requiring Session Enter. Therefore the adapter must use the pending
read methods for final values while retaining Shell acceptance and Newt's last
accepted backend-door routing. Using only the most recent `intent()` would lose
edits in these existing flows.

The existing projection marks all setting rows editable, even fixed text and a
single-choice Posture. Adopting `Row.adjustable` corrects that dial chrome; pin
this visible difference explicitly in the consumer's render tests. The old
numeric helper narrows through `u32`; the crate intentionally preserves wide
values and brings oversized values into range without that truncation.

`RowView.label` currently requires a static string. The adapter can project
labels from the same ordered `Field::ALL` metadata used to seed the rows,
followed by the existing static model/backend labels. Test row count and order
rather than widening every panel or leaking owned crate labels.

There is no Session text editor or additional dialog intent to migrate: Prompt
remains a fixed value pointing to the existing typed text form. Receipts,
normalization, model validation, backend routing, and the headless settings path
remain host responsibilities.

The measured duplicate region is roughly 300–350 production lines; a seed,
effect, and projection adapter is estimated at 90–130 lines. Measure the actual
net reduction during the consumer change. Adapt the existing settings tests,
adding Shell re-entry/link scenarios and real renderer checks at normal and
short viewport heights. The prerequisite's regressions live in
`tests/settings_panel.rs` and cover both accepted intent variants, current
pending reads after Esc, returning values to the seed, and oversized integers.
