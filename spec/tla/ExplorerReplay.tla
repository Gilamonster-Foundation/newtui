-------------------------- MODULE ExplorerReplay --------------------------
(***************************************************************************)
(* A model of `Explorer::explore` (src/explore.rs:172-282) over a           *)
(* RE-ENTERABLE world.                                                     *)
(*                                                                         *)
(* READ THIS BEFORE CITING THIS WORK.                                      *)
(* ---------------------------------                                       *)
(* Nothing here reads the Rust except `ModelMatchesRust`, which binds five  *)
(* report facts in TWO world modes (see "THE BRIDGE" below). Every other    *)
(* invariant is a theorem about THIS FILE. Citing `CoversColdReach` as "we  *)
(* proved the explorer is honest" is worse than no proof at all: the model  *)
(* is hand-written and nothing but the bridge forces it to agree with       *)
(* `explore.rs`. Bug 3a landed one commit after the fix that created the    *)
(* `exhausted` flag, in a crate whose author was actively thinking about    *)
(* that flag — "someone will re-run the table" has already been lost once   *)
(* in this repository.                                                      *)
(*                                                                         *)
(* WHY THIS MODULE EXISTS — one reason.                                     *)
(* -----------------------------------                                      *)
(* `explore.rs:280` computes `exhausted` from `queue` and the two limits.    *)
(* Neither carries any information about REACHABILITY. This module computes  *)
(* the reach set as a fixpoint over the transition function (`ReachOf`) —    *)
(* with no reference to `queue`, `seen` or `exhausted` — and compares. A     *)
(* model that recomputed the implementation's own guard would prove          *)
(* nothing.                                                                  *)
(*                                                                          *)
(* THE MODELLED WORLD is drain-on-first-take ONLY (`WorldModes`). A         *)
(* divergence that appears deeper inside a replay while the prefix replays  *)
(* identically ("shape B") is NOT modelled. Read the mutation table as a    *)
(* refutation record, never as a coverage claim.                            *)
(*                                                                          *)
(* DELIBERATELY ABSENT, each for a reason:                                  *)
(*  * `max_states` — no bug lives there; its residuals are off-by-N facts a *)
(*    four-line Rust test settles, and modelling it costs a second terminal.*)
(*  * `Explorer::states` (the second walk) — DELETED from the crate, so      *)
(*    there is no longer anything to specify. Bug 3a was its bug and it     *)
(*    went with it; `Report.views` from the one walk is what replaced it.   *)
(*  * liveness and fairness — the walk terminates by construction, so every *)
(*    liveness property would report a counterexample that says nothing     *)
(*    about the code.                                                       *)
(*  * a LOSSY `FpOf` — two defects in one model means neither mutation is   *)
(*    diagnostic. `FpOf` is the identity here ON PURPOSE; the invariants    *)
(*    are nonetheless written in the QUOTIENT form (see `Covers`).          *)
(***************************************************************************)
EXTENDS Naturals, Sequences, FiniteSets, RustObs

CONSTANTS
    \* @type: Set(Int);
    DepthCaps,      \* {1, 3}: 1 so the depth arm fires, 3 so an honest run exists.
    \* @type: Set(Str);
    WorldModes,     \* subset of {"free", "drain", "prewarmed"}
    \* @type: Bool;
    GuardOn,        \* the departure guard: does replay land where discovery said?
                    \* TRUE describes main; FALSE is the historical counterexample.
    \* @type: Bool;
    Narrow,         \* TRUE models `&=` at :280; FALSE models the pre-5307cd7 `=`.
    \* @type: Str;
    WarmStart       \* the decoy start node. Setting it to ColdStart REMOVES the decoy.

\* ---------------------------------------------------------------------------
\* THE COMPONENT. Hard-coded: a .cfg cannot carry a function literal.
\* ---------------------------------------------------------------------------
\* @type: Set(Str);
Keys  == {"right", "esc"}
\* @type: Set(Str);
Nodes == {"p0", "p1", "p2", "bye", "w0"}
\* @type: Str;
ColdStart == "p0"

ASSUME WarmStart \in Nodes
ASSUME DepthCaps \subseteq ObsCaps   \* the bridge's domain; see THE BRIDGE.

\* Three ANTI-VACUITY DECOYS live in this definition. Delete any one and a
\* mutation below goes invisible:
\*  (1) "bye" is entered ONLY by a closing transition, so a model that
\*      conflated `seen` with `judged` would still pass;
\*  (2) "w0" is unreachable from "p0" — overlap the two machines and the 3b
\*      mutation goes green (mutations/DecoyOverlap.cfg is that negative
\*      control, and it is EXECUTED, not asserted in prose);
\*  (3) DepthCaps = {1,3} — at a single cap the depth arm either never fires
\*      or never lets an honest run exist.
Step == [n \in Nodes |-> [k \in Keys |->
            IF k = "esc" THEN "bye"
            ELSE IF n = "p0" THEN "p1"
            ELSE IF n = "p1" THEN "p2"
            ELSE n]]

Closes == { <<n, "esc">> : n \in Nodes }

\* The fingerprint, as a function on states. Identity here (see the header).
FpOf == [n \in Nodes |-> n]

\* ---------------------------------------------------------------------------
\* THE ORACLE. A fixpoint over the component, evaluated with NO reference to
\* queue, seen, exhausted or any other search variable. This is the only reason
\* the module is not a restatement of its own guard.
\*
\* `Reach = Open \cup Term`. A reach set that stepped only on `Flow::Stay`
\* would be silently blind to the closing-view population, which is exactly
\* the population bug 3a destroys.
\* ---------------------------------------------------------------------------
Succ(S)    == S \cup UNION { {Step[n][k] : k \in Keys} : n \in S }
ReachOf(s) == LET f[i \in 0..Cardinality(Nodes)] ==
                  IF i = 0 THEN {s} ELSE Succ(f[i - 1])
              IN f[Cardinality(Nodes)]

Machines == {ColdStart, WarmStart}

\* Coverage of the QUOTIENT, never concrete set equality. `A = B` secretly
\* asserts that FpOf is INJECTIVE on B, and turns red on any SOUND
\* bisimulation quotient — precisely what the `Fingerprint` API exists to
\* permit. Congruence, if it is ever modelled, is a separate explicit
\* predicate over Nodes \X Nodes; it is not smuggled in here.
Covers(A, B) == \A s \in B : \E t \in A : FpOf[t] = FpOf[s]

RECURSIVE Walk(_, _)
Walk(n, path) == IF path = <<>> THEN n ELSE Walk(Step[n][Head(path)], Tail(path))

\* ---------------------------------------------------------------------------
\* THE SEARCH.
\* ---------------------------------------------------------------------------
VARIABLES
    \* @type: Seq(<<Seq(Str), Str>>);
    queue,      \* <<path, node RECORDED AT DISCOVERY>>. The witness explore.rs
                \* used to throw away. This data structure is `Departure`.
    \* @type: <<Seq(Str), Str>>;
    cur,        \* the dequeued item being expanded, or NoItem.
    \* @type: Set(Str);
    pending,    \* keys of the alphabet not yet applied to `cur` (:217).
    \* @type: Set(Str);
    judged,     \* every post-state handed to a property (:196, :254, :268) —
                \* CLOSING POST-STATES INCLUDED.
    \* @type: Set(Str);
    seen,       \* the dedupe set (:194, :264) — closing post-states EXCLUDED.
    \* @type: Bool;
    exhausted,  \* :182 starts TRUE; :214 and :280 are the only writes.
    \* @type: Int;
    nStates,    \* report.states
    \* @type: Int;
    nTrans,     \* report.transitions
    \* @type: Int;
    nTerm,      \* report.terminal_states
    \* @type: Int;
    maxDepth,   \* chosen once from DepthCaps, then constant
    \* @type: Str;
    world,      \* chosen once from WorldModes, then constant
    \* @type: Bool;
    diverged,   \* report.divergences() is non-empty
    \* @type: Bool;
    done

vars == << queue, cur, pending, judged, seen, exhausted,
           nStates, nTrans, nTerm, maxDepth, world, diverged, done >>

NoItem == <<>>   \* distinguishable from a real 2-tuple item

\* What `factory()` returns. "drain": the FIRST call yields the warm node and
\* every later one the cold node (newt-tui's `static BUFFER: OnceLock<Mutex<..>>`
\* whose `take()` drains). "prewarmed": something else in the process already
\* drained it, so EVERY call in this run yields the warm node.
InitNode(w)   == IF w \in {"drain", "prewarmed"} THEN WarmStart ELSE ColdStart
ReplayNode(w) == IF w = "prewarmed"              THEN WarmStart ELSE ColdStart

\* Where :225-228 ACTUALLY lands, as opposed to where discovery recorded.
Landed(item) == Walk(ReplayNode(world), item[1])

Init ==
    /\ \E d \in DepthCaps : maxDepth = d
    /\ \E w \in WorldModes :
         /\ world = w
         /\ queue  = << << <<>>, InitNode(w) >> >>
         /\ judged = {InitNode(w)}          \* :196 checks the initial state
         /\ seen   = {InitNode(w)}          \* :194
    /\ cur       = NoItem
    /\ pending   = {}
    /\ exhausted = TRUE                     \* :182 — starts TRUE, only cleared
    /\ nStates   = 1                        \* :195
    /\ nTrans    = 0
    /\ nTerm     = 0
    /\ diverged  = FALSE
    /\ done      = FALSE

\* :205-216. The departure guard is evaluated once per dequeued path rather
\* than once per key: `explore.rs:225-228` replays the SAME path for every key
\* of the alphabet, so in a deterministic world all |alphabet| replays land in
\* the same place and one check is equivalent to |alphabet| of them.
Take ==
    /\ ~done
    /\ cur = NoItem
    /\ queue # <<>>
    /\ LET item == Head(queue) IN
       /\ queue' = Tail(queue)
       /\ IF Len(item[1]) >= maxDepth
          THEN /\ cur'       = NoItem
               /\ pending'   = {}
               /\ exhausted' = FALSE                          \* :214
               /\ diverged'  = diverged
          \* THE DEPARTURE GUARD, as `Explorer::replay_to` now implements it.
          \* Two halves, and both are load-bearing:
          \*   * the divergence is RECORDED — `report.divergences` becomes
          \*     non-empty, which is what makes the verdict `Incomplete`;
          \*   * and NOTHING is judged from here. The crate `break`s out of the
          \*     key loop, so no outgoing key is applied to a machine the
          \*     search never explored. `cur' = NoItem` is that break.
          \* Note what does NOT happen: `exhausted` is untouched. In the crate
          \* it still means "ran out of frontier rather than hitting a limit",
          \* and a diverged run can be both exhausted and worthless. The
          \* invariants below therefore ask for `~diverged` as well.
          ELSE IF GuardOn /\ Landed(item) # item[2]
          THEN /\ cur'       = NoItem
               /\ pending'   = {}
               /\ exhausted' = exhausted
               /\ diverged'  = TRUE
          \* With the guard OFF this is the crate BEFORE the guard landed: the
          \* replay went somewhere else, nothing noticed, and every key below
          \* was applied to that other machine. Kept as the counterexample
          \* mutations/ReplayGuardOff.cfg executes, not as live behaviour.
          ELSE /\ cur'       = item
               /\ pending'   = Keys
               /\ exhausted' = exhausted
               /\ diverged'  = diverged
    /\ UNCHANGED << judged, seen, nStates, nTrans, nTerm, maxDepth, world, done >>

\* :217-275. One key. `judged` grows for EVERY post-state, closing or not;
\* `seen` and `queue` grow only for non-closing ones.
ApplyKey ==
    /\ ~done
    /\ cur # NoItem
    /\ \E k \in pending :
         LET from    == Landed(cur)
             to      == Step[from][k]
             closing == <<from, k>> \in Closes
         IN /\ pending' = pending \ {k}
            /\ nTrans'  = nTrans + 1
            /\ judged'  = judged \cup {to}                    \* :254 and :268
            /\ IF closing
               THEN /\ nTerm' = nTerm + 1                     \* :253
                    /\ UNCHANGED << seen, nStates, queue >>   \* :261 `continue`
               ELSE /\ nTerm' = nTerm
                    \* :264 — membership is by FINGERPRINT, not by state.
                    /\ IF FpOf[to] \notin { FpOf[x] : x \in seen }
                       THEN /\ seen'    = seen \cup {to}
                            /\ nStates' = nStates + 1         \* :265
                            /\ queue'   = Append(queue,
                                            << Append(cur[1], k), to >>)
                       ELSE UNCHANGED << seen, nStates, queue >>
    /\ UNCHANGED << cur, exhausted, maxDepth, world, diverged, done >>

Finish ==
    /\ ~done
    /\ cur # NoItem
    /\ pending = {}
    /\ cur' = NoItem
    /\ UNCHANGED << queue, pending, judged, seen, exhausted,
                    nStates, nTrans, nTerm, maxDepth, world, diverged, done >>

\* :280. `&=` narrows and never restores; `Narrow = FALSE` is the pre-5307cd7
\* `=`, which erased a depth truncation recorded at :214.
Terminate ==
    /\ ~done
    /\ cur = NoItem
    /\ queue = <<>>
    /\ done' = TRUE
    /\ exhausted' = IF Narrow THEN exhausted ELSE TRUE
    /\ UNCHANGED << queue, cur, pending, judged, seen,
                    nStates, nTrans, nTerm, maxDepth, world, diverged >>

Stutter == done /\ UNCHANGED vars

Next == Take \/ ApplyKey \/ Finish \/ Terminate \/ Stutter

Spec == Init /\ [][Next]_vars      \* safety only; see "liveness" in the header

\* ---------------------------------------------------------------------------
\* INVARIANTS
\* ---------------------------------------------------------------------------

\* What the CONSUMER cares about: a report stamped exhaustive must have
\* covered the reach set of the machine a fresh process would build.
CoversColdReach ==
    (done /\ exhausted /\ ~diverged) => Covers(judged, ReachOf(ColdStart))

\* Whether the run was the honest report of SOME machine. The pair splits the
\* case (it is not a redundant second guard, which would give the same answer
\* and mask a mutation): under "prewarmed" this is GREEN and
\* `CoversColdReach` is RED, and THAT is the residue statement — the run is
\* internally honest about a component the consumer will never construct, and
\* nothing in the crate names which machine was tested.
ReplayIsConsistent ==
    (done /\ exhausted /\ ~diverged) =>
        \E m \in Machines : Covers(judged, ReachOf(m)) /\ Covers(ReachOf(m), judged)

\* Bug 1's shape as a two-state relation: the flag may go TRUE -> FALSE and
\* never back. Ships in its OWN cfg — beside `CoversColdReach` the stronger
\* invariant fires first and masks the bug-1 mutation.
ExhaustedOnlyNarrows == [][exhausted' => exhausted]_vars

\* ---------------------------------------------------------------------------
\* THE BRIDGE (model fidelity). `ObsStates`/`ObsTransitions`/`ObsTerminal`/
\* `ObsExhausted` come from RustObs.tla, which is GENERATED by
\* `cargo run --example gen_model` from a real `Explorer::explore` run over a
\* Rust component implementing exactly `Step`/`Closes` above.
\*
\* This binds the model's OWN computed counters against the Rust's RECORDED
\* report, so regeneration rewrites only the Rust side: a drifted model stays
\* RED after regeneration and cannot auto-heal the way a golden vector does.
\*
\* IT NOW BINDS TWO WORLDS. "drain" became bindable when the departure guard
\* landed in the crate: `gen_model` runs the SAME `Explorer::explore` over a
\* factory whose first product is the warm node and whose every later product
\* is the cold one, which is what `InitNode`/`ReplayNode` say "drain" is. That
\* run is what pins the guard's observable behaviour — one divergence, nothing
\* judged past it, and `report.exhausted` still TRUE — rather than leaving the
\* guard-on model a description of code nobody ran.
\*
\* LIMITS, still: five observations per cap per world, not a refinement.
\* "prewarmed" is deliberately NOT bound — see PrewarmedConsistent.cfg, whose
\* whole point is that it is indistinguishable from "free" from inside, so a
\* binding would add no information and would suggest one had been gained.
\* ---------------------------------------------------------------------------
ModelMatchesRust ==
    (done /\ world \in {"free", "drain"}) =>
        /\ nStates   = ObsStates[world][maxDepth]
        /\ nTrans    = ObsTransitions[world][maxDepth]
        /\ nTerm     = ObsTerminal[world][maxDepth]
        /\ exhausted = ObsExhausted[world][maxDepth]
        /\ diverged  = ObsDiverged[world][maxDepth]

\* ---------------------------------------------------------------------------
\* MUST-GO-RED PROBE. Deliberately kept out of every shipped cfg and executed
\* by scripts/check-mutations.sh: if this ever holds, no honest exhaustive run
\* is reachable and every green above is vacuous.
\* ---------------------------------------------------------------------------
ClaimSurvives == ~(done /\ exhausted /\ Covers(judged, ReachOf(ColdStart)))
=============================================================================
