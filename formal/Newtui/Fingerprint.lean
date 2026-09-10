/- The `Fingerprint` obligation, as a TYPE.

   ## READ THIS BEFORE CITING THIS FILE

   **Nothing here reads the Rust.** There is no bridge, no generated vector, no
   extraction. This file names and LOCATES an obligation that `newtui`'s
   `Component::fingerprint` contract carries and no Rust type can host; it
   verifies nothing about `src/`. Citing it as "we proved the fingerprint is
   sound" would be worse than citing nothing.

   ## Why this is Lean and not a paragraph

   `Fingerprint`'s contract (component.rs:29-42) is *"equal for two states iff
   they behave the same from here on"*. That is a bisimulation — a relation over
   the whole reachable graph — and it is genuinely not a Rust type. The honest
   formal move is not to fake one; it is to make the obligation an ARGUMENT that
   something cannot be built without. `Quotient.lift` does exactly that:
   `quotStep` below is the step function on the quotient the explorer actually
   walks, and it does not elaborate unless it is handed a proof of `Transfers`.
   Delete the `h` and the file goes red. That is the whole deliverable.

   ## The direction trap

   A sound abstraction for ∀-safety is an OVER-approximation — existential
   abstraction, in the sense of Clarke/Grumberg/Long (TOPLAS 1994): it may lose
   precision but never asserts something false. The fingerprint quotient is the
   opposite. It PRUNES: two states judged equal, one of them never enqueued.
   That is an UNDER-approximation, which preserves nothing about safety and is
   sound only in the case where it removes nothing. Never write "the fingerprint
   is an abstraction" and stop there.

   Vocabulary, all standard, none invented here: bisimulation (Park, Milner);
   observation-consistency and the transfer / zig-zag condition; `ker fp` as a
   right congruence refining Myhill–Nerode; quotient transition system (Baier &
   Katoen ch. 7). One slip worth naming because every draft of this made it:
   *"step factors through fp"* is FALSE. What holds under `Transfers` is that
   `fp ∘ step` factors through `fp`. The concrete successor is not determined by
   the fingerprint, and that difference is the whole content of `quotStep`.

   ## What the two witnesses are for

   `creep` and `sneak` separate the two halves of the congruence, and the
   separation changed a decision. The cheap guard everyone reaches for —
   record the `View` beside the `Fingerprint`, assert on a collision — checks
   `Consistent` and nothing else. `ofView_consistent` says that check cannot
   fire on a component whose default fingerprint is INJECTIVE. `sneak` says the
   half it cannot see (`Transfers`) is where the exposure lives. `creep` says
   the guard is nonetheless load-bearing on a hand-rolled `Fingerprint::of`,
   which is the population where the measured false green was found. Keep the
   guard; bill it accurately.

   THE INJECTIVITY QUALIFIER IS NEW, AND IT IS A CORRECTION. An earlier version
   of this file asserted `ofView_consistent` unconditionally over a model whose
   `fp` was the view itself, and billed it as checked against the Rust. The Rust
   `Fingerprint` is a `String` built by delimiter concatenation, which collides.
   `smudge` is that collision as a term. See the block above `ofEncodedView`.
-/

namespace Newtui

/-- A component, as three decision FUNCTIONS rather than data: how one key
    moves it, what it fingerprints as, what it shows. The Rust trait
    (`component.rs:93-114`) is exactly this triple. -/
structure Comp (S K F V : Type) where
  /-- `Component::handle`, with the `Flow` erased: this file is about the
      fingerprint, and a second defect in one model makes neither diagnostic. -/
  step : S → K → S
  /-- `Component::fingerprint`. -/
  fp : S → F
  /-- `Component::view`. -/
  view : S → V

variable {S K F V : Type}

/-- **Observation consistency.** Equal fingerprints show the same thing. This
    is the half a collision guard can check. -/
def Consistent (c : Comp S K F V) : Prop :=
  ∀ s t, c.fp s = c.fp t → c.view s = c.view t

/-- **Transfer (the zig-zag condition).** Equal fingerprints stay equal after
    any key. This is the half a collision guard cannot see, and the half the
    explorer's pruning actually depends on. -/
