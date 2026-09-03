#!/usr/bin/env bash
# The trusted-base gate — a REAL gate, not a badge. Three needles.
#
# 1. `sorry`. `lake build` exits 0 on a `sorry`: an unproven theorem is a
#    warning, not an error, so "sorry-free" would otherwise be a human assertion
#    that CI never checks. This is the check that makes it mechanical.
#    (newt-agent's `formal/` has the identical gap; it is filed there.)
#
# 2. `native_decide`, refused on the same grounds. It discharges a goal by
#    running compiled code and adds `Lean.ofReduceBool` to the trusted base —
#    the compiler and the runtime join the kernel as things you have to believe.
#
# 3. An `axiom` DECLARATION, and the audit line that must accompany every
#    theorem. "Zero axioms" used to be REPORTED and not ENFORCED: this script
#    rejected `sorry` and `native_decide` only, `#print axioms` printed an
#    `info` message nobody's build depended on, and a future theorem backed by a
#    new axiom would compile, print its dependency decoratively, and leave the
#    gate green. Preservation was the defect, not today's numbers.
#
# WHAT EACH HALF CAN AND CANNOT SEE — the split matters:
#
#   * this script sees an `axiom` declared IN THIS TREE. It cannot see an axiom
#     arriving from core (`Classical.choice` via `Classical.em` or
#     `byContradiction`, `propext`, `Quot.sound`), because no new keyword
#     appears in the file. Those are precisely the ones acquired by accident.
#   * the `#guard_msgs in #print axioms` block in Fingerprint.lean sees ALL of
#     them, from anywhere, because it pins the message and a mismatch is a build
#     ERROR. But it only watches the declarations it names.
#
# So needle 3 has two parts: reject a declared `axiom`, AND require that every
# `theorem` in the tree is named by a `#print axioms` line — closing the gap
# where a new theorem simply lands outside the audit. Both are executed against
# seeded mutations by `scripts/check-lean-mutations.sh`; a gate with no mutation
# that turns it red is decoration.
#
# POSITIVE READ ASSERTION. An absence check fails OPEN: anything that shrinks
# the scanned text makes it MORE likely to pass, so a moved directory or a typo
# in the glob would report "clean" forever. This asserts it actually read the
# files first, and names the count it expects to have grown past.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
# NEWTUI_FORMAL_DIR exists so check-lean-mutations.sh can point this gate at a
# seeded copy of the tree. Same shape as check.sh's TLA_SPEC_DIR.
formal="${NEWTUI_FORMAL_DIR:-$root/formal}"

# The floor is the HAND-WRITTEN tree — the root module and the model. It has
# ZERO MARGIN at two files against a floor of two, which is fine but means
# nobody gets to "tidy" a file away without this failing and saying so.
#
# PIPELINE PARITY: mirrored by .github/workflows/formal.yml (job `lean`) and by
# the `just no-sorry` recipe, which `just check` and .githooks/pre-push both
# run. It is a pure grep with no toolchain, so there is no excuse for it being
# CI-only. Editing one means auditing the others.
mapfile -t files < <(find "$formal" -name '*.lean' -not -path '*/.lake/*' | sort)
if [ "${#files[@]}" -lt 2 ]; then
  echo "check-lean-proofs: found only ${#files[@]} .lean files under $formal." >&2
  echo "The scan is not reading the formal tree, so the absence it reports" >&2
  echo "means nothing. Expected at least Newtui.lean and" >&2
  echo "Newtui/Fingerprint.lean." >&2
  exit 1
fi

if grep -nE '\b(sorry|native_decide)\b' "${files[@]}"; then
  echo >&2
  echo "check-lean-proofs: an unproven or unkernel-checked declaration is above." >&2
  echo "\`lake build\` exits 0 on a \`sorry\`, so this is the only thing standing" >&2
  echo "between a stub and a green badge. Prove it or delete it." >&2
  echo >&2
  echo "This gate is deliberately DUMB: it does not parse Lean, so it also fires" >&2
  echo "on a MENTION inside a comment. That is the right trade — the alternative" >&2
  echo "is a comment-aware scanner whose bugs all fail open. If the hit is a" >&2
  echo "mention, reword the comment; do not weaken the needle." >&2
  exit 1
fi

