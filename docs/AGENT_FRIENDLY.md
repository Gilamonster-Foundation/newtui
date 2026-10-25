# Agent-friendly components

Status: intent. This document records a direction and the decisions it leaves
open. It changes no code and commits the library to no API.

## Intent

newtui is a suite of TUI components and tools built to be embedded in agentic
TUIs. Each piece has two kinds of user: an operator at a terminal, and an agent
that drives the piece or reads its state. "Agent-friendly" means the second user
gets the same facts as the first without parsing a drawn screen.

The suite ships as Rust crates with PyO3 bindings, published to crates.io and
PyPI.

## What already fits

| Piece | Agent-facing property | Reference |
|---|---|---|
| `Component`, `Key`, `Flow` | A state machine over keys with no terminal type in the seam | [`src/component.rs`](../src/component.rs) |
| `View`, `WidgetOutput` | Display as plain data; `WidgetLine::text` flattens a line; notices stay exact | [`src/view.rs`](../src/view.rs), [`src/widget/output.rs`](../src/widget/output.rs) |
| `ChangeSet` | `to_unified()` and `to_markdown()` serve a consumer without a terminal | [diff model](diff-model.md) |
| `Explorer`, `properties` | Behavior is judged headless, and an incomplete search says so | [testing model](testing-model.md) |
| Portable corpus | Holds a Python or Go reimplementation to the same observable behavior | [corpus](corpus.md) |
| `newtui-py` | A Python face over the same components | [plan, package E](PLAN.md) |

## Gaps

1. **No uniform text face.** The diff model has Markdown. Most other pieces have
   cell lines only, so an agent reading a gauge or a settings panel has no
   stated plain-text contract.
2. **Actions are keys, not intents.** `Flow` has two arms, `Stay` and
   `Close(bool)`, and what an acceptance means is left to the host. That is the
   right rule for effects. It leaves an agent with no typed way to ask for, or
   read, a review action such as stage, unstage or discard. The
   [diff widget](diff-widget.md) already assigns those to the host, and #19's
   changeset state is the first place they matter.
3. **No structured face.** Nothing emits a versioned, machine-readable form of a
   view or a set of available actions. The core cannot add one directly, because
   its runtime dependency closure must stay empty.
4. **Nothing is published.** The core is `0.1.0` and unpublished, `newtui-py` sets
   `publish = false`, and consumers use an immutable git revision. CI builds the
   Python face for coverage only.

## Principles

1. **A text face without a terminal.** Every shipped piece renders plain text or
   Markdown from the same data as its cells. The faces are generated once and
   must not disagree, and a test checks that they do not.
2. **Actions are data.** A component reports what was asked for as a typed
   intent, and the host performs the effect. This extends the existing rule that
   components never perform effects. Stage, unstage and discard remain host
   operations.
3. **State is readable.** An agent can read the current view, the selection, the
   available actions and the notices without inferring them from cells. Clipping
   is never silent; notices stay exact, as they are today.
4. **Behavior is judged headless.** A piece counts as agent-friendly when the
   `Explorer` and properties cover it and the corpus can hold a reimplementation
   to it. An incomplete search reports `exhausted: false` and is never called
   clean.
5. **The leaf invariant holds.** Serialization and any other new face live
   outside the core, in non-default workspace members, the way `newtui-py`,
   `newtui-corpus` and `newtui-mermaid` do. `tests/leaf.rs` keeps the core's
   runtime closure empty.
6. **Rust and Python are one surface.** The PyO3 layer exposes the same types,
   intents and faces and carries no logic of its own. The same corpus judges both
   languages.

## Distribution

- **Crates.** Keep crates small. The core `newtui` stays a leaf. The ratatui
  adapter stays a feature. Faces, the corpus, diagrams and the Python module stay
  separate members that a consumer opts into.
- **PyPI.** Build wheels from `newtui-py` with maturin. Open choice: one `abi3`
  wheel per platform, or one wheel per Python version. `requires-python` is
  `>=3.9` today. Platforms to cover are macOS arm64, Linux and Windows.
- **Names.** On 2026-09-30, `newtui` returned 404 on crates.io and on PyPI, and
  `newtui-py` and `newtui-core` returned 404 on PyPI. Reserve the names when the
  first release is cut.
- **Order.** The plan says to use registry releases when available and a pinned
  git revision until then. Publishing is a release-gate change and is separate
  from this document. MSRV stays pinned at 1.88.

## Open questions

1. Where does the structured face live? A new member such as `newtui-agent` with
   serde behind it keeps the core empty. Hand-emitted JSON in the core avoids a
   member but grows the core and duplicates a serializer. This document leans
   toward the member.
2. What is the intent type? One small enum in the core, or a typed intent per
   component plus a shared trait?
3. What is the structured format and its schema id? The corpus already uses
   `newtui.behavior-corpus/v1` with a content-addressed carrier. Reusing that
   convention would give agent output a version and an identity.
4. In Python, do faces return dataclasses, dicts or JSON strings?
5. Do the Rust crate and the Python package share one version number?

## First slices

Ordered, and none implemented here.

1. **Text-face audit.** For each shipped piece, state its text face, add the
   missing ones, and test that the faces agree. Start with `WidgetOutput` and the
   diff display.
2. **Diff review intents.** Define stage, unstage and discard intents alongside
   the changeset state in #19. This is the first component that needs actions.
3. **Agent face member.** Add the structured output once questions 1 and 3 are
   settled.
4. **Corpus coverage.** Extend the corpus to every piece that gains intents.
5. **Publish.** Reserve names, add the wheel build matrix, and cut the first
   release through the release gate.

## Not in this change

No code, no API, and no change to the packages in [the plan](PLAN.md).

---

Model: claude-sonnet-5-5 | Harness: Claude Code | Operator: Shawn Hartsock | Time: 07:58 EDT | Date: 2026-09-30