def Transfers (c : Comp S K F V) : Prop :=
  ∀ s t k, c.fp s = c.fp t → c.fp (c.step s k) = c.fp (c.step t k)

/-- The contract `Fingerprint`'s doc comment states, in full. Carried as an
    explicit HYPOTHESIS everywhere and proven nowhere — that is deliberate. It
    is what a consumer signs by taking the default, and naming it turns "we did
    not check this" from an invisible hole into a visible one. -/
def IsCongruence (c : Comp S K F V) : Prop := Consistent c ∧ Transfers c

/-- `ker fp` — always an equivalence, congruence or not. That it is an
    equivalence is free; that it respects `step` is the entire question. -/
def fpSetoid (c : Comp S K F V) : Setoid S where
  r a b := c.fp a = c.fp b
  iseqv := ⟨fun _ => rfl, fun h => h.symm, fun h₁ h₂ => h₁.trans h₂⟩

/-- **THE DELIVERABLE.** The step function on the quotient the explorer walks.

    `Quotient.lift` takes the congruence proof as an ARGUMENT, so this
    definition does not typecheck without `h`. The obligation `newtui` states
    in a doc comment is, here, a term that must be supplied. -/
def quotStep (c : Comp S K F V) (h : Transfers c) (k : K) :
    Quotient (fpSetoid c) → Quotient (fpSetoid c) :=
  Quotient.lift (fun s => Quotient.mk (fpSetoid c) (c.step s k))
    (fun _ _ hab => Quotient.sound (h _ _ k hab))

/-! ## The default fingerprint, and the correspondence that was WRONG

    This block previously held one theorem, `ofView_consistent`, over a `ofView`
    whose `fp` was the view ITSELF. It was billed as "checked against
    `component.rs:48-63`" and it was not checked closely enough. `Fingerprint`
    is a `String`. The Rust default is not `fp = view`; it is `fp = enc ∘ view`
    for a serialiser `enc : View → String`, and `enc` is a delimiter
    CONCATENATION:

        format!("{}\u{1f}{}", view.title, view.footer)      -- component.rs:48

    which is not injective, because nothing forbids U+001F inside a title:

        enc ("a\u{1f}b", "c") == "a\u{1f}b\u{1f}c" == enc ("a", "b\u{1f}c")

    Two DIFFERENT views, one fingerprint. Reproduced against the shipped code,
    not reasoned about. So `Consistent` is FALSE of the Rust default as it
    stands, and the old theorem — true of its own model — described an
    implementation that does not exist. A model nothing forces to agree with the
    code is exactly the failure this file's own header warns about, committed in
    the file that warns about it.

    The repair is to model what the Rust does and make the missing property an
    explicit HYPOTHESIS, the same move `IsCongruence` already makes: the default
    is consistent IF AND ONLY IF the serialiser is injective, and the injectivity
    is now something a reader can see is being assumed.

    STATUS AGAINST `src/` — THIS HAS LANDED. `Fingerprint` on `main` is
    structural: a `FingerprintBase` that is either the `View` itself or a
    caller's opaque seed, plus `extra : Vec<String>`, one element per `and`.
    There is no serialiser left to be non-injective, so `enc` is the identity
    on the base and `ofView_consistent` below — the `enc = id` corollary —
    describes the shipped default for the first time.

    Three things that stay true and must not be rounded off:

    * `smudge` and `smudge_not_consistent` are kept as the REGRESSION WITNESS
      for the representation that shipped, not as a description of live code. A
      counterexample retired from the implementation is still the reason the
      hypothesis is stated;
    * consistency of the default is a claim about VIEWS, and the Rust guard
      that pins it was renamed `structural_fingerprints_keep_their_boundaries`
      for the same reason this file distinguishes them: two states the view
      cannot tell apart SHOULD share a fingerprint, and the crate documents
      that non-injectivity as design. `Consistent` here is the behavioural
      property, never state identity;
    * `Fingerprint::of` still takes an arbitrary caller summary. Nothing in
      Lean or in Rust makes a caller's summary injective — that obligation is
      `Transfers`, it is discharged by the CALLER, and `sneak_not_transfers`
      is what a violation of it looks like. -/

