#!/usr/bin/env bash
# THE BRIDGE — the only thing tying spec/tla/ExplorerReplay.tla to src/.
#
# A hand-written TLA+ module is a model of `explore.rs` that NOTHING forces to
# agree with it. `examples/gen_model.rs` runs the real `Explorer::explore` over
# a Rust component implementing exactly the model's transition function — once
# with a pure factory and once with a draining one — and emits five report
# facts per world as `spec/tla/lib/RustObs.tla`. The model computes those same
# five facts from its own transitions; `ModelMatchesRust` compares.
#
# TWO GATES, and both are needed:
#   1. THIS SCRIPT — regenerate and diff. A change to `explore.rs` that moves a
#      counter makes the committed RustObs.tla stale, and this fails.
#   2. TLC on ExplorerReplay.cfg — `ModelMatchesRust`. Once someone regenerates,
#      the model still computes the OLD numbers and TLC goes red.
#
# The pair is why this is not a golden vector. Regeneration rewrites only the
# RUST side, so a drifted model stays red after regeneration; it cannot heal
# into "somebody edited the number to match".
#
# HONEST LIMITS, stated so nobody oversells this:
#   * it binds FIVE OBSERVATIONS PER CAP in TWO world modes ("free" and
#     "drain"), not a refinement. "drain" became bindable when the departure
#     guard landed; "prewarmed" stays unbound on purpose, because it is
#     indistinguishable from "free" from inside the run, so a binding would
#     add nothing and imply something;
#   * it binds `Explorer::explore` only, which is now the crate's only walk —
#     `Explorer::states` and its bug 3a are deleted;
#   * a model change that is wrong in a way the four counters cannot see is
#     invisible to it.
#
# Usage: scripts/check-model.sh          # verify the committed copy
#        scripts/check-model.sh --write  # regenerate it
#
# PIPELINE PARITY: mirrored by .github/workflows/formal.yml (job `bridge`) and
# by the `just model` recipe, which `just check` runs. That CI job carries NO
# paths filter, deliberately: a change to src/explore.rs must reach it.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
readonly target="$root/spec/tla/lib/RustObs.tla"

# POSITIVE READ ASSERTION. A generator that produced nothing — a build that
# silently emitted an empty file, a redirect into the wrong path — would make
# an empty diff look like agreement. The floor is the module header plus the
# four Obs definitions.
readonly FLOOR_LINES=12

fresh="$(mktemp)"
trap 'rm -f "$fresh"' EXIT

( cd "$root" && cargo run --quiet --example gen_model ) > "$fresh"

lines="$(wc -l < "$fresh")"
if [ "$lines" -lt "$FLOOR_LINES" ]; then
  echo "check-model: the generator produced $lines lines, expected at least $FLOOR_LINES." >&2
  echo "An empty or truncated generation would make the diff below vacuous." >&2
  exit 1
fi
grep -q '^ObsExhausted' "$fresh" || {
  echo "check-model: generated output has no ObsExhausted definition." >&2
  exit 1
}

if [ "${1:-}" = "--write" ]; then
  cp "$fresh" "$target"
  echo "check-model: wrote $target ($lines lines)."
  exit 0
fi

if ! diff -u "$target" "$fresh"; then
  echo >&2
  echo "check-model: spec/tla/lib/RustObs.tla has DRIFTED from the crate." >&2
  echo "The explorer's observable report changed. Regenerate with" >&2
  echo "  scripts/check-model.sh --write" >&2
  echo "and then EXPECT TLC to go red on ModelMatchesRust until the model in" >&2
  echo "spec/tla/ExplorerReplay.tla is brought back into agreement. That red is" >&2
  echo "the point of this gate: the model does not get to follow the code" >&2
  echo "silently." >&2
  exit 1
fi

echo "check-model: RustObs.tla matches a fresh $lines-line generation."
