//! **Terminal UI components you can drive in isolation.**
//!
//! Two families:
//!
//! - **Components** are interactive — a state machine over keys. A settings
//!   panel, a chooser, a form, a pager.
//! - **Widgets** are display — a pure function from data to styled text. A
//!   sparkline, a butterfly meter, a heat bar, a gauge.
//!
//! # The problem this exists for
//!
//! Interactive terminal code is usually tested the way it is written: end to
//! end, through a real terminal, one scripted path at a time. That catches the
//! path someone thought of. It does not catch the fourth Esc, the arrow held
//! down past the end of a list, or the row that scrolled out from under a
//! selection index.
//!
//! Here a component declares three things — how it handles one key, what it
//! currently shows, and what makes two of its states the same — and
//! [`Explorer`] walks EVERY reachable state, applying every key from each one,
//! checking the acceptance [`Property`] set at each state and each transition.
//!
//! # What this crate refuses to do
//!
//! It decodes no keys, owns no terminal, renders nothing by default, and
//! performs no effect. A component describes; a host draws and acts. That line
//! is what keeps the dependency closure empty at `--no-default-features`
//! (`tests/leaf.rs` asserts it), which is in turn what lets one component suite
//! be shared by several harnesses — including headless ones with no terminal
//! at all.

#![forbid(unsafe_code)]
#![warn(missing_docs, clippy::pedantic)]
// The explorer's counters are usize and its reports are prose; a cast lint on
// arithmetic that cannot overflow a report would be noise.
#![allow(clippy::module_name_repetitions)]

mod component;
pub mod components;
mod explore;
mod key;
mod property;
mod view;
mod widget;

pub use component::{Component, Fingerprint, Flow};
pub use explore::{
    Divergence, DivergenceReason, Explorer, PropertyCoverage, Report, Verdict, Violation,
};
pub use key::Key;
pub use property::{properties, Named, Observation, Property, PropertyOutcome};
pub use view::{Row, View};
#[cfg(feature = "ratatui")]
pub use widget::ratatui_lines;
pub use widget::{
    bar, butterfly, core_grid, gauge, heat_meter, sparkline, CoreSeries, Run, SparkDirection, Tone,
    WidgetLine, WidgetOutput, WidgetOutputError,
};

/// The README's example is compiled and RUN, not read and believed.
///
/// It is the only worked example the crate publishes, so it is the idiom every
/// consumer copies as their first acceptance test — and it shipped both
/// uncompilable (its `use` was missing two names) and, once fixed, failing (an
/// `.adjustable()` row with no `.selected()`). An example nobody compiles is
/// the same class of claim as a search that reports itself exhaustive.
///
/// `cfg(doctest)` rather than `#![doc = include_str!(..)]`: the crate's own
/// front-page docs are the module comment above, not the README.
// GUARD: TheReadmeIsCompiledAndRun — this is a guard; tests/mutations.rs must show it red.
#[cfg(doctest)]
#[doc = include_str!("../README.md")]
struct TheReadmeIsCompiledAndRun;

/// Every catalogue example is compiled and RUN without replacing the crate's
/// own front-page documentation with an inventory that spans namespaces.
// GUARD: TheCatalogIsCompiledAndRun — this is a guard; tests/mutations.rs must show it red.
#[cfg(doctest)]
#[doc = include_str!("../docs/CATALOG.md")]
struct TheCatalogIsCompiledAndRun;

#[cfg(test)]
mod tests {
    use super::*;

    /// A dial that CLAMPS — the shape most rows in this line have.
    struct Dial {
        level: u8,
        done: Option<bool>,
    }

    impl Component for Dial {
        fn handle(&mut self, key: Key) -> Flow {
            match key {
                Key::Left => {
                    self.level = self.level.saturating_sub(1);
                    Flow::Stay
                }
                Key::Right => {
                    self.level = (self.level + 1).min(3);
                    Flow::Stay
                }
                Key::Enter => {
                    self.done = Some(true);
                    Flow::Close(true)
                }
                Key::Esc => {
                    self.done = Some(false);
                    Flow::Close(false)
                }
                _ => Flow::Stay,
            }
        }

