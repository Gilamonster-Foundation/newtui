#!/usr/bin/env bash
# The EXPECTED-RED runner for the Lean gate — spec/tla/mutations/, for proofs.
#
# Every guard gets a mutation that turns it RED, EXECUTED. The TLA+ side has had
# `scripts/check-mutations.sh` since this branch opened; the Lean side had a
# table in a pull request body, hand-run once. That asymmetry is how "zero
# axioms" survived as a claim nothing checked: the gate rejected `sorry` and
# `native_decide`, `#print axioms` printed an `info` message, and nobody had run
# the experiment that would have shown the difference.
#
# Each mutation is APPENDED to a copy of formal/, never applied in place: an
# interrupted in-place run leaves a seeded axiom in the tree, and a mutation
# runner that can poison the thing it audits is worse than none.
#
# THE FOUR MUTATIONS, and which gate each must trip:
#
#   M1  `sorry`                     → check-lean-proofs.sh needle 1
#   M2  `axiom` declaration          → check-lean-proofs.sh needle 3a
#   M3  a theorem with no audit line → check-lean-proofs.sh needle 3b
#   M4  a proof via `Classical`      → `lake build`, on #guard_msgs
#
# M4 IS THE ONE THAT MATTERS, and the reason the other three are not enough.
# It declares nothing, so M2's grep is silent. It carries its own audit line, so
# M3 is silent. It contains no `sorry`, so M1 is silent. The ONLY thing that
# sees it is `#guard_msgs`, because the theorem picks up `Classical.choice` from
# core and the pinned message stops matching. Assert against `lake build`
# directly — "Lean printed something" is not the claim; "the formal gate exits
# non-zero" is.
#
# PIPELINE PARITY: mirrored by .github/workflows/formal.yml (job `lean`) and by
# the `just lean-mutations` recipe. Editing one means auditing the others.
set -uo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
readonly gate="$root/scripts/check-lean-proofs.sh"
readonly formal="$root/formal"
readonly target_rel="Newtui/Fingerprint.lean"

pass=0 fail=0
ok()  { printf 'ok   - %s\n' "$1"; pass=$((pass + 1)); }
bad() { printf 'FAIL - %s\n' "$1"; fail=$((fail + 1)); }

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT

# A pristine copy, .lake included so `lake build` is incremental rather than a
# cold toolchain run. If it has never been built, M4 simply takes longer.
cp -a "$formal" "$work/formal"
readonly tree="$work/formal"
readonly target="$tree/$target_rel"
readonly pristine="$work/pristine.lean"
cp "$target" "$pristine"

restore() { cp "$pristine" "$target"; }

# ── Control: the UNMUTATED copy must be green under both gates ──────────────
# Without this the four reds below are uninformative — a tree that fails for an
# unrelated reason would "pass" every expected-red row. This is the row that
# says the copy is a faithful one.
if NEWTUI_FORMAL_DIR="$tree" bash "$gate" >/dev/null 2>&1; then
  ok "control: the unmutated copy passes check-lean-proofs.sh"
else
  bad "control: the unmutated copy FAILS the gate — every red below is meaningless"
fi

# ── M1-M3: the text gate ────────────────────────────────────────────────────
# seeded_text <label> <needle-in-stderr> <lean...>
seeded_text() {
  local label="$1" needle="$2"; shift 2
  restore
  printf '\n%s\n' "$@" >> "$target"
  local out code
  out="$(NEWTUI_FORMAL_DIR="$tree" bash "$gate" 2>&1)"; code=$?
  if [ "$code" -eq 0 ]; then
    bad "$label: the gate stayed GREEN"
  elif printf '%s' "$out" | grep -qF "$needle"; then
    ok "$label: gate RED (exit $code)"
  else
    bad "$label: went red, but not on '$needle' — a red for the wrong reason is a green in disguise"
    printf '%s\n' "$out" | head -4 >&2
  fi
}

# M1. The `sorry` gate, previously verified by hand in a pull request body.
seeded_text "M1 sorry" "an unproven or unkernel-checked declaration" \
  'namespace Newtui' \
  'theorem seeded_stub : 1 = 1 := by sorry' \
  'end Newtui'

# M2. A declared assumption. `lake build` exits 0 on this; the grep must not.
seeded_text "M2 declared assumption" 'declaration is above' \
  'namespace Newtui' \
  'axiom seeded_false : False' \
  'theorem seeded_uses_it : 0 = 1 := seeded_false.elim' \
  'end Newtui'

# M3. A theorem that simply lands outside the audit — no assumption, nothing
# unproven, just unwatched. This is the hole a per-theorem audit has if nothing
# forces the audit to be total.
seeded_text "M3 unaudited theorem" "has no '#print axioms" \
  'namespace Newtui' \
  'theorem seeded_unaudited : 1 = 1 := rfl' \
  'end Newtui'

# ── M4: the build gate ──────────────────────────────────────────────────────
# The mutation no grep can see. `Classical.byContradiction` pulls
# `Classical.choice` in from core; the theorem carries an audit line claiming no
# dependencies, so ONLY the pinned #guard_msgs message can tell.
restore
cat >> "$target" <<'LEAN'

namespace Newtui
theorem seeded_classical : Consistent sneak :=
  fun _ _ h => Classical.byContradiction fun hn => hn h
/-- info: 'Newtui.seeded_classical' does not depend on any axioms -/
#guard_msgs in
#print axioms seeded_classical
end Newtui
LEAN

# First: the text gate is BLIND to it. If this ever fails, M4 has stopped being
# the mutation it claims to be and is proving something the grep already knew.
if NEWTUI_FORMAL_DIR="$tree" bash "$gate" >/dev/null 2>&1; then
  ok "M4 precondition: check-lean-proofs.sh cannot see it (this is the point)"
else
  bad "M4 precondition: the text gate caught it, so it no longer tests #guard_msgs"
fi

out="$(cd "$tree" && lake build 2>&1)"; code=$?
if [ "$code" -eq 0 ]; then
  bad "M4 Classical-backed theorem: \`lake build\` stayed GREEN — #guard_msgs is not enforcing"
elif printf '%s' "$out" | grep -qF 'Docstring on `#guard_msgs` does not match'; then
  if printf '%s' "$out" | grep -qF 'Classical.choice'; then
    ok "M4 Classical-backed theorem: \`lake build\` RED on #guard_msgs, naming Classical.choice (exit $code)"
  else
    bad "M4: red on #guard_msgs but Classical.choice is not in the message"
    printf '%s\n' "$out" | grep -i axiom | head -4 >&2
  fi
else
  bad "M4: \`lake build\` failed for some other reason (exit $code)"
  printf '%s\n' "$out" | tail -6 >&2
fi

restore

printf '\n%d passed, %d failed\n' "$pass" "$fail"
[ "$fail" -eq 0 ]