/-- The default with the ENCODING made explicit: `fp = enc ∘ view`, which is
    what `Fingerprint::of_view` is. -/
def ofEncodedView (step : S → K → S) (view : S → V) (enc : V → F) : Comp S K F V :=
  { step := step, fp := fun s => enc (view s), view := view }

/-- The default is observation-consistent EXACTLY WHEN its serialiser is
    injective. This is the obligation `of_view` carries and its doc comment does
    not state. -/
theorem ofEncodedView_consistent_of_injective
    (step : S → K → S) (view : S → V) (enc : V → F) (hinj : Function.Injective enc) :
    Consistent (ofEncodedView step view enc) := fun _ _ h => hinj h

/-- A view as `(title, footer)` and a fingerprint as their separator-joined
    concatenation — `component.rs:48` with `Nat` for `Char` and `0` for U+001F,
    which changes nothing about the argument and makes every proof `decide`. -/
def smudge : Comp Bool Unit (List Nat) (List Nat × List Nat) where
  step s _ := s
  fp s := (if s then ([1], [2, 0, 3]) else ([1, 0, 2], [3])).1 ++
          0 :: (if s then ([1], [2, 0, 3]) else ([1, 0, 2], [3])).2
  view s := if s then ([1], [2, 0, 3]) else ([1, 0, 2], [3])

/-- **THE COLLISION, AS A TERM.** Both states serialise to `[1,0,2,0,3]` and
    they show different things — so the shipped default does NOT satisfy
    `Consistent`, and a collision guard comparing views WOULD fire on it. The
    old `ofView_consistent` said the opposite about the same function. -/
theorem smudge_not_consistent : ¬ Consistent smudge := by
  intro h
  exact absurd (h false true rfl) (by decide)

/-- `Fingerprint::of_view` under the representation the crate SHIPS: the
    fingerprint holds the view itself, so `enc = id`. -/
def ofView (step : S → K → S) (view : S → V) : Comp S K V V :=
  ofEncodedView step view id

/-- Under an INJECTIVE default the observation half is definitional, so a guard
    that records the `View` beside the `Fingerprint` and asserts on a collision
    is inert — which is the claim the rest of this file is built on. It holds
    for `enc = id`, which is the structural `Fingerprint` on `main`. It does
    NOT hold for the concatenating `enc` that shipped before it — see `smudge`,
    kept as the witness for why the hypothesis has to be stated at all. -/
theorem ofView_consistent (step : S → K → S) (view : S → V) :
    Consistent (ofView step view) :=
  ofEncodedView_consistent_of_injective step view id fun _ _ h => h

/-- `Fingerprint::of("creep")` — a seed with no relation to the state, which
    `of` permits: it is `seed.to_string()` and therefore arbitrary. -/
def creep : Comp Nat Unit Bool Bool where
  step s _ := if s < 3 then s + 1 else s
  fp _ := true
  view s := decide (3 ≤ s)

/-- The collision guard is NOT vacuous. It fires here, on a hand-rolled
    fingerprint — the population where the measured false green lives. -/
theorem creep_not_consistent : ¬ Consistent creep := by
  intro h
  exact absurd (h 0 3 rfl) (by decide)

/-- **THE RESIDUE.** Views agree, so the guard is silent; behaviour diverges
    one key later. -/
def sneak : Comp Nat Unit Bool Bool where
  step s _ := if s = 0 then 2 else s
  fp s := decide (s < 2)
  view s := decide (s < 2)

theorem sneak_consistent : Consistent sneak := fun _ _ h => h

theorem sneak_not_transfers : ¬ Transfers sneak := by
  intro h
  exact absurd (h 0 1 () rfl) (by decide)

/-- States 0 and 1 merge under `fp`, and one key later they show DIFFERENT
    things. The explorer enqueues one and drops the other, so a view an
    operator can reach is never judged. -/
theorem sneak_loses_a_reachable_view :
    sneak.view (sneak.step 0 ()) ≠ sneak.view (sneak.step 1 ()) := by decide

