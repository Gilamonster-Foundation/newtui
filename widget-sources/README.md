# Widget implementation sources

The complete implementation modules live here so every catalog source link is
readable in this repository.

| Module | Implementations |
|---|---|
| [metrics.rs](metrics.rs) | Heat graphs, inverted histories, heat meters, bars, core histories, machine/GPU cards, adaptive layouts, and scrollbars |
| [swarm.rs](swarm.rs) | Butterfly rates, activity heat rows, and status histories |
| [budget.rs](budget.rs) | Budget gauges |
| [character.rs](character.rs) | Animated ASCII character frames and activity states |
| [machine_tab.rs](machine_tab.rs) | Process and pod resource tables |
| [settings.rs](settings.rs) | Text fields, checkboxes, option cycling, and scrolling forms |

These modules include their existing application adapters. The reusable,
compiled library API is in [src/widget](../src/widget) and
[src/components](../src/components); the [catalog](../docs/CATALOG.md) identifies
the pieces that are available now. The remaining implementations are being
adapted to host-supplied data and the same library contracts.