# ── Needle 3a: an `axiom` DECLARATION ───────────────────────────────────────
# Line-anchored, unlike the needle above, and that is a deliberate difference.
# A file whose job is auditing axioms says the word in prose constantly; a bare
# \baxiom\b would fire on every sentence and the needle would be weakened within
# a week, which is the failure mode the comment above warns about. A DECLARATION
# opens a command, so it starts its line (after optional attributes/modifiers).
# `#print axioms` and "does not depend on any axioms" are plural and never match.
#
# This is a smaller needle than "the word anywhere", so it is not the only
# defence: needle 3b below forces every theorem into the `#guard_msgs` audit,
# where an axiom from ANY source — including one this grep missed — changes the
# pinned message and fails `lake build`.
if grep -nE '^[[:space:]]*(@\[[^]]*\][[:space:]]*)*(private[[:space:]]+|protected[[:space:]]+|noncomputable[[:space:]]+)*axiom[[:space:]]' "${files[@]}"; then
  echo >&2
  echo "check-lean-proofs: an \`axiom\` declaration is above." >&2
  echo "An axiom is an assumption the kernel will not check. Adding one makes" >&2
  echo "every theorem downstream of it conditional, and \`lake build\` would" >&2
  echo "still exit 0. Prove it, or state plainly in the pull request that this" >&2
  echo "tree now has a non-empty trusted base and delete the zero-axiom claim." >&2
  exit 1
fi

# ── Needle 3b: every theorem is named in the axiom audit ────────────────────
# Without this, the audit is a subset, and a subset is where the next theorem
# lands unwatched. Grep-only: a declaration name and a `#print axioms` line, no
# Lean parsing.
#
# THE GRAMMAR THIS RECOGNISES, and it is a grammar rather than the word
# `theorem` because the first version was the word and it had a hole a review
# found: `@[simp] theorem foo` and `lemma foo` both landed OUTSIDE the audit
# with the gate green, still reporting its count of audited declarations. The
# recognised forms are, in order on one line:
#
#   * any number of attribute groups — `@[simp]`, `@[simp, norm_cast]`;
#   * any of the modifiers `private`, `protected`, `noncomputable`;
#   * the keyword `theorem` OR `lemma`;
#   * the name, which may be dotted (`Foo.bar`).
#
# Attributes on their OWN line need nothing: the declaration line then begins
# with the keyword and matches the plain form.
#
# LIMITS, stated because a scanner that does not state them gets trusted past
# them. This does not parse Lean. It will not see a declaration produced by a
# macro or `deriving`, one whose keyword is separated from its attributes by a
# line break in the middle of the attribute list, or an `example`/`instance`
# that carries a proof obligation under another keyword. Needle 3a and the
# `#guard_msgs` pins are the defences that do not depend on this recognition;
# a Lean-environment enumeration would replace it, and is the right next slice.
missing=0
while read -r name; do
  [ -n "$name" ] || continue
  grep -qE "^[[:space:]]*#print axioms[[:space:]]+${name}[[:space:]]*$" "${files[@]}" || {
    echo "check-lean-proofs: '$name' has no '#print axioms $name' audit line." >&2
    missing=$((missing + 1))
  }
done < <(grep -hoE '^[[:space:]]*(@\[[^]]*\][[:space:]]*)*(private[[:space:]]+|protected[[:space:]]+|noncomputable[[:space:]]+)*(theorem|lemma)[[:space:]]+[A-Za-z_][A-Za-z0-9_.'"'"'!?]*' "${files[@]}" \
         | sed -E 's/.*(theorem|lemma)[[:space:]]+//')

if [ "$missing" -gt 0 ]; then
  echo >&2
  echo "$missing declaration(s) are outside the axiom audit. Add, next to the" >&2
  echo "others in Fingerprint.lean's AxiomAudit section:" >&2
  echo >&2
  echo "    /-- info: 'Newtui.<name>' does not depend on any axioms -/" >&2
  echo "    #guard_msgs in" >&2
  echo "    #print axioms <name>" >&2
  echo >&2
  echo "\`#print axioms\` alone prints and passes. \`#guard_msgs\` is what makes" >&2
  echo "the expected message a checked artifact instead of console decoration." >&2
  exit 1
fi

audited="$(grep -hE '^[[:space:]]*#print axioms[[:space:]]' "${files[@]}" | wc -l | tr -d ' ')"
echo "check-lean-proofs: ${#files[@]} Lean files, no sorry, no native_decide," \
     "no axiom declaration, $audited audited declaration(s)."
