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
pub use explore::{Explorer, Report, Violation};
pub use key::Key;
pub use property::{properties, Named, Observation, Property};
pub use view::{Row, View};

#[cfg(test)]
mod tests {
    use super::*;

    /// A dial that CLAMPS — the shape most rows in this line have.
    struct Dial {
        level: u8,
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
                Key::Enter => Flow::Close(true),
                Key::Esc => Flow::Close(false),
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
                .footer("←→ change · Enter apply · Esc cancel")
        }
    }

    fn dial() -> Dial {
        Dial { level: 0 }
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
            .violations
            .iter()
            .find(|v| v.property == "only adjustable rows move")
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
            .violations
            .iter()
            .find(|v| v.property == "escape always closes without applying")
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

        let report = Explorer::new([Key::Right])
            .max_states(10)
            .explore(|| Endless { n: 0 }, &[]);
        assert!(!report.exhausted, "an unbounded dial cannot be exhausted");
        assert!(!report.is_clean(), "a sample is not a clean bill of health");
        assert!(
            report.to_string().contains("STOPPED AT A LIMIT"),
            "and the report says so: {report}"
        );
    }

    /// The recorded corpus is the states themselves — data a reimplementation
    /// can be held to without sharing code with this one.
    #[test]
    fn the_state_corpus_is_recordable() {
        let states = Explorer::new(Key::navigation()).states(dial);
        assert_eq!(states.len(), 4);
        let values: Vec<&str> = states.iter().map(|v| v.rows[0].value.as_str()).collect();
        assert_eq!(values, ["0", "1", "2", "3"], "in discovery order");
    }
}
