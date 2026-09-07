# The exhaustive search: how it stays trustworthy

Moved out of the README so the front door stays a pointer, not an essay. This
is the rationale behind `Explorer::explore` and `Report` — what "exhaustive"
buys you, and why a property answers three ways instead of two.

Continuing the `Volume` example from the README: drop `.adjustable()` from
that row — so the renderer promises a plain value and an arrow moves it
anyway — and the same three lines say:

```text
11 states, 66 transitions, 22 terminal, 3 properties
1 violations:

only adjustable rows move
  after: Left
  `level` is not adjustable but Left changed it from `50` to `40`
```

## Exhaustive, and the counterexample is minimal for free

Two properties of the search matter more than the word *exhaustive*:

**It walks states, not paths.** Deduplicating on a fingerprint turns
`keys^depth` sequences into the handful of states a component can actually be
in. An eleven-position dial with four keys is 44 transitions, not a
combinatorial explosion.

**Breadth-first means the first path to a violation is the shortest one.** A
failure reports the minimal key sequence that reaches it, by construction —
there is no shrinking step to trust, tune, or wait for.

## Three answers, not two

`report.verdict()` is `Clean`, `Violated`, or `Incomplete { reason, .. }`. The
third is the one that matters: a search that stopped at a limit, that was
handed no property, that was handed no key, that was handed a property whose
domain it never once reached, or **whose replay did not land where discovery
said it would**, comes back `Incomplete` and says which, because
*no violations* and *nothing checked* are not the same claim and a boolean
cannot tell them apart. `violations` is not a public field, so the weak
assertion is not something a consumer can write by accident.

That last reason is why a property answers with three outcomes and not a
`Result`. `NotApplicable`, `Held`, `Violated` — because "no complaint" from a
claim about Esc means one thing over an alphabet with Esc in it and nothing at
all over one without, and `report.properties` carries the difference per
property. Explore a dial over `[Key::Right]` with a claim about Esc and the
report refuses to call it clean, naming the property that was never asked.

**The walk checks its own reconstruction.** Each state is reached by replaying
its key path into a fresh component from your factory, so the search is only
sound if that replay arrives where the search said it would. It is compared,
per reconstruction, against the fingerprint recorded at discovery — an impure
factory (a cached read, a clock, a `OnceLock` an earlier test filled) makes the
report `Incomplete` with the path and both fingerprints, instead of quietly
judging a machine the search never explored. What that establishes is
within-run consistency: a factory that is *consistently* pre-warmed is
self-consistent and passes, and `Report::divergences` documents that residual
rather than glossing it.

**The corpus is the same walk.** `report.views` is every distinct view the
search judged — including the view a component closed on, which is where
"escape left the draft alone" lives — and `report.exhausted` is what says
whether that is all of them. One walk and one completeness flag, because a
corpus handed to a reimplementation as "the states it must satisfy" would, if
quietly truncated, pass something that does less. There used to be a second
walk with a second flag; the two disagreed, and the one advertised as the
conformance artifact was the strict subset.
