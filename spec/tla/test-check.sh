#!/usr/bin/env bash
# Regression tests for the fail-closed TLC runner (spec/tla/check.sh).
# Proves a green exit means specs were actually checked, and every degenerate
# input fails closed. Run: spec/tla/test-check.sh
#
# Rows 9-14 are the MUTATIONS FOR THE MANIFEST. Each of check.sh's four
# validations gets a row that turns it red, because a validation with no
# executed counterexample is a claim, not a gate. Row 9 in particular used to
# assert the OPPOSITE — that a `.tla` with no `.cfg` was silently skipped and
# the run stayed green. Recording a defect is not fixing it, and a proof gate
# that green-lights its own silent skip is the worst instance of the class this
# directory exists to pin. It now asserts the failure.
set -uo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
check="$here/check.sh"
pass=0 fail=0

ok()   { printf 'ok   - %s\n' "$1"; pass=$((pass+1)); }
bad()  { printf 'FAIL - %s\n' "$1"; fail=$((fail+1)); }

# A minimal checkable module, so a fixture directory can hold a real pair.
mkpair() {  # mkpair <dir> <Name>
  printf '%s\n' "---- MODULE $2 ----" 'VARIABLE x' 'Init == x = 0' \
    'Next == UNCHANGED x' '====' > "$1/$2.tla"
  printf '%s\n' 'INIT Init' 'NEXT Next' > "$1/$2.cfg"
}

# expect_exit <expected> <desc> -- <cmd...>
expect_exit() {
  local want="$1" desc="$2"; shift 3   # drop the "--"
  "$@" >/dev/null 2>&1; local got=$?
  if [ "$want" = "nonzero" ]; then
    [ "$got" -ne 0 ] && ok "$desc (exit $got)" || bad "$desc (expected nonzero, got 0)"
  else
    [ "$got" -eq "$want" ] && ok "$desc" || bad "$desc (expected $want, got $got)"
  fi
}

# 1. Happy path: the real ExplorerReplay spec checks clean (invokes TLC).
#    (newt-agent's copy of this file points at its `Smoke` module; newtui has
#    no Smoke — the shipped model is the happy path, and a second module whose
#    only job is to be checkable would be a file with nothing to say.)
expect_exit 0 "ExplorerReplay checks clean" -- bash "$check" ExplorerReplay

# 2. A requested spec that is not in the manifest fails (it cannot exist and be
#    unlisted — validations 2 and 3 forbid that — so this is one check, not two).
expect_exit nonzero "requested nonexistent spec fails" -- bash "$check" DoesNotExist

# 3. Unsafe / path-traversal spec names are rejected.
expect_exit nonzero "path-traversal spec name rejected" -- bash "$check" ../Smoke
expect_exit nonzero "slash in spec name rejected" -- bash "$check" a/b

# 4. An EMPTY dir fails (no manifest, so nothing could be checked).
empty="$(mktemp -d)"; trap 'rm -rf "$empty"' EXIT
expect_exit nonzero "empty dir default-discovery fails" -- \
  env TLA_SPEC_DIR="$empty" bash "$check"

# 5. A .cfg with no matching .tla → validation (1) fires.
cfgonly="$(mktemp -d)"; touch "$cfgonly/Foo.cfg"; printf 'Foo\n' > "$cfgonly/models.txt"
expect_exit nonzero "Foo.cfg without Foo.tla → no pair, fails" -- \
  env TLA_SPEC_DIR="$cfgonly" bash "$check"
rm -rf "$cfgonly"

# 6. A .tla with no .cfg, requested explicitly → fails.
tlaonly="$(mktemp -d)"; printf '%s\n' '---- MODULE Foo ----' '====' > "$tlaonly/Foo.tla"
printf 'Foo\n' > "$tlaonly/models.txt"
expect_exit nonzero "requested Foo without Foo.cfg fails" -- \
  env TLA_SPEC_DIR="$tlaonly" bash "$check" Foo
rm -rf "$tlaonly"

# 7. A wrong-version $TLA2TOOLS_JAR is refused (not silently run).
badjar="$(mktemp)"; printf 'not a jar' > "$badjar"
expect_exit nonzero "wrong-version \$TLA2TOOLS_JAR refused" -- \
  env TLA2TOOLS_JAR="$badjar" bash "$check" ExplorerReplay
