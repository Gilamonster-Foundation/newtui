# Optional Mermaid reports

`newtui-mermaid` is a separate, non-default workspace member. It requires Rust
1.95 and uses exact-pinned `merman = "=0.8.0-alpha.6"` with only the `ascii`
feature. The core remains a dependency-free Rust 1.88 crate. Building the
default core or catalog does not select Mermaid or raise its toolchain floor.

The upstream facade owns parsing, typed models, layout, resources and operation
control. This adapter adds retained source/results and pure viewport projection.
It contains no Mermaid grammar, terminal driver, provider calls or application
state. The backend API is available as `newtui_mermaid::backend` for configuring
caller-owned input limits and cancellation.

```rust,ignore
use newtui_mermaid::{backend, mermaid, MermaidDocument};

let renderer = backend::Renderer::new();
let document = MermaidDocument::render(
    "flowchart LR\nA[Start] --> B[Done]",
    &renderer,
    backend::OperationControl::new(),
    backend::ascii::AsciiResourcePolicy::default(),
);
let output = mermaid(&document, 40, 12);
let scrolled = document.viewport().row_offset(2).column_offset(4).render(20, 6);
```

The executable version of this example is the member's crate documentation.
`MermaidDocument::source()` returns the exact original bytes. `result()` returns
the complete upstream `AsciiOutput` or its typed error, including diagnostic
codes/spans, resource-limit details and cancellation reasons. A valid
header-only diagram is empty; a parse error is never an empty success.

## Reports and viewports

Preparing a document parses and renders once. Resizing or scrolling only
projects the prepared text. The source and complete backend report remain
unchanged, and neither is placed in a component `View` or fingerprint.

The first slice requests Plain output, ASCII structural characters and
unrestricted `Allow`. Automatic structured fallback is deliberately not the
default: the probe's small state diagram expanded to 6,107 rows at width eight.
The complete diagram is retained for scrolling instead.

Upstream reports width and height, but its `overflowed` field describes only a
requested width bound. The viewport checks both axes itself. `OmittedRows`
reports rows above/below the visible body; `ClippedColumns` reports cells left/
right of each displayed body row. If notices exist, the last available row is
a caption, and omitted-row counts include that reserved row. At height one the
caption replaces the body, and all diagram rows are reported as omitted. At
zero width no diagram rows are displayed. The complete report remains the
source of full diagram dimensions even when no body row can fit.

Notices survive at zero size. A complete caption is `Full`, `!` is `Indicator`,
and a notice with no available cell is `Hidden`. Hosts can show every notice's
`message()` outside the preview and retain full typed errors separately.
`WidgetContentState` distinguishes valid empty data, invalid input, unsupported
content, resource refusal, cancellation, and other preparation failures.

ASCII structure does not remove authored Unicode. The adapter preserves each
grapheme's Unicode-profile display width when projecting into the core's closed
alphabet. A wide unsupported grapheme can become several `?` cells; unsupported
zero-width marks can be removed while their declared base remains. The
`GlyphReplacements` count is affected codepoints in the complete backend text
before clipping, not emitted replacement cells or original source occurrences.
Both original source and backend text remain available without substitutions.

## Accepted capability and resource boundaries

Merman reports partial semantic coverage for the Flowchart, Sequence, State,
Class and ER families. The fixture matrix proves examples from those families,
not all Mermaid syntax. Other families can return typed unsupported errors.
Malformed input keeps the backend's diagnostics. No successful drawing is
invented for unsupported or unrecognized syntax.

The caller's `Renderer` supplies input-resource and parsing policy. Its
`AsciiResourcePolicy` passes unchanged to the backend; `OperationControl`
supplies cancellation or a cooperative deadline. The adapter does not create a
second deadline. Tests cover supplied source/output limits and cancellation;
the selection probe also observed a 250 ms deadline stopping a larger layout.
That observation does not establish a universal hard latency bound.

For Newt adoption, reuse its existing `mermaid::INFO`, `BUDGETS`, `measure`,
extension registry and source-preserving presentation. Keep its 16 KiB source,
256-node, 512-edge, depth-32, 256 KiB output and 250 ms result limits, and retain
the SVG flowchart path for graphics. The optional terminal enhancement can
consume this crate without creating a second grammar or budget registry.

## Dependency selection

The probe selected the latest Merman facade because it owns one source-to-output
operation and exposes typed reports. Its crate license is MIT OR Apache-2.0;
the selected transitive closure also contains MPL-2.0 packages. License
information comes from the published package manifests.

`graphs-tui` 0.4.0 is AGPL-3.0-or-later, depends on unicode-width and winnow, and
built on Rust 1.88 in the probe. Its Mermaid auto-detection misread class/ER
fixtures as D2 and silently clipped narrow flowcharts. Merman 0.7.0-alpha.1
declares Rust 1.87 but lacks State rendering and the report API required here.

The selected backend source revision is
`d529f858ea3d337a1bdc8fe12e44e1403ededf2e`; see its
[terminal support documentation](https://github.com/Latias94/merman/blob/d529f858ea3d337a1bdc8fe12e44e1403ededf2e/crates/merman-ascii/README.md).
Core/default-host Rust 1.88 gates and the exact Rust 1.95 adapter gate remain
separate. Selecting an optional host Mermaid feature must explicitly document
and validate the higher floor.