        fn view(&self) -> View {
            View::titled("dial")
                .row(
                    Row::new("level", self.level.to_string())
                        .adjustable()
                        .selected(),
                )
                .footer(match self.done {
                    None => "←→ change · Enter apply · Esc cancel",
                    Some(true) => "saved",
                    Some(false) => "discarded",
                })
        }
    }

    fn dial() -> Dial {
        Dial {
            level: 0,
            done: None,
        }
    }

    fn acceptance() -> Vec<Box<dyn Property>> {
        vec![
            Box::new(properties::selection_is_always_in_range()),
            Box::new(properties::escape_always_closes_without_applying()),
            Box::new(properties::only_adjustable_rows_move()),
        ]
    }

    fn check<C: Component>(factory: impl Fn() -> C) -> Report {
        let owned = acceptance();
        let refs: Vec<&dyn Property> = owned.iter().map(AsRef::as_ref).collect();
        Explorer::new(Key::navigation()).explore(factory, &refs)
    }

    /// **The search is exhaustive and small.** Four levels, six keys — the
    /// reachable set is four states, not `6^depth` paths. This is the claim the
    /// whole design rests on, so it is asserted with a number.
    #[test]
    fn a_clamping_dial_has_exactly_its_positions_as_states() {
        let report = check(dial);
        assert_eq!(report.states, 4, "levels 0..=3 and nothing else: {report}");
        assert!(report.exhausted, "the search finished: {report}");
        assert!(report.is_clean(), "{report}");
    }

    /// **A door that dials is caught, with the shortest path to it.**
    ///
    /// The defect: a row rendered as not-adjustable that moves under an arrow
    /// anyway. An operator passing over it repoints something they were not
    /// editing.
    #[test]
    fn a_non_adjustable_row_that_moves_is_caught_minimally() {
        struct Door {
            walked: u8,
        }
        impl Component for Door {
            fn handle(&mut self, key: Key) -> Flow {
                match key {
                    // The bug: a door that responds to →.
                    Key::Right => {
                        self.walked += 1;
                        Flow::Stay
                    }
                    Key::Esc => Flow::Close(false),
                    _ => Flow::Stay,
                }
            }
            fn view(&self) -> View {
                View::titled("door").row(
                    // Rendered as a door: no dial chrome promised.
                    Row::new(
                        "backend",
                        format!("sol{}", "!".repeat(self.walked as usize)),
                    )
                    .selected(),
                )
            }
        }

        let report = check(|| Door { walked: 0 });
        let found = report.violations_named("only adjustable rows move");
        let [violation] = found.as_slice() else {
            panic!("the door's dialling must be caught, exactly once: {report}")
        };
        assert_eq!(
            violation.path,
            vec![Key::Right],
            "and the path to it is minimal: {violation}"
        );
    }

    /// **A state with no way out is caught**, which is the property that
    /// cannot be written as a single-path test: it is a claim about EVERY
    /// reachable state.
    #[test]
    fn a_swallowed_escape_is_caught() {
        struct Trap {
            deep: bool,
        }
        impl Component for Trap {
            fn handle(&mut self, key: Key) -> Flow {
                match key {
                    Key::Down => {
                        self.deep = true;
                        Flow::Stay
                    }
                    // The bug: Esc works at the top and is swallowed below.
                    Key::Esc if !self.deep => Flow::Close(false),
                    _ => Flow::Stay,
                }
            }
            fn view(&self) -> View {
                View::titled("trap")
                    .row(Row::new("where", if self.deep { "deep" } else { "top" }).selected())
            }
        }

        let report = check(|| Trap { deep: false });
        let found = report.violations_named("escape always closes without applying");
        let [violation] = found.as_slice() else {
            panic!("the trap must be caught, exactly once: {report}")
        };
        assert_eq!(violation.path, vec![Key::Down, Key::Esc], "{violation}");
    }

