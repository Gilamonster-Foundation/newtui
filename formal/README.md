# `formal/` — the Lean layer

One file, ~140 lines with its header, and it verifies **nothing about `src/`**.
It exists to make one obligation a *type* rather than a paragraph.

```sh
cd formal
lake build                    # checks every theorem; exit 0 iff all proofs go through
../scripts/check-lean-proofs.sh   # the stub gate — `lake build` does NOT do this
```

Or from the repo root: `just lean` and `just no-sorry`. CI runs both in
`.github/workflows/formal.yml`.

## Read this before citing a theorem

**Nothing in Lean reads the Rust.** There is no bridge here, no generated
vector, no extraction — unlike `spec/tla/`, which has one and says what it is
worth. `Newtui/Fingerprint.lean` is a model of the *contract*
`Component::fingerprint` states in prose, and every theorem in it is a
statement about that model.

The vocabulary registry this line uses has three levels — `none`, `spec`,
`proven` — and it is reproduced here even though `newtui` is standalone,
because writing it down is what keeps `spec` from being read as `proven`:

- **`none`** — asserted in prose, checked by nobody.
- **`spec`** — machine-checked *about the model*. A theorem discharged by
  `simp` or `rfl` on its own definitions establishes that the model is
  consistent. It does **not** establish that any code refines it.
- **`proven`** — machine-checked and constrains something beyond the model's
  own definitional shape: a constructive witness, or a result that needed an
  argument rather than unfolding.

`#print axioms` on every theorem below reports **no axiom dependencies at
all** — not even `propext`. Every `decide` is a kernel reduction; the
compiled-code tactic that would add `Lean.ofReduceBool` to the trusted base is
refused by `scripts/check-lean-proofs.sh`.

| Theorem | Status | What it establishes | What it does **not** establish |
|---|---|---|---|
| `ofView_consistent` | **proven** (definitional, but the definition was checked against `component.rs:48-63`) | Under the default fingerprint, the observation half of the congruence holds by construction — so the cheap guard everyone reaches for (record the `View` beside the `Fingerprint`, assert on a collision) **cannot fire** on a component that took the default. This changed a decision: two designs were about to bill that guard as closing the observation half outright. | Anything about separator injection. `of_view` joins fields with `\u{1f}`/`\u{1e}`, and a row label *containing* those bytes is the one way it fails. That is a Rust property test, not a theorem. |
| `creep_not_consistent` | **proven, constructive** | The collision guard is **not** vacuous: it fires on a `Fingerprint::of(..)` seed, which is `seed.to_string()` and therefore arbitrary, and on `Fingerprint::and(..)`, which the crate's own doc tells consumers to reach for. That is the population where the measured false green lives — keep the guard. | That any shipped component has a `creep`-shaped fingerprint. |
| `sneak_consistent`, `sneak_not_transfers`, `sneak_loses_a_reachable_view` | **proven, constructive** | The two halves of the congruence are independent, and the half a collision guard cannot see (`Transfers`) is where the exposure is. `sneak` passes every collision check and still merges two states that show different things one key later. These constrain because they are *terms*, not because a tactic closed a goal. | That any shipped component is or is not a congruence. That is undecidable in general — it is a bisimulation over the whole reachable graph. |
| `consistency_is_not_congruence` | **proven, constructive** | The previous row as one term: `Consistent sneak ∧ ¬ IsCongruence sneak`. | — |
| `quotStep` | **spec (model-internal)** | The obligation is a **type**. `Quotient.lift` takes the congruence proof as an argument, so the step function on the quotient the explorer actually walks does not elaborate without it. Deleting the argument is a type error, mechanically (see below). | That `IsCongruence` is decidable, approximable, or checkable in Rust. It is none of those. |
| `IsCongruence` | **a definition, carried as an explicit hypothesis, never claimed as a theorem** | What a consumer signs by taking the default fingerprint. | Anything, on its own. Its whole value is turning "we did not check this" from an invisible hole into a visible one. |

