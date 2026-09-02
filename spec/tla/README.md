# `spec/tla/` — the TLA+ layer

One module, `ExplorerReplay.tla`, modelling `Explorer::explore`
(`src/explore.rs:172-282`) over a **re-enterable world**. Four configurations of
it are checked green; five more are checked for the verdict they are supposed
to produce, four RED and one GREEN.

```sh
spec/tla/check.sh                # TLC-check every <Name>.tla + <Name>.cfg pair here
spec/tla/check.sh ExplorerReplay # just one; prints `tla-checked-count=N`
spec/tla/test-check.sh           # fail-closed regression tests for the runner
scripts/check-mutations.sh       # the EXPECTED-RED runner (mutations/)
scripts/check-model.sh           # the bridge: regenerate RustObs.tla and diff
```

`check.sh` and `test-check.sh` are copied from `newt-agent/spec/tla/`;
`check-lean-proofs.sh` from `precedence-ladder/scripts/`. Deliberately copied,
not reinvented.

## Why this module exists — one reason

`explore.rs:280` computes `exhausted` from `queue` and the two limits. Neither
carries any information about **reachability**. This module computes the reach
set as a fixpoint over the transition function (`ReachOf`), *with no reference
to `queue`, `seen` or `exhausted`*, and compares. A model that recomputed the
implementation's own guard would prove nothing.

Two things about the statement, both of which cost a rewrite to get right:

- **Exhaustion is coverage of the QUOTIENT, never concrete set equality.**
  `judged = Reach` secretly asserts that the fingerprint is *injective* on
  `Reach`, and turns red on any **sound** bisimulation quotient — precisely
  what the `Fingerprint` API exists to permit. The invariants use
  `Covers(A, B) == \A s \in B : \E t \in A : FpOf[t] = FpOf[s]`. Congruence, if
  it is ever modelled, is a separate explicit predicate; it is not smuggled in
  through an equality.
- **`Reach = Open ∪ Term`.** A reach set that stepped only on `Flow::Stay`
  would be blind to the closing-view population, which is exactly the
  population bug 3a destroys.

## What is checked

| Configuration | Verdict | What it says |
|---|---|---|
| `ExplorerReplay.cfg` | green | The honest world (`"free"`) at HEAD, `GuardOn = FALSE`. Carries `ModelMatchesRust` — **the bridge**. |
| `DepartureGuardCloses.cfg` | green | `{"free","drain"}` with the S0 departure guard ON. Identical to `mutations/ReplayGuardOff.cfg` except for `GuardOn`, and that one is RED. The pair is the argument that the guard is load-bearing. |
| `NarrowingHolds.cfg` | green | `ExhaustedOnlyNarrows`, in its own cfg. Bug 1 is fixed and stays fixed. |
| `PrewarmedConsistent.cfg` | green | The residue, green half: a pre-warmed run is the *sound and complete* report of a real machine. |
| `mutations/ReplayGuardOff.cfg` | **RED** on `CoversColdReach` | **Bug 3b, live at `7f69a3d`.** The counterexample is below. |
| `mutations/NarrowOff.cfg` | **RED** on `ExhaustedOnlyNarrows` | Bug 1's regression gate (`=` in place of `&=` at `:280`). |
| `mutations/PrewarmedResidue.cfg` | **RED** on `CoversColdReach` | The residue, red half. Permanent by design. |
| `mutations/HonestRunExists.cfg` | **RED** on `ClaimSurvives` | The anti-vacuity probe: an honest exhaustive run IS reachable, so the greens above are not holding on an unreachable antecedent. |
| `mutations/DecoyOverlap.cfg` | **GREEN** | A negative control. `ReplayGuardOff` with the two machines overlapped — the 3b detector goes blind. |

## The live catch, and its counterexample

`mutations/ReplayGuardOff.cfg` — world `"drain"`, where the first `factory()`
call returns the warm machine and every later one the cold machine. This is
`newt-tui/src/type_ahead.rs`'s `static BUFFER: OnceLock<Mutex<Vec<u8>>>`, whose
`take()` **drains**, consumed inside component construction.

```
Error: Invariant CoversColdReach is violated.
State 1: <Initial predicate>
  queue = <<<<<<>>, "w0">>>>   judged = {"w0"}   exhausted = TRUE
...
State 14: <Terminate>
  done      = TRUE
  world     = "drain"
  maxDepth  = 3
  queue     = <<>>
  judged    = {"w0", "p1", "p2", "bye"}
  exhausted = TRUE
  nStates = 3   nTrans = 6   nTerm = 3
```

`judged` contains `w0`, which is not reachable from the cold start, and is
missing `p0`, which is the cold start. It is **neither machine's reach set**,
and the run is stamped `exhausted = TRUE`. That is the bug: a report describing
neither the cold nor the warm component, presented as exhaustive.

The design change it yields is in the model's data structure — `queue` carries
`<<path, node RECORDED AT DISCOVERY>>`, the witness `explore.rs:273` throws
away. `GuardOn = TRUE` compares it after replay and clears `exhausted` on
mismatch, and `DepartureGuardCloses.cfg` is that configuration going green.

## The residue, and why it is committed red

`PrewarmedConsistent.cfg` (green on `ReplayIsConsistent`) and
`mutations/PrewarmedResidue.cfg` (red on `CoversColdReach`) are the **same
configuration**, both with `GuardOn = TRUE`. The departure guard does not help:
discovery and replay agree because *both are the wrong machine*.

So the two invariants are not a redundant pair — they split the case, and the
split is the answer to "can the environment hazard be closed structurally?"
**No.** The run is internally honest about a component the consumer will never
construct, and nothing in the crate names which machine was tested. That turns
"the guards close about two thirds" into a countable excluded-site set that may
only shrink.