    /// A capped search says so, and `is_clean` refuses to call it a proof.
    /// "No violations in the part I looked at" is not "no violations".
    // GUARD: tests::a_capped_search_is_a_sample_and_admits_it — this is a guard; tests/mutations.rs must show it red.
    #[test]
    fn a_capped_search_is_a_sample_and_admits_it() {
        struct Endless {
            n: u32,
        }
        impl Component for Endless {
            fn handle(&mut self, key: Key) -> Flow {
                if key == Key::Right {
                    self.n += 1;
                }
                Flow::Stay
            }
            fn view(&self) -> View {
                View::titled("endless").row(Row::new("n", self.n.to_string()).selected())
            }
        }

        // A real property, so the CAP is the only thing that can make this
        // report incomplete — a test that would go red for two reasons cannot
        // say which one it is pinning.
        let in_range = properties::selection_is_always_in_range();
        let report = Explorer::new([Key::Right])
            .max_states(10)
            .explore(|| Endless { n: 0 }, &[&in_range]);
        assert!(!report.exhausted, "an unbounded dial cannot be exhausted");
        assert!(!report.is_clean(), "a sample is not a clean bill of health");
        assert!(
            report.to_string().contains("STOPPED AT A LIMIT"),
            "and the report says so: {report}"
        );
    }

    /// **A DEPTH-capped search is a sample too**, and says so.
    ///
    /// The regression this pins was live in the first commit: the depth arm
    /// set `exhausted = false`, and the assignment at the end of the search
    /// then overwrote it unconditionally. Since the loop only exits with an
    /// empty queue, that assignment was always `true` — so a truncated run
    /// reported itself exhaustive and `is_clean()` agreed. A 500-position dial
    /// came back as "65 states, exhausted", having seen 13% of them.
    ///
    /// Found by an adversarial critique of the work-package plan, before a
    /// package author could hit it: package A's acceptance is this crate's
    /// first real `exhausted: true`, and an implementer raising a limit to get
    /// there would have been handed a false green.
    ///
    /// The `max_states` arm was already covered. This is the other one, which
    /// is the lesson: two limits, and the test only knew about one.
    // GUARD: tests::a_depth_capped_search_is_a_sample_and_admits_it — this is a guard; tests/mutations.rs must show it red.
    #[test]
    fn a_depth_capped_search_is_a_sample_and_admits_it() {
        struct LongDial {
            n: u32,
        }
        impl Component for LongDial {
            fn handle(&mut self, key: Key) -> Flow {
                if key == Key::Right && self.n < 500 {
                    self.n += 1;
                }
                Flow::Stay
            }
            fn view(&self) -> View {
                View::titled("dial").row(Row::new("n", self.n.to_string()).selected())
            }
        }

        // The dial is finite and the state count is far under `max_states`, so
        // depth is the ONLY limit that can bind here.
        let in_range = properties::selection_is_always_in_range();
        let report = Explorer::new([Key::Right]).explore(|| LongDial { n: 0 }, &[&in_range]);
        assert!(
            report.states <= 65,
            "the default depth cap truncates this walk: {report}"
        );
        assert!(!report.exhausted, "and the report must say so: {report}");
        assert!(!report.is_clean(), "a truncated walk is not a clean bill");
        assert!(
            report.to_string().contains("STOPPED AT A LIMIT"),
            "in words, not just a flag: {report}"
        );

        // Given depth enough, the same dial IS exhausted — so the flag tracks
        // the search rather than being pessimistic about everything.
        let full = Explorer::new([Key::Right])
            .max_depth(600)
            .explore(|| LongDial { n: 0 }, &[&in_range]);
        assert_eq!(full.states, 501, "0..=500: {full}");
        assert!(full.exhausted && full.is_clean(), "{full}");
    }