rm -f "$badjar"

# 8. A directory holding ONLY a .tla (no .cfg) has no checkable pair → fails.
tlanocfg="$(mktemp -d)"; printf '%s\n' '---- MODULE Foo ----' '====' > "$tlanocfg/Foo.tla"
printf 'Foo\n' > "$tlanocfg/models.txt"
expect_exit nonzero "dir with only Foo.tla → no pair, fails" -- \
  env TLA_SPEC_DIR="$tlanocfg" bash "$check"
rm -rf "$tlanocfg"

# ── The four manifest validations, each with the mutation that turns it red ──

# 9. THE SILENT SKIP — WAS BLESSED HERE, NOW REFUSED. A `.tla` with no `.cfg`
#    beside a complete pair used to be dropped by `for cfg in *.cfg` WITHOUT A
#    WORD, and the run exited green on a count that did not include it. This row
#    asserted that behaviour and called it a "known gap". Under the manifest,
#    validation (3) names the file and the run fails. This is the regression
#    test for the fix, and it is the mutation for (3).
mixed="$(mktemp -d)"
printf '%s\n' '---- MODULE Skipped ----' '====' > "$mixed/Skipped.tla"
mkpair "$mixed" Pair
printf 'Pair\n' > "$mixed/models.txt"
out="$(TLA_SPEC_DIR="$mixed" bash "$check" 2>&1)"; code=$?
if [ "$code" -ne 0 ] && printf '%s' "$out" | grep -q 'Skipped.tla is not named in models.txt'; then
  ok "(3) an unlisted root .tla FAILS the run and is named (exit $code)"
else
  bad "(3) unlisted Skipped.tla did not fail with its own name (exit $code)"
fi
# And the escape hatch is location, not silence: moved to lib/, it is a support
# module and the same directory checks clean. Without this half, "make it fail"
# could be satisfied by a rule that also rejects every legitimate import.
mkdir -p "$mixed/lib" && mv "$mixed/Skipped.tla" "$mixed/lib/Skipped.tla"
expect_exit 0 "(3) the same module under lib/ is a support module, run is green" -- \
  env TLA_SPEC_DIR="$mixed" bash "$check"
rm -rf "$mixed"

# 10. (2) A root .cfg that nobody listed. The mirror of row 9 — the manifest has
#     to be exhaustive in BOTH directions or a model can exist and go unchecked
#     by being left out of the list rather than by lacking a file.
unlisted="$(mktemp -d)"; mkpair "$unlisted" Pair; mkpair "$unlisted" Orphan
printf 'Pair\n' > "$unlisted/models.txt"
out="$(TLA_SPEC_DIR="$unlisted" bash "$check" 2>&1)"; code=$?
if [ "$code" -ne 0 ] && printf '%s' "$out" | grep -q 'Orphan\.\(cfg\|tla\) is not named in models.txt'; then
  ok "(2) a root .cfg missing from the manifest FAILS the run (exit $code)"
else
  bad "(2) unlisted Orphan.cfg did not fail (exit $code)"
fi
rm -rf "$unlisted"

# 11. (1) A manifest entry with no files at all.
ghost="$(mktemp -d)"; mkpair "$ghost" Pair; printf 'Pair\nGhost\n' > "$ghost/models.txt"
out="$(TLA_SPEC_DIR="$ghost" bash "$check" 2>&1)"; code=$?
if [ "$code" -ne 0 ] && printf '%s' "$out" | grep -q "lists 'Ghost'"; then
  ok "(1) a manifest entry with no .tla FAILS the run (exit $code)"
else
  bad "(1) phantom manifest entry did not fail (exit $code)"
fi
rm -rf "$ghost"

# 12. THE EXPECTED-RED ROW THE REVIEW ASKED FOR: a model DROPPED from the
#     manifest. Both files present, both valid, simply not listed. Under the old
#     glob this was invisible in the other direction; here (2)/(3) catch it and
#     name the file, so an omission cannot be a silent green.
dropped="$(mktemp -d)"; mkpair "$dropped" Kept; mkpair "$dropped" Forgotten
printf 'Kept\nForgotten\n' > "$dropped/models.txt"
expect_exit 0 "a two-model manifest checks both" -- \
  env TLA_SPEC_DIR="$dropped" bash "$check"