## Anti-vacuity decoys — do not "tidy" these

Delete any one and a mutation goes invisible.

1. **`"bye"` is entered ONLY by a closing transition.** It is what lets `seen`
   and `judged` be different sets without the difference being unobservable.
2. **`"w0"` is unreachable from `"p0"`.** Overlap the two machines and the 3b
   mutation goes green — and that is not a claim, it is
   `mutations/DecoyOverlap.cfg`, executed by the runner.
3. **`DepthCaps = {1, 3}`.** At cap 1 the depth arm fires; at cap 3 an honest
   exhaustive run exists. One cap alone makes one of the two unreachable.

## Model fidelity — what ties this to `src/`, and what does not

A hand-written TLA+ module is a model of `explore.rs` that nothing forces to
agree with it. The empirical reason to care: **bug 3a landed one commit after
the fix that created the `exhausted` flag**, in a crate whose author was
actively thinking about that flag. "We will update the spec when the code
changes" is not a hypothesis here; it is a bet this repository has already lost.

Two mechanisms, both executed:

**(a) `scripts/check-mutations.sh` — the expected-red runner.** Every invariant
here has a mutation that turns it red, and the runner asserts BOTH a non-zero
exit AND that TLC named *that* invariant. A red for the wrong reason is a green
in disguise. It also has a floor on the number of configurations it read,
because an absence check fails open.

It exists as a separate script because `check.sh` `die`s on the first TLC
failure **inside** its discovery loop: one red spec aborts the run and every
spec after it is never checked. A permanently-red residue configuration cannot
enter CI through it at all.

**(b) `scripts/check-model.sh` — the bridge.** `examples/gen_model.rs`
implements, in Rust, exactly the `Step`/`Closes` function this module
hard-codes, runs the **real** `Explorer::explore` over it at each depth cap,
and emits four report counters as the generated `RustObs.tla`. The model
computes those same four numbers *itself*; `ModelMatchesRust` compares them.

Regeneration rewrites only the Rust side, so a drifted model stays red after
regeneration — it cannot heal into "somebody edited the number to match", which
is what a golden vector does when the model is a hand-written re-implementation.
Both halves are verified: re-introducing bug 1 in `explore.rs` (`=` for `&=` at
`:280`) makes `check-model.sh` red on the diff, and regenerating past that makes
TLC red on `ModelMatchesRust`.

**Its honest limits**, so nobody oversells it:

- it binds **four scalars per cap in one world mode** (`"free"`), not a
  refinement. `"drain"` becomes bindable when the departure guard exists in the
  crate, because the model with `GuardOn = TRUE` describes post-fix code;
- it binds `Explorer::explore` only. `Explorer::states` — where bug 3a lives —
  is a second hand-written walk, deliberately unmodelled because it is being
  deleted;
- a model change that is wrong in a way the four counters cannot see is
  invisible to it. What compensates is (i) the mutation runner, which is
  mechanical, and (ii) this module being small enough — one component, one arm
  — that a reader can hold it against `explore.rs:172-282` in one sitting. That
  is a real answer, not a good one.

## Four members of the false-completeness class living in this tooling

Said out loud rather than discovered later.

1. **`lake build` exits 0 on an unproven theorem.** Gated by
   `scripts/check-lean-proofs.sh`; the transcript is in `formal/README.md`.
2. **A `[[lean_lib]]` missing from `defaultTargets` compiles nothing**, and CI
   goes green having built an empty tree. `formal/lakefile.toml` says so at the
   top and names line 2.
3. **A `.tla` with no matching `.cfg` is silently skipped by discovery** — add
   a model, forget its `.cfg`, and CI stays green having never checked it.
   Recorded as a passing test in `test-check.sh` (cases 8 and 9) rather than
   fixed, because `check.sh` is copied verbatim from `newt-agent` and forking it
   is the more expensive mistake. The count it prints is the only thing that
   can betray the skip.
4. **`check.sh` aborts on the first red**, so a red spec hides every spec after
   it. Worked around by `mutations/` living outside discovery and being run one
   at a time.

## Deliberately absent

- **`max_states`** — no bug lives there, and modelling it costs a per-key
  cursor and a second terminal. Its residuals (the cap overshoots by
  `|alphabet| - 1`; the initial state is exempt at `:195`) are off-by-N facts a
  four-line Rust test settles.
- **The second walk (`Explorer::states`)** — it is being deleted. Specifying
  code that should not exist removes bug 3a until someone edits the copy.
- **Liveness and fairness** — the walk terminates by construction, so every
  liveness property would report a counterexample that says nothing about the
  code.
- **A lossy `FpOf`** — two defects in one model means neither mutation is
  diagnostic. `FpOf` is the identity, and the invariants are written in the
  quotient form anyway.
- **"Shape B"** — a divergence appearing deeper inside a replay whose prefix
  replays identically. Worth having; about fifteen lines; not worth blocking
  this on. Until it exists, read the table above as a refutation record, never
  as a coverage claim.

## Pinned toolchain

| tool | version | pin |
|---|---|---|
| `tla2tools` (TLC / SANY) | **1.7.4** (TLC 2.19) | sha256 `936a262061c914694dfd669a543be24573c45d5aa0ff20a8b96b23d01e050e88` |

`check.sh` is fail-closed: it resolves the jar in order — `$TLA2TOOLS_JAR` (a
wrong version is a hard error, never run) → `~/opt/tla2tools/` → cache → an
atomic, checksum-verified download. Bump the version and the checksum in
lock-step.