    /// **A state the fingerprint merges is a state the walk DROPS**, and the
    /// report still says `exhausted: true`.
    ///
    /// This is why the fingerprint's identity had to become structural rather
    /// than get better separators. `seen.insert(component.fingerprint())` is
    /// the gate: when two distinct states collide there, the second is never
    /// recorded and never enqueued, so everything reachable only through it is
    /// unreachable to the search — while the queue empties normally and every
    /// honesty mechanism in the crate reports a finished, complete walk. The
    /// false-completeness class, in the DEFAULT path, with no caller mistake.
    ///
    /// Four separators, because the defect was never a particular character:
    /// each pair below collides under some flattening of the structure into one
    /// string, and the empty one collides under all of them. A structural
    /// identity has no flattening to collide in.
    // GUARD: tests::a_state_the_fingerprint_merges_is_a_state_the_walk_drops — this is a guard; tests/mutations.rs must show it red.
    #[test]
    fn a_state_the_fingerprint_merges_is_a_state_the_walk_drops() {
        struct Sneaky {
            n: u8,
            sep: &'static str,
        }
        impl Component for Sneaky {
            fn handle(&mut self, key: Key) -> Flow {
                if key == Key::Right && self.n < 2 {
                    self.n += 1;
                }
                Flow::Stay
            }
            fn view(&self) -> View {
                let (title, footer) = match self.n {
                    0 => (format!("a{}b", self.sep), "c".to_string()),
                    1 => ("a".to_string(), format!("b{}c", self.sep)),
                    // Reachable ONLY through state 1.
                    _ => ("third".to_string(), String::new()),
                };
                View::titled(title)
                    .row(Row::new("r", "v").selected())
                    .footer(footer)
            }
        }

        let in_range = properties::selection_is_always_in_range();
        for sep in ["\u{1f}", "/", " | ", ""] {
            let report = Explorer::new([Key::Right]).explore(|| Sneaky { n: 0, sep }, &[&in_range]);
            assert!(report.exhausted, "nothing capped this walk: {report}");
            assert_eq!(
                report.states, 3,
                "separator {sep:?}: two distinct states merged, so the third — \
                 reachable only through the second — was never walked, and the \
                 report says the search finished: {report}"
            );
            assert!(
                report.views.iter().any(|v| v.title == "third"),
                "separator {sep:?}: the corpus is a silent SUBSET, handed to a \
                 reimplementation as the states it must satisfy: {report}"
            );
        }
    }

    /// **A walk that checked no property is not a clean bill** — P6.
    ///
    /// It finished, it hit no limit, and it judged nothing: `is_clean()` used
    /// to return true, and a report carried no count that could tell you. The
    /// degenerate case of the crate's own discipline — a search reports what it
    /// looked at, and "nothing" is a thing to report.
    // GUARD: tests::a_run_that_checked_nothing_is_not_a_clean_bill — this is a guard; tests/mutations.rs must show it red.
    #[test]
    fn a_run_that_checked_nothing_is_not_a_clean_bill() {
        let report = Explorer::new(Key::navigation()).explore(dial, &[]);
        assert!(report.exhausted, "the walk itself finished: {report}");
        assert!(!report.is_clean(), "but it judged nothing: {report}");
        assert!(
            matches!(report.verdict(), Verdict::Incomplete { reason, .. }
                     if reason.contains("NO PROPERTY")),
            "and says which vacuity it is: {report}"
        );
    }

    /// **A walk over an empty alphabet is not a clean bill** — P6, the other
    /// half. No key was ever applied, so every property held over exactly one
    /// state: the start. `Explorer::new`'s own doc encourages trimming the
    /// alphabet, and the empty trim is a proof about nothing.
    // GUARD: tests::a_walk_over_an_empty_alphabet_is_not_a_clean_bill — this is a guard; tests/mutations.rs must show it red.
    #[test]
    fn a_walk_over_an_empty_alphabet_is_not_a_clean_bill() {
        let owned = acceptance();
        let refs: Vec<&dyn Property> = owned.iter().map(AsRef::as_ref).collect();
        let report = Explorer::new(Vec::new()).explore(dial, &refs);
        assert_eq!(report.transitions, 0, "no key exists to apply: {report}");
        assert!(report.exhausted, "and nothing capped it: {report}");
        assert!(!report.is_clean(), "which proves nothing: {report}");
        // The REASON, not just the flag. An empty alphabet also leaves every
        // transition property inapplicable, so two conjuncts can refuse this
        // walk — and a test that only asked `!is_clean()` could not say which
        // one it was pinning, and would stay green with this one deleted.
        assert!(
            matches!(report.verdict(), Verdict::Incomplete { reason, .. }
                     if reason.contains("NO KEY WAS EVER APPLIED")),
            "and says which vacuity it is: {report}"
        );
    }

