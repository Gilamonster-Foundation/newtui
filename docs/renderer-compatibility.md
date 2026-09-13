# Renderer compatibility

The optional `ratatui` feature and the live catalog use **Ratatui 0.30.2**.
The core, its optional renderer, Python bindings and default catalog retain
the **Rust 1.88** minimum. A headless `newtui` consumer still has no runtime
dependencies; neither the renderer nor the catalog is enabled by default.

`ratatui_lines` returns Ratatui 0.30 `Line` values for the host's widgets and
frame. Hosts using this API should select the same Ratatui 0.30 family.
Ratatui 0.29 types are distinct, even when their names match.

```toml
[dependencies]
newtui = { version = "0.1", features = ["ratatui"] }
ratatui = "0.30.2"
```

Until newtui is published, use an immutable Git revision as described in the
[adoption plan](PLAN.md#current-delivery-and-newt-adoption).

## Host migration

Ratatui's default backend uses Crossterm 0.29. The catalog imports its event
types from `ratatui::crossterm`, so its key decoder and backend use the same
version. Hosts with a direct Crossterm dependency should use 0.29 as well.
Enabling an older backend feature does not override a newer enabled version.

Ratatui 0.30 gives `Backend` an associated `Error` type. A host wrapper around
`CrosstermBackend` can declare `type Error = std::io::Error;`; `TestBackend`
instead uses `std::convert::Infallible`. Existing widget builders, styled
lines, frames and cell buffers keep their familiar interfaces. See the
[upstream migration guide](https://ratatui.rs/highlights/v030/) and
[backend version policy](https://ratatui.rs/concepts/backends/).

Hosts using `tui-textarea` 0.7 can adopt the maintained
[ratatui-textarea 0.8](https://github.com/ratatui/ratatui-textarea/releases/tag/ratatui-textarea-v0.8.0)
with a Cargo package alias to preserve their `tui_textarea` imports. That
dependency belongs to the host; newtui does not depend on a text editor.

## Verification

The existing widget conversion test checks host-selected styles in a real
Ratatui buffer and emits those cells through a Crossterm backend writing to
memory. Catalog tests drive its actual event decoder and inspect rendered
cells across fixtures and sizes. The Rust 1.88 CI lane builds the core with
all features and explicitly builds both non-default hosts. The leaf test
also checks the resolved runtime dependency graph to verify that headless
consumers remain renderer-free.