## The obligation is load-bearing, mechanically

Delete the congruence argument from `quotStep` and the build fails with the
obligation spelled out:

```
$ # replace `Quotient.sound (h _ _ k hab)` with `Quotient.sound hab`
$ lake build
error: Newtui/Fingerprint.lean:100:35: Application type mismatch: The argument
  hab
has type
  x✝¹ ≈ x✝
but is expected to have type
  c.step x✝¹ k ≈ c.step x✝ k
error: build failed
```

## The stub gate is real, and here is the proof

`scripts/check-lean-proofs.sh` greps this tree for unproven and
non-kernel-checked declarations. It exists because **`lake build` exits 0 on an
unproven theorem** — verified on this machine, not assumed:

```
$ printf '\ntheorem seeded_stub : 1 = 1 := by <stub>\n' >> Newtui/Fingerprint.lean
$ lake build >/dev/null 2>&1; echo $?
0                       # green, with an unproven theorem in the tree
$ ../scripts/check-lean-proofs.sh; echo $?
.../Newtui/Fingerprint.lean:157:theorem seeded_stub : 1 = 1 := by <stub>
1                       # the gate catches it
```

The gate is deliberately **comment-blind**: it does not parse Lean, so it also
fires on a mention inside a comment — which is why this README spells the two
needles obliquely and the `.lean` file never names them at all. That is the
right trade; the alternative is a comment-aware scanner whose bugs all fail
open. If a hit is a mention, reword the comment. Never weaken the needle.

Its positive read assertion has **zero margin**: two `.lean` files against a
floor of two. Fine, but nobody gets to tidy a file away.

## What was cut, and why

The design this file implements listed several hundred more lines. Anything
that constrained nothing was deleted rather than kept for the count:

- **A `Verdict` file** (admissibility, `checks > 0`). The mechanism binds and
  the proof does not. A theorem proving `violations.is_empty()` is not
  refutation-complete leaves the README free to keep writing it; a
  three-constructor `Verdict { Clean | Violated | Incomplete(reason) }` makes
  the weak assertion **unwriteable**. Ship the type, skip the theorem.
- **A hand-rolled re-implementation of the BFS in Lean.** Maximum maintenance
  for no catch, and the draft already diverged from the code at the cap where
  the bugs live: it cleared a flag and continued, where `explore.rs:219`
  `return`s a second terminal that abandons the queue.
- **"No Rust type expresses this" as a theorem.** You cannot quantify over
  Rust types in Lean. The honest form is `quotStep`, above: an obligation you
  cannot build without.
- **"The pre-warmed residue is undetectable" as a theorem.** Delivered
  strictly better as a *trace*: `spec/tla/PrewarmedConsistent.cfg` is green and
  `spec/tla/mutations/PrewarmedResidue.cfg` is red on the same configuration,
  and a reader can step through the counterexample. A theorem about a residue
  over an observation power the author gets to define is decoration with a
  proof attached.
- **An impossibility theorem for the bare-`Vec<View>` return type.**
  Functionhood dressed as a theorem. The gate was the return type and it is
  already closed.
- **Golden vectors in Lean.** Wrong layer. `Report` genuinely *is* a pure
  function of (component, alphabet, caps), so vectors apply in principle — but
  they would bridge Lean's re-implementation, not the TLA+ module that catches
  the only live bug. One generator, at the TLA+ layer.

## The direction trap

Stated in the file header too, because every draft of this work got it wrong at
least once. A sound abstraction for ∀-safety is an **over**-approximation
(existential abstraction; Clarke/Grumberg/Long, TOPLAS 1994). The fingerprint
quotient **prunes** — it is an **under**-approximation, which preserves nothing
about safety and is sound only where it removes nothing. And *"step factors
through `fp`"* is false; what holds under `Transfers` is that `fp ∘ step`
factors through `fp`.