    /// **A property whose domain the alphabet never reaches is not a clean
    /// bill** — the second blocking finding of the review of this branch.
    ///
    /// `Report.properties_checked` was `properties.len()`, assigned before a
    /// single observation was examined, and `incomplete_because` treated
    /// nonzero as sufficient. So this walk — a real component, a real alphabet,
    /// four states, four transitions, one supplied property, no violations,
    /// `exhausted: true` — came back `Clean` without Escape ever being applied,
    /// against a property whose entire subject is Escape. Reproduced before the
    /// fix:
    ///
    /// ```text
    /// 4 states, 4 transitions, 0 terminal, 1 properties
    /// no violations
    /// exhausted=true properties_checked=1 violations=0 is_clean=true
    /// ```
    ///
    /// It is the same shape as the two degenerate runs above and could not be
    /// caught by the same mechanism, because a count cannot distinguish a
    /// property that HELD from one that was never asked anything it knows
    /// about. `Property` returning `Result<(), String>` could not either, which
    /// is why the seam grew a third answer.
    // GUARD: tests::a_property_the_alphabet_never_reaches_is_not_a_clean_bill — this is a guard; tests/mutations.rs must show it red.
    #[test]
    fn a_property_the_alphabet_never_reaches_is_not_a_clean_bill() {
        let escapes = properties::escape_always_closes_without_applying();
        let report = Explorer::new([Key::Right]).explore(dial, &[&escapes]);

        assert!(report.exhausted, "the walk itself finished: {report}");
        assert!(report.transitions > 0, "and it walked somewhere: {report}");
        assert_eq!(report.properties.len(), 1, "with a property: {report}");
        assert!(
            !report.is_clean(),
            "but Escape was never applied, so the one claim it was given has \
             said nothing: {report}"
        );
        assert!(
            matches!(report.verdict(), Verdict::Incomplete { reason, .. }
                     if reason.contains("NEVER APPLIED")),
            "and says which vacuity it is: {report}"
        );
        assert_eq!(
            report.never_applied(),
            ["escape always closes without applying"],
            "naming the property, because the fix is usually the alphabet: \
             {report}"
        );

        // The coverage is what a count could not carry: consulted at every
        // observation, in its domain at none of them.
        let coverage = &report.properties[0];
        assert!(coverage.observations > 0, "it WAS consulted: {report}");
        assert_eq!(coverage.applicable, 0, "and never in its domain: {report}");
        assert_eq!(coverage.held, 0, "so it held nothing: {report}");

        // Give it the key it is about and the same walk is clean — so this is
        // the alphabet being judged, not a report that is pessimistic about
        // everything.
        let full = Explorer::new([Key::Right, Key::Esc]).explore(dial, &[&escapes]);
        assert!(full.is_clean(), "{full}");
        assert!(full.properties[0].held > 0, "and it held: {full}");
    }

