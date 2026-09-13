# Linked pane navigation

`components::linked_panes::LinkedPanes` supplies package J's reusable keyboard
navigation over two host-owned surfaces. Source/preview and old/new text can
use the same state. The component contains counts and correspondence ranges,
cursor addresses, window offsets and capacities, focus, and link evidence.
It owns no source text, terminal, parser, file, timer, or external action.

The host uses BSP geometry to place its surfaces and uses `position(side)` to
choose their visible rows. `view()` exposes numeric interaction state for the
acceptance harness; it is not the document display. Its default fingerprint
retains both cursors, offsets, capacities, focus, policy, and the full immutable
numeric mapping, including the evidence from the last directional selection.

## Correspondence and missing regions

`Correspondence::new([first_count, second_count], regions)` accepts ordered,
monotone, nonoverlapping half-open ranges. An empty range is a boundary on one
surface; a region empty on both surfaces is rejected. Reversed, out-of-range,
overlapping, and crossing ranges return an indexed `CorrespondenceError`.
The constructor never sorts or renumbers the host's regions. Moved-block
correspondences that cross are outside this first monotone domain.

In locked mode, a selected row inside a region maps proportionally between that
region's actual endpoints. Outside the regions, the nearest actual source
endpoint is used; an equal-distance gap chooses the previous host region. A
range end is a boundary, so its last actual row is `end - 1`. With no nonempty
source region, whole-surface proportional mapping is an explicit
`MatchKind::NoCorrespondence` fallback. Uncovered rows never become an invented
exact region match.

`Mapping.target` distinguishes `Row(n)` from `Boundary(n)`. In particular, an
empty destination range at EOF remains a boundary even if its nonempty pane's
viewport cursor is clamped to the last row. Hosts highlight the boundary, not
that unrelated last row. A completely empty pane has `cursor: None` and offset
zero. `LinkRelation.source` records which side supplied the last mapping;
switching focus does not reinterpret or reverse it.

## Keys, selection, and windows

- Up/Down move one row. PageUp/PageDown move the active window capacity.
  Home/End select the first/last row. Arithmetic saturates at the domain bounds.
- Tab/BackTab change focus only. They never feed a rounded counterpart back
  into the original selection; a 3-row to 2-row mapping must survive a focus
  round trip without losing the original middle row.
- `l` cycles locked, proportional, and unlinked modes. Explicit `set_mode`
  uses the active side as authority once, including a request for the current
  mode. In unlinked mode, cursor movement leaves the other cursor/window alone.
- Esc returns `Close(false)`, leaves the state unchanged, and emits no intent.
  Enter, Left/Right, Backspace, and unrelated character/control/other keys are
  ignored; representative ignored keys are in the explored alphabet.

Selection is cursor-first: map the active cursor once, then keep both cursors
inside their windows. Unequal expanded regions can have differently aligned
window tops. The API does not promise exact mapped top offsets while keeping
both selected locations visible; those promises conflict for expansions such
as 10 source rows corresponding to 100 preview rows in 5-row windows.
Independent wheel/top-anchor scrolling is not part of this keyboard slice.

A clamped selection request that leaves the active cursor unchanged is a no-op.
Pressing Up on row zero after switching focus cannot silently move the other
selection. `resize` preserves both cursor addresses and link evidence while
clamping windows. Zero-height viewports use an effective one-row logical window
for bounds; they make no claim that a cursor is physically visible. The host
still decides whether any cells can be drawn.

Proportional mapping aligns the first and last actual rows using integer
arithmetic: `floor(i * (destination_count - 1) / (source_count - 1))`, with
explicit empty/single-row cases. A `u128` intermediate prevents overflow across
the supported 32-bit and 64-bit `usize` domains.

## Evidence and remaining composition

The bounded fixture has two surfaces, three regions including a deletion and
an uncovered gap, and two-row windows. Its full 17-key alphabet visits 272
states and 4,624 transitions with `exhausted: true`. Separate tests cover
unequal expansion, insertion/deletion boundaries, gap ties, malformed mappings,
zero/single/maximum counts and capacities, resize, ignored keys, and fingerprint
distinctions. Registered mutations pin rounded focus feedback, saturated
navigation, false counterpart rows, omitted scroll state, and host effect entry
points. The source boundary check supplements the empty runtime dependency
closure; it is not a general effect system for arbitrary Rust.

The diff-specific file/hunk/line cursor, context expansion, review intent
vocabulary, and changeset decisions remain the next #19 composition. This
component does not stage files or parse patches. Real terminal ownership and
host repaint behavior still require the host's acceptance tests.