printf 'Kept\n' > "$dropped/models.txt"          # drop one — the mutation
out="$(TLA_SPEC_DIR="$dropped" bash "$check" 2>&1)"; code=$?
if [ "$code" -ne 0 ] && printf '%s' "$out" | grep -q 'Forgotten'; then
  ok "a model dropped from the manifest FAILS and is named (exit $code)"
else
  bad "dropping Forgotten from models.txt did not fail (exit $code)"
fi
rm -rf "$dropped"

# 13. The manifest itself must exist and must name something. `models.txt`
#     deleted, or emptied to comments, is a run that would check nothing.
nomanifest="$(mktemp -d)"; mkpair "$nomanifest" Pair
expect_exit nonzero "no models.txt at all → fails" -- \
  env TLA_SPEC_DIR="$nomanifest" bash "$check"
printf '# only a comment\n' > "$nomanifest/models.txt"
expect_exit nonzero "models.txt naming nothing → fails" -- \
  env TLA_SPEC_DIR="$nomanifest" bash "$check"
printf 'Pair\nPair\n' > "$nomanifest/models.txt"
expect_exit nonzero "a duplicated manifest entry → fails" -- \
  env TLA_SPEC_DIR="$nomanifest" bash "$check"
rm -rf "$nomanifest"

# 14. (4) THE COUNT. The three validations above police the INPUTS; this one
#     polices the OUTCOME, and it is the only one that survives a discovery bug
#     reintroduced below the manifest. Mutate the runner itself — a `continue`
#     in the check loop, the exact shape of the bug this file used to bless —
#     and the count assertion must fail even though every input is valid.
mutant="$(mktemp -d)"; mkpair "$mutant" Pair; mkpair "$mutant" Second
printf 'Pair\nSecond\n' > "$mutant/models.txt"
sed 's|^  log "TLC checking \${spec}.tla …"|  log "TLC checking ${spec}.tla …"; [ "$spec" = Second ] \&\& continue|' \
  "$check" > "$mutant/check-mutant.sh"
# The needle is the SEEDED text, not the word `continue` — check.sh already
# contains one, so a laxer grep would pass on an un-mutated copy.
if ! grep -qF '[ "$spec" = Second ] && continue' "$mutant/check-mutant.sh"; then
  bad "(4) could not seed the skip mutation — the sed no longer matches check.sh"
else
  out="$(TLA_SPEC_DIR="$mutant" bash "$mutant/check-mutant.sh" 2>&1)"; code=$?
  if [ "$code" -ne 0 ] && printf '%s' "$out" | grep -q 'a spec was skipped'; then
    ok "(4) a runner that skips a spec FAILS on the count (exit $code)"
  else
    bad "(4) seeded skip did not trip the count assertion (exit $code)"
  fi
fi
rm -rf "$mutant"

# 15. The SAME assertion, one level up, in scripts/check-mutations.sh. It used
#     to glob `mutations/*.cfg` and compute its FLOOR from that same glob, so a
#     mutation `.tla` with no `.cfg` vanished and the floor could not tell. It
#     now reads `mutations/models.txt`, which check.sh validates on every call —
#     but the verdict loop can still shrink, and a run that reports "4 passed,
#     0 failed" while never touching the fifth mutation is a green that measured
#     less than it claimed. Seed exactly that.
cmroot="$(mktemp -d)"
sed 's|^  cfg="\$muts/\$name.cfg"|  cfg="$muts/$name.cfg"; [ "$name" = NarrowOff ] \&\& continue|' \
  "$here/../../scripts/check-mutations.sh" > "$cmroot/mutant.sh"
if ! grep -qF '[ "$name" = NarrowOff ] && continue' "$cmroot/mutant.sh"; then
  bad "check-mutations count: could not seed the skip — the sed no longer matches"
else
  out="$(NEWTUI_REPO_ROOT="$here/../.." bash "$cmroot/mutant.sh" 2>&1)"; code=$?
  if [ "$code" -ne 0 ] && printf '%s' "$out" | grep -q 'a mutation was skipped'; then
    ok "check-mutations: a skipped mutation FAILS on the verdict count (exit $code)"
  else
    bad "check-mutations: seeded skip did not trip the count assertion (exit $code)"
  fi
fi
rm -rf "$cmroot"

printf '\n%d passed, %d failed\n' "$pass" "$fail"
[ "$fail" -eq 0 ]