    /// **Two properties that share a name are both checked** — R5.
    ///
    /// Retirement is per PROPERTY: the same broken rule reached by forty paths
    /// is one defect. It used to be keyed on the caller-supplied name STRING,
    /// so two distinct claims that happened to share a name retired each other
    /// — the second was never evaluated at any depth, in a report stamped
    /// `exhausted: true`. `Named::new` takes a free-form `impl Into<String>`
    /// with no uniqueness check anywhere, and an acceptance set composed from
    /// two modules is this crate's whole distribution story.
    // GUARD: tests::two_properties_with_one_name_are_both_checked — this is a guard; tests/mutations.rs must show it red.
    #[test]
    fn two_properties_with_one_name_are_both_checked() {
        let first = Named::new("invariant", |_: &Observation<'_>| {
            PropertyOutcome::Violated("the FIRST claim broke".to_string())
        });
        let second = Named::new("invariant", |_: &Observation<'_>| {
            PropertyOutcome::Violated("the SECOND claim broke".to_string())
        });
        let report = Explorer::new(Key::navigation()).explore(dial, &[&first, &second]);

        let Verdict::Violated(found) = report.verdict() else {
            panic!("both claims broke: {report}")
        };
        let details: Vec<&str> = found.iter().map(|v| v.detail.as_str()).collect();
        assert_eq!(
            details,
            ["the FIRST claim broke", "the SECOND claim broke"],
            "a property silenced by a NAME COLLISION is never checked at any \
             depth, in a report that calls itself exhaustive: {report}"
        );
    }

    /// **A named lookup returns EVERY violation with that name**, not the
    /// first one.
    ///
    /// The two halves have to agree. R5 made retirement per POSITION precisely
    /// so that two claims sharing a name are two findings — and a singular
    /// `violation(name)` that returned the first would then hide the second
    /// behind the same name collision, one accessor away from the bug R5 had
    /// just fixed. `Named::new` takes a free-form string with no uniqueness
    /// check, so this is reachable by composing two acceptance sets, which is
    /// the crate's distribution story.
    // GUARD: tests::a_named_lookup_returns_every_violation_with_that_name — this is a guard; tests/mutations.rs must show it red.
    #[test]
    fn a_named_lookup_returns_every_violation_with_that_name() {
        let first = Named::new("invariant", |_: &Observation<'_>| {
            PropertyOutcome::Violated("the FIRST claim broke".to_string())
        });
        let second = Named::new("invariant", |_: &Observation<'_>| {
            PropertyOutcome::Violated("the SECOND claim broke".to_string())
        });
        let report = Explorer::new(Key::navigation()).explore(dial, &[&first, &second]);

        let found = report.violations_named("invariant");
        let details: Vec<&str> = found.iter().map(|v| v.detail.as_str()).collect();
        assert_eq!(
            details,
            ["the FIRST claim broke", "the SECOND claim broke"],
            "a lookup that returns the first match hides the second behind the \
             very name collision R5 stopped silencing it: {report}"
        );
        assert!(
            report.violations_named("no such claim").is_empty(),
            "and a name nothing broke under is empty, not a panic: {report}"
        );
    }

    /// The recorded corpus is the states themselves — data a reimplementation
    /// can be held to without sharing code with this one.
    #[test]
    fn the_state_corpus_is_recordable() {
        let report = check(dial);
        assert!(report.exhausted, "and it is all of them: {report}");
        let open: Vec<&str> = report
            .views
            .iter()
            .filter(|v| v.footer.starts_with('\u{2190}'))
            .map(|v| v.rows[0].value.as_str())
            .collect();
        assert_eq!(open, ["0", "1", "2", "3"], "in discovery order");

        // A truncated corpus says so, and it is the SAME flag that says the
        // search was truncated, because it is the same walk. Handed to a
        // reimplementation as "the states", a silent subset would pass
        // something that does less.
        let capped = Explorer::new(Key::navigation())
            .max_states(2)
            .explore(dial, &[]);
        assert!(
            !capped.exhausted,
            "a capped corpus is a subset, and admits it"
        );
    }

    /// **A state that CLOSED is a state the corpus carries** — bug 3a.
    ///
    /// The corpus used to come from a second walk, `Explorer::states`, which
    /// `continue`d past every closing view without recording it while its
    /// `complete` flag — cleared at exactly one site, the numeric limit —
    /// stayed true. So `complete` meant "no cap was hit" while its doc promised
    /// "not a subset", and this dial reported a COMPLETE corpus of 4 views out
    /// of 6 reachable. Every settings panel has a closing view that differs
    /// from its open one, so the trigger is not exotic; it is the motivating
    /// component.
    ///
    /// The fix was to delete the second walk. One walk, one completeness flag,
    /// and the law is satisfied by construction rather than by two functions
    /// remembering to agree.
    // GUARD: tests::the_corpus_holds_the_states_that_closed — this is a guard; tests/mutations.rs must show it red.
    #[test]
    fn the_corpus_holds_the_states_that_closed() {
        let report = check(dial);
        assert!(report.exhausted, "nothing capped this walk: {report}");
        let footers: Vec<&str> = report.views.iter().map(|v| v.footer.as_str()).collect();
        assert!(
            footers.contains(&"saved") && footers.contains(&"discarded"),
            "a corpus of {} views calling itself complete, missing the states \
             an operator ends in: {footers:?}",
            report.views.len()
        );
        // Four levels, each of which the operator can leave two ways: 4 open
        // views and 8 closing ones. The deleted walk called 4 of the 12
        // "complete".
        assert_eq!(report.views.len(), 12, "4 open, 8 closing: {footers:?}");
    }

    // --- the replay guard --------------------------------------------------

    /// A counter whose factory can DRIFT: the first `agrees_for` products
    /// start at 0, every one after that starts at 1.
    ///
    /// This is what an impure factory looks like in miniature — a cached read,
    /// a clock, a `OnceLock` filled by an earlier test, a pre-warmed buffer.
    /// The important half is `agrees_for`: a factory that misbehaved on call
    /// one would be caught by almost anything, and is not the interesting case.
    struct Counter {
        level: u8,
        /// Closes on Right instead of counting — the drifted machine that is
        /// not merely at a different level but a different SHAPE.
        brittle: bool,
    }

    impl Component for Counter {
        fn handle(&mut self, key: Key) -> Flow {
            match key {
                Key::Right if self.brittle => Flow::Close(false),
                Key::Right => {
                    self.level = (self.level + 1).min(3);
                    Flow::Stay
                }
                Key::Esc => Flow::Close(false),
                _ => Flow::Stay,
            }
        }

        fn view(&self) -> View {
            View::titled("counter").row(
                Row::new("level", self.level.to_string())
                    .selected()
                    .adjustable(),
            )
        }
    }

    /// A factory that counts its own products, so a test can say WHEN it drifts.
    struct Drifting {
        built: std::cell::Cell<usize>,
        agrees_for: usize,
        brittle_after: Option<usize>,
    }

    impl Drifting {
        fn new(agrees_for: usize) -> Self {
            Self {
                built: std::cell::Cell::new(0),
                agrees_for,
                brittle_after: None,
            }
        }

        fn brittle_after(mut self, n: usize) -> Self {
            self.brittle_after = Some(n);
            self
        }

        fn build(&self) -> Counter {
            let n = self.built.get();
            self.built.set(n + 1);
            Counter {
                level: u8::from(n >= self.agrees_for),
                brittle: self.brittle_after.is_some_and(|after| n >= after),
            }
        }
    }

    fn walk(factory: impl Fn() -> Counter) -> Report {
        let owned = acceptance();
        let refs: Vec<&dyn Property> = owned.iter().map(AsRef::as_ref).collect();
        Explorer::new([Key::Right, Key::Esc]).explore(factory, &refs)
    }

    /// A deterministic factory replays to the same state every time, so the
    /// guard is silent and the walk is exactly what it was before.
    ///
    /// The anti-vacuous half of every test below: a guard that fired on an
    /// ordinary pure component would be useless however well it caught the
    /// impure ones, and nothing else here would notice.
    #[test]
    fn a_deterministic_factory_replays_where_discovery_said() {
        let report = walk(|| Counter {
            level: 0,
            brittle: false,
        });
        assert!(
            report.divergences().is_empty(),
            "a pure factory diverged from itself: {:?}",
            report.divergences()
        );
        assert!(
            matches!(report.verdict(), Verdict::Clean),
            "{report}\n{:?}",
            report.verdict()
        );
    }

    /// **A replay that lands somewhere else makes the report INCOMPLETE**, and
    /// it is not a property violation.
    ///
    /// Nothing about the component has been shown to be wrong. What has been
    /// shown is that the harness walked one machine and judged another, so the
    /// honest answer is that this report clears nothing — the same answer a
    /// capped run gets, for a worse reason.
    // GUARD: tests::a_replay_that_lands_elsewhere_is_not_a_clean_bill — this is a guard; tests/mutations.rs must show it red.
    #[test]
    fn a_replay_that_lands_elsewhere_is_not_a_clean_bill() {
        // Agrees for the initial build and the first reconstruction, then
        // drifts — so discovery is consistent and a LATER replay is not.
        let drifting = Drifting::new(2);
        let report = walk(|| drifting.build());

        assert!(
            !report.divergences().is_empty(),
            "the factory changed its product mid-walk and nothing noticed: \
             {report}"
        );
        match report.verdict() {
            Verdict::Incomplete { reason, .. } => assert!(
                reason.contains("REPLAY"),
                "incomplete for the wrong reason: {reason}"
            ),
            other => panic!("a walk over a drifting factory was {other:?}: {report}"),
        }
        assert!(
            !report.is_clean(),
            "a report from a machine that was never walked called itself clean"
        );
    }

    /// A component that CLOSES partway through a replay is a divergence too,
    /// and a distinguishable one.
    ///
    /// This is the case a fingerprint comparison alone would report as an
    /// ordinary mismatch and misdiagnose: the replay did not arrive at another
    /// state, it stopped being able to receive keys at all. Every remaining key
    /// in the path would have been delivered to a closed component.
    // GUARD: tests::a_replay_that_closes_early_is_caught_as_such — this is a guard; tests/mutations.rs must show it red.
    #[test]
    fn a_replay_that_closes_early_is_caught_as_such() {
        // Level never drifts; the SHAPE does, after the frontier has been
        // built, so a path of length >= 1 is replayed into a component that
        // closes on the key it used to count with.
        let drifting = Drifting::new(usize::MAX).brittle_after(3);
        let report = walk(|| drifting.build());

        let closed_early = report
            .divergences()
            .iter()
            .any(|d| matches!(d.reason, DivergenceReason::ClosedDuringReplay { .. }));
        assert!(
            closed_early,
            "a component that closed DURING replay was not reported as such: \
             {:?}",
            report.divergences()
        );
        assert!(!report.is_clean(), "{report}");
    }

    /// **Nothing is judged from a departure point already known to be wrong.**
    ///
    /// The divergence is detected before the outgoing key is applied, so the
    /// walk records no transition from there — a violation found on that
    /// machine would be reported against a path that does not reach it.
    ///
    /// It also pins the ORDER of the incompleteness reasons. This walk has zero
    /// transitions, and "NO KEY WAS EVER APPLIED" is true of it; the reason it
    /// must give is the divergence, because that is the one that explains why
    /// no key was applied.
    // GUARD: tests::a_diverged_departure_judges_nothing — this is a guard; tests/mutations.rs must show it red.
    #[test]
    fn a_diverged_departure_judges_nothing() {
        // Drifts on the very first reconstruction: the initial build is level
        // 0, every product after it is level 1, so the empty path replays to
        // the wrong state before a single key is pressed.
        let drifting = Drifting::new(1);
        let report = walk(|| drifting.build());

        assert_eq!(
            report.transitions, 0,
            "a key was applied to a machine the search never explored: {report}"
        );
        assert!(report
            .violations_named("selection is always in range")
            .is_empty());
        match report.verdict() {
            Verdict::Incomplete { reason, violations } => {
                assert!(
                    reason.contains("REPLAY"),
                    "the divergence is the reason that explains the others, so \
                     it must be the one reported, not {reason:?}"
                );
                assert!(violations.is_empty(), "{violations:?}");
            }
            other => panic!("{other:?}: {report}"),
        }
    }

    /// A divergence prints the path and BOTH fingerprints.
    ///
    /// The point of the guard is diagnosis: a factory that is not reproducing
    /// its own states is a bug in the caller's test setup, and "something was
    /// inconsistent" would send them looking in the wrong place.
    #[test]
    fn a_divergence_says_enough_to_diagnose_it() {
        let drifting = Drifting::new(2);
        let report = walk(|| drifting.build());
        let first = report
            .divergences()
            .first()
            .expect("the drifting factory diverged");
        let printed = first.to_string();

        assert!(
            printed.contains("expected") && printed.contains("actual"),
            "{printed}"
        );
        assert_ne!(
            first.expected, first.actual,
            "a divergence whose two fingerprints are equal is not one"
        );
        assert!(
            printed.contains(&first.expected.to_string())
                && printed.contains(&first.actual.to_string()),
            "the fingerprints are not in the message: {printed}"
        );
        assert!(
            first.path.is_empty() || printed.contains(&first.path[0].to_string()),
            "the path is not in the message: {printed}"
        );
    }
}
