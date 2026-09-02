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
   `Consistent` and nothing else. `ofView_consistent` says that check CANNOT
   FIRE on a component that took the default fingerprint. `sneak` says the
   half it cannot see (`Transfers`) is where the exposure lives. `creep` says
   the guard is nonetheless load-bearing on a hand-rolled `Fingerprint::of`,
   which is the population where the measured false green was found. Keep the
   guard; bill it accurately.
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

/-- `Fingerprint::of_view` — newtui's DEFAULT (`component.rs:112`). The
    fingerprint IS the view. -/
def ofView (step : S → K → S) (view : S → V) : Comp S K V V :=
  { step := step, fp := view, view := view }

/-- **The one-line justification for this whole file.** Under the default
    fingerprint the observation half is definitional, so a guard that records
    the `View` beside the `Fingerprint` and asserts on a collision is INERT.
    Checked against `component.rs:48-63`: `of_view` serialises the title, the
    footer and every field of every row, so fingerprint equality is view
    equality by construction. -/
theorem ofView_consistent (step : S → K → S) (view : S → V) :
    Consistent (ofView step view) := fun _ _ h => h

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

end Newtui
