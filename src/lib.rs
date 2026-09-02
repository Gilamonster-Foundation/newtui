//! **Terminal UI components you can drive in isolation.**
//!
//! Two families:
//!
//! - **Components** are interactive — a state machine over keys. A settings
//!   panel, a chooser, a form, a pager.
//! - **Widgets** are display — a pure function from data to cells. A
//!   sparkline, a butterfly meter, a heat bar, a gauge. (Landing next; the
//!   interaction seam came first because it is the one with a correctness
//!   story that a test can hold.)
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
mod explore;
mod key;
mod property;
mod view;

pub use component::{Component, Fingerprint, Flow};
pub use explore::{Explorer, Report, Verdict, Violation};
pub use key::Key;
pub use property::{properties, Named, Observation, Property};
pub use view::{Row, View};

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
#[cfg(doctest)]
#[doc = include_str!("../README.md")]
struct TheReadmeIsCompiledAndRun;

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
        let violation = report
            .violation("only adjustable rows move")
            .unwrap_or_else(|| panic!("the door's dialling must be caught: {report}"));
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
        let violation = report
            .violation("escape always closes without applying")
            .unwrap_or_else(|| panic!("the trap must be caught: {report}"));
        assert_eq!(violation.path, vec![Key::Down, Key::Esc], "{violation}");
    }

    /// A capped search says so, and `is_clean` refuses to call it a proof.
    /// "No violations in the part I looked at" is not "no violations".
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
    #[test]
    fn a_walk_over_an_empty_alphabet_is_not_a_clean_bill() {
        let owned = acceptance();
        let refs: Vec<&dyn Property> = owned.iter().map(AsRef::as_ref).collect();
        let report = Explorer::new(Vec::new()).explore(dial, &refs);
        assert_eq!(report.transitions, 0, "no key exists to apply: {report}");
        assert!(report.exhausted, "and nothing capped it: {report}");
        assert!(!report.is_clean(), "which proves nothing: {report}");
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
    #[test]
    fn two_properties_with_one_name_are_both_checked() {
        let first = Named::new("invariant", |_: &Observation<'_>| {
            Err("the FIRST claim broke".to_string())
        });
        let second = Named::new("invariant", |_: &Observation<'_>| {
            Err("the SECOND claim broke".to_string())
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
}
