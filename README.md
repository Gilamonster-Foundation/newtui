<img src="docs/logos/newtui-logo_128.png" alt="newtui logo" width="128" />

# newtui

**Shawn's custom TUI widgets.**

Controls, charts, and workspace panels developed for Shawn's TUI harnesses,
being collected into a reusable Rust library.

**[Browse the widget catalog →](docs/WIDGETS.md)**

[![Heat graphs and meters from gila-monitor-tui](docs/widgets/metrics-preview.svg)](docs/WIDGETS.md)

- **Controls:** settings panels, choosers, and configuration editors.
- **Charts:** heat graphs, butterfly meters, heat bars, gauges, and machine cards.
- **Workspace:** split panes, trees, tabs, linked previews, and changeset review.

Bring your data and terminal host. Widgets describe what to show; your harness
handles drawing, input, and effects. The core has no runtime dependencies by
default.

## Status

The component API and state explorer are implemented. Widgets are being drawn
from [gila-monitor-tui](https://github.com/hartsock/gilabot/tree/main/gila-monitor-tui/src/ui)
and the ongoing [Newt TUI refactor](https://github.com/Gilamonster-Foundation/newt-agent).
The catalog tracks what is available here, what lives in a donor harness,
and what is planned. Rendering adapters and demos have not landed yet.

## Use the core

```sh
cargo add newtui --git https://github.com/Gilamonster-Foundation/newtui
```

Implement [`Component`](src/component.rs) to handle keys and return a
[`View`](src/view.rs). Use [`Explorer`](src/explore.rs) and
[`properties`](src/property.rs) to check behavior without opening a terminal.
Exploration covers the supplied keys and states distinguished by your
fingerprint; `report.is_clean()` also requires the search to finish within its
limits.

[Development plan](docs/PLAN.md) · Rust 1.88+ · [Apache-2.0](LICENSE)
