#!/usr/bin/env bash
# The EXPECTED-RED runner — the mechanism that stops the mutation table rotting.
#
# Every invariant in spec/tla/ needs a mutation that turns it RED, or it is
# decoration. Every design that proposed this shipped its mutation table as
# PROSE, to be re-run by hand. This repository has already lost that bet: bug
# 3a landed ONE COMMIT after the fix that created the `exhausted` flag, in a
# crate whose author was actively thinking about that flag. This script is the
# table, executed.
#
# WHY IT IS NOT `spec/tla/check.sh`. That runner is fail-closed on RED — it
# `die`s on the first TLC failure, INSIDE the discovery loop, so one red spec
# aborts the run and every spec after it is never checked. A permanently-red
# residue configuration cannot enter CI through it at all. So mutations live in
# spec/tla/mutations/, outside `check.sh`'s discovery, and are run one at a
# time from here with the verdict inverted.
#
# THE VERDICT IS DECLARED IN THE CFG, first line:
#   \* EXPECT-RED: <InvariantOrPropertyName>
#   \* EXPECT-GREEN: <one-line reason>
#
# EXPECT-RED asserts BOTH a non-zero exit AND that TLC named THAT invariant.
# A red for the wrong reason is a green in disguise: it means the mutation is
# being caught by a different guard than the one it exists to exercise, and the
# guard under test could already be dead.
#
# EXPECT-GREEN is for NEGATIVE CONTROLS — a mutation that removes an
# anti-vacuity decoy and makes the detector next door go blind. Asserting that
# it goes green is what proves the decoy is load-bearing.
#
# POSITIVE READ ASSERTION. An absence check fails OPEN: a moved directory or a
# typo in the glob would report "clean" forever. This asserts a floor on how
# many mutation configurations it actually read.
#
# THE SET COMES FROM `mutations/models.txt`, NOT FROM A GLOB. The glob had the
# same silent-skip hole as check.sh's discovery did: a mutation `.tla` whose
# `.cfg` was never written vanished from the run, and the FLOOR — computed from
# that same glob — could not tell the difference. check.sh validates the
# manifest in four directions on every call, so listing a mutation without
# writing both files is a hard failure here too.
#
# PIPELINE PARITY: mirrored by .github/workflows/formal.yml (job `tla`) and by
# the `just mutations` recipe. Editing one means auditing the others.
set -uo pipefail

# NEWTUI_REPO_ROOT exists so test-check.sh can run a SEEDED COPY of this script
# from outside the tree. Same shape as check.sh's TLA_SPEC_DIR and
# check-lean-proofs.sh's NEWTUI_FORMAL_DIR.
root="${NEWTUI_REPO_ROOT:-$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)}"
readonly muts="$root/spec/tla/mutations"
readonly check="$root/spec/tla/check.sh"
readonly manifest="$muts/models.txt"

# The floor is the hand-written set as of the branch that introduced it. It may
# only go UP. If a mutation is deleted, this fails and someone has to say why
# in a commit message rather than in silence.
readonly FLOOR=5

pass=0 fail=0
ok()  { printf 'ok   - %s\n' "$1"; pass=$((pass + 1)); }
bad() { printf 'FAIL - %s\n' "$1"; fail=$((fail + 1)); }

[ -f "$manifest" ] || { echo "check-mutations: no manifest at $manifest" >&2; exit 1; }

names=()
while IFS= read -r line; do
  line="${line%%#*}"
  line="${line#"${line%%[![:space:]]*}"}"
  line="${line%"${line##*[![:space:]]}"}"
  [ -n "$line" ] && names+=("$line")
done < "$manifest"

if [ "${#names[@]}" -lt "$FLOOR" ]; then
  echo "check-mutations: $manifest names ${#names[@]} mutation(s), expected at least $FLOOR." >&2
  echo "The scan is not reading the mutation manifest, so a clean run from it" >&2
  echo "would mean nothing. Fix the path, or say in the commit why a mutation" >&2
  echo "was deleted." >&2
  exit 1
fi

for name in "${names[@]}"; do
  cfg="$muts/$name.cfg"
  if [ ! -f "$cfg" ]; then
    bad "$name: named in models.txt but $cfg does not exist"
    continue
  fi
  verdict="$(sed -n '1s/^\\\* *EXPECT-\(RED\|GREEN\):.*/\1/p' "$cfg")"
  expect="$(sed -n '1s/^\\\* *EXPECT-RED: *\([A-Za-z0-9_]*\).*/\1/p' "$cfg")"

  if [ -z "$verdict" ]; then
    bad "$name: first line is not '\\* EXPECT-RED: <Name>' or '\\* EXPECT-GREEN: <reason>'"
    continue
  fi

  out="$(TLA_SPEC_DIR="$muts" bash "$check" "$name" 2>&1)"
  code=$?

  case "$verdict" in
    RED)
      if [ -z "$expect" ]; then
        bad "$name: EXPECT-RED with no invariant name to check for"
      elif [ "$code" -eq 0 ]; then
        bad "$name: expected RED on $expect, TLC found no error"
      elif printf '%s' "$out" | grep -qE "(Invariant|Action property|Temporal property|Property) $expect is violated"; then
        ok "$name: RED on $expect"
      else
        bad "$name: went red, but NOT on $expect — a red for the wrong reason is a green in disguise"
        printf '%s\n' "$out" | grep -E '^Error' | head -3 >&2
      fi
      ;;
    GREEN)
      if [ "$code" -eq 0 ]; then
        ok "$name: GREEN (negative control)"
      else
        bad "$name: expected GREEN, TLC failed (exit $code)"
        printf '%s\n' "$out" | grep -E '^Error' | head -3 >&2
      fi
      ;;
  esac
done

printf '\n%d passed, %d failed, %d mutation configurations read\n' \
  "$pass" "$fail" "${#names[@]}"

# Same shape as check.sh's (4): every manifest entry must have produced a
# verdict. A `continue` above must not be able to quietly shrink the run.
if [ "$((pass + fail))" -ne "${#names[@]}" ]; then
  echo "check-mutations: $((pass + fail)) verdict(s) for ${#names[@]} manifest entries — a mutation was skipped" >&2
  exit 1
fi
[ "$fail" -eq 0 ]
