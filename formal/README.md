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

### The trusted base, and how it is kept

Every theorem below reports **no axiom dependencies at all** — not even
`propext`. `quotStep` is the one exception: it is built on `Quotient.lift`,
whose soundness is *stated* in terms of `Quot.sound`, one of Lean's three
kernel axioms, so it reports `[Quot.sound]`. That is not something a different
proof avoids, and it is recorded rather than hidden.

**That paragraph is not a promise, it is a build artifact.** It used to be a
promise: `scripts/check-lean-proofs.sh` rejected `sorry` and `native_decide`
only, `#print axioms` printed an `info` message, and a theorem that acquired a
dependency tomorrow would print something new while the gate stayed green. The
claim was *reported*, never *enforced*, and preservation is the whole point.
Three mechanisms now hold it:

1. every audited declaration carries `#guard_msgs in #print axioms <name>` in
   `Fingerprint.lean`, so the expected message is compared during `lake build`
   and a mismatch is an **error**;
2. `check-lean-proofs.sh` rejects an `axiom` declaration, and requires every
   `theorem` in the tree to have an audit line — a subset audit is just a place
   for the next theorem to land unwatched;
3. `scripts/check-lean-mutations.sh` seeds four mutations and asserts each goes
   red. One of them (a proof routed through `Classical.byContradiction`) is
   invisible to every grep and is caught **only** by the pinned message. That is
   the row that shows `#guard_msgs` is doing work.

Every `decide` is a kernel reduction; the compiled-code tactic that would add
`Lean.ofReduceBool` to the trusted base is refused by the same script.

| Theorem | Status | What it establishes | What it does **not** establish |
|---|---|---|---|
| `ofEncodedView_consistent_of_injective` | **proven** | The Rust default `Fingerprint::of_view` is `fp = enc ∘ view` for a serialiser `enc`, and its observation-consistency is exactly `enc` being **injective**. The obligation `of_view`'s doc comment does not state, stated. | That newtui's `enc` *is* injective. It is not — see the next row. |
| `smudge_not_consistent` | **proven, constructive** | `component.rs:48` joins the title and the footer with U+001F, so `("a\u{1f}b","c")` and `("a","b\u{1f}c")` serialise identically while showing different things. The shipped default is **not** observation-consistent, and the collision guard *would* fire on it. Reproduced against the crate, then written down as a term. | That a real component hits it. Nothing in the type system forbids U+001F in a title; nothing observed one either. |
| `ofView_consistent` | **proven** (`enc = id`) | With an injective default — the structural `Fingerprint` the sibling branch `fix/false-completeness-class` is building, a `View` base plus a `Vec<String>` of extras — the observation half holds by construction, so the cheap guard (record the `View`, assert on a collision) cannot fire on a component that took the default. This changed a decision: two designs were about to bill that guard as closing the observation half outright. | **Anything about `src/` as it stands today.** This is the theorem that used to be billed "checked against `component.rs:48-63`" and was not: the check missed that `Fingerprint` is a `String`. It becomes a live correspondence when the sibling branch lands, and not before. |
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

## The proof gates are real, and the proof is executed, not transcribed

`scripts/check-lean-mutations.sh` seeds four mutations into a copy of this tree
and asserts each one goes red. It replaces a hand-run transcript that used to
live in this section — a table someone ran once is exactly the artifact this
whole layer exists to refuse.

| | mutation | must be caught by |
|---|---|---|
| M1 | an unproven declaration | `check-lean-proofs.sh`, needle 1 |
| M2 | a declared assumption | `check-lean-proofs.sh`, needle 3a |
| M3 | a theorem with no audit line | `check-lean-proofs.sh`, needle 3b |
| M4 | a proof routed through `Classical` | **`lake build`**, on `#guard_msgs` |

**M4 is the one that matters**, and the reason the other three are not enough.
It declares nothing, so M2's grep is silent; it carries its own audit line, so
M3 is silent; it is fully proven, so M1 is silent. The only thing that sees it
is the pinned `#print axioms` message, because the theorem picks up
`Classical.choice` from core and the expected output stops matching. The runner
asserts the precondition too — that the text gate is blind to M4 — because
otherwise the row could pass while proving something the grep already knew.

All four exist because **`lake build` exits 0 on every one of them except M4**.
An unproven theorem is a warning; a declared assumption is legal; an unaudited
theorem is ordinary Lean. The gate is what makes any of it mechanical.

The text gate is deliberately **comment-blind**: it does not parse Lean, so the
`sorry`/`native_decide` needles also fire on a mention inside a comment — which
is why this README spells them obliquely. That is the right trade; the
alternative is a comment-aware scanner whose bugs all fail open. If a hit is a
mention, reword the comment. Never weaken the needle. The assumption needle is
the one exception: it is anchored to the start of a line, because a file whose
job is auditing the trusted base says the word in prose constantly and a needle
people fight every week gets weakened within a month. `Fingerprint.lean`'s
prose is reflowed to respect it, and needle 3b is what makes the smaller needle
safe — anything it misses still has to pass `#guard_msgs`.

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