/-- What a collision guard can and cannot see, as one term: `sneak` passes the
    check and is not a congruence. 100% of the default path's exposure is
    `Transfers`, which no collision check reaches. -/
theorem consistency_is_not_congruence :
    Consistent sneak ∧ ¬ IsCongruence sneak :=
  ⟨sneak_consistent, fun hc => sneak_not_transfers hc.2⟩

/-! ## The axiom audit — a CHECKED artifact, not console decoration

    "Zero axioms" was previously a sentence in a pull request body, supported by
    a `#print axioms` run someone did once. `#print axioms` is an `info`
    message: it prints, `lake build` exits 0, and a theorem that tomorrow
    acquires a dependency prints a different message that nobody reads. The
    claim was REPORTED, never ENFORCED, and preservation is the whole point of a
    gate.

    `#guard_msgs` turns the expected output into part of the build. The
    docstring above each command is compared to the message the command
    produces; a mismatch is an ERROR and `lake build` exits non-zero. Introduce
    a dependency under any of these and the line below it stops matching.

    (These paragraphs are reflowed so no line BEGINS with the word this file
    forbids: `scripts/check-lean-proofs.sh` rejects a declaration by anchoring
    to the start of a line, and its doctrine is to reword the prose rather than
    weaken the needle. That is the rule being followed, not a coincidence.)

    THIS IS THE HALF `scripts/check-lean-proofs.sh` CANNOT DO. That gate greps
    for the `axiom` keyword, which catches an axiom declared HERE. It cannot see
    an axiom pulled in from core — `Classical.em`, `byContradiction`,
    `Classical.choice` — because no new keyword appears in this file. Those are
    exactly the axioms a proof acquires by accident, and only this section
    catches them. Both mutations are executed by
    `scripts/check-lean-mutations.sh`.

    Every declaration this file exports is listed. There is no "audited subset":
    a subset is a place for the next theorem to land unwatched. -/
section AxiomAudit

/-- info: 'Newtui.ofEncodedView_consistent_of_injective' does not depend on any axioms -/
#guard_msgs in
#print axioms ofEncodedView_consistent_of_injective

/-- info: 'Newtui.smudge_not_consistent' does not depend on any axioms -/
#guard_msgs in
#print axioms smudge_not_consistent

/-- info: 'Newtui.ofView_consistent' does not depend on any axioms -/
#guard_msgs in
#print axioms ofView_consistent

/-- info: 'Newtui.creep_not_consistent' does not depend on any axioms -/
#guard_msgs in
#print axioms creep_not_consistent

/-- info: 'Newtui.sneak_consistent' does not depend on any axioms -/
#guard_msgs in
#print axioms sneak_consistent

/-- info: 'Newtui.sneak_not_transfers' does not depend on any axioms -/
#guard_msgs in
#print axioms sneak_not_transfers

/-- info: 'Newtui.sneak_loses_a_reachable_view' does not depend on any axioms -/
#guard_msgs in
#print axioms sneak_loses_a_reachable_view

/-- info: 'Newtui.consistency_is_not_congruence' does not depend on any axioms -/
#guard_msgs in
#print axioms consistency_is_not_congruence

-- `quotStep` is a definition, not a theorem, and it is THE deliverable.
--
-- IT IS THE ONE DECLARATION HERE THAT IS NOT AXIOM-FREE, and this line is how
-- we found that out. The pull request body claimed "no axiom dependencies at
-- all, not even `propext`" for everything in this file; the first run of this
-- section said otherwise. `Quotient.sound` is *stated* in terms of `Quot.sound`,
-- one of Lean's three kernel axioms, so anything built with `Quotient.lift`
-- carries it — it is not something a different proof would avoid.
--
-- It is recorded rather than removed, which is the difference between this and
-- the situation it replaces: the dependency is now pinned, and a SECOND axiom
-- arriving alongside it fails the build. A claim nobody checked has become a
-- number that may not move without someone editing this line.
/-- info: 'Newtui.quotStep' depends on axioms: [Quot.sound] -/
#guard_msgs in
#print axioms quotStep

end AxiomAudit

end Newtui
