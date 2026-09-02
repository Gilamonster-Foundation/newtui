//! What a component MUST do — as named, checkable claims.
//!
//! A property is a claim about OBSERVABLE BEHAVIOUR: it reads a view, or a
//! before/key/after triple, and says whether the component still holds. It
//! never reaches inside a component, which is what lets the same property judge
//! two implementations of one thing — including a reimplementation in another
//! language, another framework, or another agent's codebase.
//!
//! They are data. A component adds one by writing a closure, not a test file,
//! and the library ships the ones every component of this shape needs.

use crate::{Flow, Key, View};

/// What the explorer is showing a property.
#[derive(Debug, Clone, Copy)]
pub enum Observation<'a> {
    /// A reachable state, on its own.
    State {
        /// What the component shows here.
        view: &'a View,
    },
    /// One key applied to one state.
    Transition {
        /// What it showed before the key.
        from: &'a View,
        /// The key applied.
        key: Key,
        /// What it shows after.
        to: &'a View,
        /// What the key did — stayed open, or closed (and whether as an apply).
        flow: Flow,
    },
}

impl Observation<'_> {
    /// The state being judged — the one that RESULTED, for a transition.
    #[must_use]
    pub fn view(&self) -> &View {
        match self {
            Self::State { view } | Self::Transition { to: view, .. } => view,
        }
    }
}

/// What a property made of ONE observation.
///
/// Three answers, not two, for the same reason [`crate::Verdict`] has three.
/// A property that says "no complaint" is saying one of two entirely different
/// things — *I judged this and it held*, or *this is not mine to judge* — and
/// `Result<(), String>` cannot tell them apart. That gap is what let
/// `Report.properties_checked` mean SUPPLIED while it was named CHECKED: the
/// count was `properties.len()`, fixed before a single observation was looked
/// at, and a walk over an alphabet that never reaches a property's domain came
/// back `Clean` with that property never once applied.
///
/// The split is DOMAIN versus BODY: whether the observation is one this claim
/// speaks about at all, and then whether the claim holds of it. `Held` is the
/// right answer for a body that is satisfied vacuously — an arrow on a row that
/// IS adjustable is an observation `only_adjustable_rows_move` judged and
/// passed, not one it declined.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PropertyOutcome {
    /// Outside this property's domain — it has nothing to say here, and its
    /// silence is not evidence about anything.
    NotApplicable,
    /// In its domain, and it held.
    Held,
    /// In its domain, and it did not hold. Carries what went wrong, phrased for
    /// someone reading a failure.
    Violated(String),
}

/// A named claim, checked at every state and transition the explorer reaches.
pub trait Property {
    /// How this claim is named in a report.
    fn name(&self) -> &str;

    /// Judge one observation. See [`PropertyOutcome`] for why there are three
    /// answers and not two.
    fn check(&self, observation: &Observation<'_>) -> PropertyOutcome;
}

/// A property written as a closure — the way a component declares its own.
pub struct Named<F> {
    name: String,
    check: F,
}

impl<F> Named<F>
where
    F: Fn(&Observation<'_>) -> PropertyOutcome,
{
    /// Name a claim and give it a body.
    pub fn new(name: impl Into<String>, check: F) -> Self {
        Self {
            name: name.into(),
            check,
        }
    }
}

impl<F> Property for Named<F>
where
    F: Fn(&Observation<'_>) -> PropertyOutcome,
{
    fn name(&self) -> &str {
        &self.name
    }

    fn check(&self, observation: &Observation<'_>) -> PropertyOutcome {
        (self.check)(observation)
    }
}

/// The claims every row-and-value component in this line has to satisfy.
///
/// Each one is here because it was a real defect somewhere first. They are
/// functions rather than consts so a caller composes the set it wants — three
/// Cs: the acceptance set is data a component assembles, not a fixed list this
/// module hardcodes for everyone.
pub mod properties {
    use super::{Named, Observation, PropertyOutcome};
    use crate::Flow;

    /// **At most one row is selected, and a non-empty component selects one.**
    ///
    /// The defect this catches: a selection index into a row list that is
    /// REBUILT each frame. When a row disappears — a conditional field, a
    /// filtered list — the index can point past the end, and the component
    /// silently acts on nothing. Reported as "nothing happened", which is the
    /// hardest kind of bug to be shown.
    ///
    /// Domain: any view with rows. A view with none has no selection to be in
    /// or out of range, so it is `NotApplicable` rather than a free pass — a
    /// component whose every view is empty is one this property never judged,
    /// and the report says so instead of calling it clean.
    #[must_use]
    pub fn selection_is_always_in_range() -> Named<impl Fn(&Observation<'_>) -> PropertyOutcome> {
        Named::new("selection is always in range", |observation| {
            let view = observation.view();
            if view.rows.is_empty() {
                return PropertyOutcome::NotApplicable;
            }
            match view.selection_count() {
                0 => PropertyOutcome::Violated(format!(
                    "{} rows and nothing selected — a key would act on no row",
                    view.rows.len()
                )),
                1 => PropertyOutcome::Held,
                n => PropertyOutcome::Violated(format!("{n} rows claim the cursor at once")),
            }
        })
    }

    /// **Esc always closes, and never applies.**
    ///
    /// The way out must exist from EVERY reachable state, not just the ones a
    /// demo visits. A sub-mode that swallows Esc — an open command line, a
    /// nested form — strands the operator in a state whose exit they have to
    /// guess.
    ///
    /// A component with a genuine two-stage exit (Esc closes a sub-mode, a
    /// second Esc closes the component) does not satisfy this as written and
    /// should say so: use `escape_always_leaves_something` instead, which
    /// requires only that Esc CHANGES the state.
    ///
    /// Domain: an escape key applied to a state. Over an alphabet with no
    /// escape in it this property is `NotApplicable` everywhere, which is
    /// exactly the walk that used to come back `Clean` having never tried the
    /// key the claim is about.
    #[must_use]
    pub fn escape_always_closes_without_applying(
    ) -> Named<impl Fn(&Observation<'_>) -> PropertyOutcome> {
        Named::new("escape always closes without applying", |observation| {
            let Observation::Transition { key, flow, .. } = observation else {
                return PropertyOutcome::NotApplicable;
            };
            if !key.is_escape() {
                return PropertyOutcome::NotApplicable;
            }
            match flow {
                Flow::Close(false) => PropertyOutcome::Held,
                Flow::Close(true) => {
                    PropertyOutcome::Violated("escape closed the component AS AN APPLY".to_string())
                }
                Flow::Stay => {
                    PropertyOutcome::Violated("escape left the component open".to_string())
                }
            }
        })
    }

    /// The weaker escape claim, for a component with sub-modes: Esc must
    /// always DO something — close, or leave the state it was in.
    ///
    /// Same domain as the strict form: an escape key applied to a state.
    #[must_use]
    pub fn escape_always_leaves_something() -> Named<impl Fn(&Observation<'_>) -> PropertyOutcome> {
        Named::new("escape always leaves something", |observation| {
            let Observation::Transition {
                from,
                key,
                to,
                flow,
                ..
            } = observation
            else {
                return PropertyOutcome::NotApplicable;
            };
            if !key.is_escape() {
                return PropertyOutcome::NotApplicable;
            }
            if matches!(flow, Flow::Close(_)) || from != to {
                PropertyOutcome::Held
            } else {
                PropertyOutcome::Violated(
                    "escape neither closed nor changed anything — a state with \
                     no way out"
                        .to_string(),
                )
            }
        })
    }

    /// **A row that is not adjustable does not move under ← or →.**
    ///
    /// The chrome a renderer draws for an adjustable row PROMISES that arrows
    /// change it. The inverse has to hold too, or a stray arrow silently
    /// repoints something the operator was only passing over — and this is the
    /// property that says so at every state, not just the row someone tested.
    ///
    /// Domain: a horizontal arrow applied to a state that has a cursor row, and
    /// that row still exists after the key. The `adjustable` flag is the BODY,
    /// not the domain: an arrow on a row that IS adjustable is an observation
    /// this property judged and passed. Reading it the other way — vacuously
    /// true, therefore not applicable — was tried and reverted, because it
    /// makes this property `NotApplicable` at every observation of a component
    /// whose rows are all dials, which is the crate's own worked example and
    /// its own `Dial` test. The line has to fall somewhere, and "did this
    /// observation get judged" is the honest place.
    ///
    /// It is also, per the review, only as good as `Row::adjustable` — which
    /// conflates *the operator can dial this* with *this changes under an
    /// arrow*, and those come apart on any chooser with a derived detail pane.
    /// A row-quantified rewrite is blocked on that, not on this seam.
    #[must_use]
    pub fn only_adjustable_rows_move() -> Named<impl Fn(&Observation<'_>) -> PropertyOutcome> {
        Named::new("only adjustable rows move", |observation| {
            let Observation::Transition { from, key, to, .. } = observation else {
                return PropertyOutcome::NotApplicable;
            };
            if !matches!(key, crate::Key::Left | crate::Key::Right) {
                return PropertyOutcome::NotApplicable;
            }
            let Some(at) = from.selected() else {
                return PropertyOutcome::NotApplicable;
            };
            let (Some(before), Some(after)) = (from.rows.get(at), to.rows.get(at)) else {
                return PropertyOutcome::NotApplicable;
            };
            if before.adjustable || before.value == after.value {
                PropertyOutcome::Held
            } else {
                PropertyOutcome::Violated(format!(
                    "`{}` is not adjustable but {key} changed it from `{}` to `{}`",
                    before.label, before.value, after.value
                ))
            }
        })
    }

    // DELETED: `a_repeated_key_reaches_a_fixed_point`. Its body was
    // `|_| Ok(())` — a property that cannot fail, which is the exact thing
    // this crate exists to refuse — while its doc claimed the explorer flagged
    // a revisit and the README sold it in the shipped set. Nothing in
    // `explore.rs` detected cycles or referenced the name: a WRAPPING dial,
    // the property's own subject, explored with only that property, came back
    // "4 states, 4 transitions, no violations, is_clean() == true".
    //
    // Deleted rather than implemented, because it is not the same shape as
    // anything else here. Every other property judges one observation and can
    // say so at the state that broke it; cycle-freedom is a claim about the
    // GRAPH, computable in `Report` from edges the walk already has, and it
    // would have to arrive as a report-level check with its own counterexample
    // vocabulary. Wrapping is also a legitimate design, so it is a claim a
    // component opts into rather than one the harness asserts. Worth having;
    // not worth pretending to have.
    //
    // `every_shipped_property_rejects_something` below is what stops the shape
    // coming back.
}

#[cfg(test)]
mod tests {
    use super::properties::*;
    use super::*;
    use crate::{Row, View};

    fn view(rows: Vec<Row>) -> View {
        View {
            title: "t".into(),
            rows,
            footer: String::new(),
        }
    }

    /// What a property said went wrong, or a panic naming what it said instead.
    ///
    /// The positive cases below assert `Held` or `NotApplicable` by NAME rather
    /// than through one "no complaint" predicate. That distinction is the whole
    /// of the fix: `is_ok()` could not tell a property that judged an
    /// observation from one that declined it, and a report built on the second
    /// reading called a walk clean that had never applied the key its only
    /// claim was about.
    fn refusal(outcome: PropertyOutcome) -> String {
        match outcome {
            PropertyOutcome::Violated(detail) => detail,
            other => panic!("expected a violation, got {other:?}"),
        }
    }

    #[test]
    fn the_selection_property_catches_both_shapes_of_wrong() {
        let p = selection_is_always_in_range();
        let ok = view(vec![Row::new("a", "1").selected(), Row::new("b", "2")]);
        assert_eq!(
            p.check(&Observation::State { view: &ok }),
            PropertyOutcome::Held
        );

        let none = view(vec![Row::new("a", "1")]);
        let err = refusal(p.check(&Observation::State { view: &none }));
        assert!(err.contains("nothing selected"), "{err}");

        let two = view(vec![
            Row::new("a", "1").selected(),
            Row::new("b", "2").selected(),
        ]);
        assert!(refusal(p.check(&Observation::State { view: &two })).contains("at once"));

        // An empty component has no selection to be in or out of range. NOT a
        // pass: there was nothing here for this property to judge, and a
        // component whose every view is empty is one it never judged at all.
        let empty = view(Vec::new());
        assert_eq!(
            p.check(&Observation::State { view: &empty }),
            PropertyOutcome::NotApplicable
        );
    }

    #[test]
    fn the_escape_property_rejects_both_ways_of_getting_it_wrong() {
        let p = escape_always_closes_without_applying();
        let v = view(vec![Row::new("a", "1").selected()]);
        let at = |flow| Observation::Transition {
            from: &v,
            key: Key::Esc,
            to: &v,
            flow,
        };
        assert_eq!(p.check(&at(Flow::Close(false))), PropertyOutcome::Held);
        assert!(refusal(p.check(&at(Flow::Close(true)))).contains("AS AN APPLY"));
        assert!(refusal(p.check(&at(Flow::Stay))).contains("left the component open"));

        // A non-escape key is none of its business — and that is NOT the same
        // answer as "escape behaved", which is what a `Result` made of it.
        assert_eq!(
            p.check(&Observation::Transition {
                from: &v,
                key: Key::Enter,
                to: &v,
                flow: Flow::Close(true),
            }),
            PropertyOutcome::NotApplicable
        );

        // Neither is a state observation, where no key was applied at all.
        assert_eq!(
            p.check(&Observation::State { view: &v }),
            PropertyOutcome::NotApplicable
        );
    }

    #[test]
    fn a_door_that_dials_is_caught() {
        let p = only_adjustable_rows_move();
        let before = view(vec![Row::new("backend", "sol").selected()]);
        let after = view(vec![Row::new("backend", "other").selected()]);
        let err = refusal(p.check(&Observation::Transition {
            from: &before,
            key: Key::Right,
            to: &after,
            flow: Flow::Stay,
        }));
        assert!(err.contains("not adjustable"), "{err}");

        // The same move on an adjustable row is the point of the row — and it
        // is HELD, not NotApplicable. The domain is "an arrow on the cursor
        // row"; the `adjustable` flag is the body. Read the other way, this
        // property would be inapplicable at every observation of a component
        // whose rows are all dials — the crate's own `Dial` and the README's
        // `Volume` — and both would report Incomplete.
        let dial_before = view(vec![Row::new("tenacity", "auto").adjustable().selected()]);
        let dial_after = view(vec![Row::new("tenacity", "relaxed")
            .adjustable()
            .selected()]);
        assert_eq!(
            p.check(&Observation::Transition {
                from: &dial_before,
                key: Key::Right,
                to: &dial_after,
                flow: Flow::Stay,
            }),
            PropertyOutcome::Held
        );

        // A vertical key is outside the domain entirely.
        assert_eq!(
            p.check(&Observation::Transition {
                from: &dial_before,
                key: Key::Down,
                to: &dial_after,
                flow: Flow::Stay,
            }),
            PropertyOutcome::NotApplicable
        );
    }

    /// **Every shipped property has an observation it REFUSES**, and the count
    /// is pinned to the module so a new one cannot arrive without a witness.
    ///
    /// The defect this replaces: `a_repeated_key_reaches_a_fixed_point` shipped
    /// with the body `|_| Ok(())`, advertised in the README, documenting cycle
    /// detection that existed nowhere. A property that cannot fail is the exact
    /// thing this crate exists to refuse, and it is invisible to every other
    /// test in the file — each of those checks the properties it names, and
    /// this one checks that the file names no others.
    ///
    /// The source scan asserts it READ something before it counts, because an
    /// absence check fails OPEN: anything that shrinks the scanned text makes
    /// it likelier to pass.
    #[test]
    fn every_shipped_property_rejects_something() {
        let none_selected = view(vec![Row::new("a", "1")]);
        let cursor = view(vec![Row::new("a", "1").selected()]);
        let moved = view(vec![Row::new("a", "2").selected()]);
        let swallowed = |key| Observation::Transition {
            from: &cursor,
            key,
            to: &cursor,
            flow: Flow::Stay,
        };

        // Each shipped property, against an observation that MUST break it.
        let refusals = [
            (
                "selection is always in range",
                selection_is_always_in_range().check(&Observation::State {
                    view: &none_selected,
                }),
            ),
            (
                "escape always closes without applying",
                escape_always_closes_without_applying().check(&swallowed(Key::Esc)),
            ),
            (
                "escape always leaves something",
                escape_always_leaves_something().check(&swallowed(Key::Esc)),
            ),
            (
                "only adjustable rows move",
                only_adjustable_rows_move().check(&Observation::Transition {
                    from: &cursor,
                    key: Key::Right,
                    to: &moved,
                    flow: Flow::Stay,
                }),
            ),
        ];
        for (name, outcome) in &refusals {
            assert!(
                matches!(outcome, PropertyOutcome::Violated(_)),
                "`{name}` answered {outcome:?} to the observation that must \
                 break it — a property that cannot fail is decoration in the \
                 acceptance set of every component that includes it, and \
                 NotApplicable is not a pass either"
            );
        }

        let module = include_str!("property.rs")
            .split_once("pub mod properties {")
            .expect("this file declares the shipped properties")
            .1;
        assert!(
            module.contains("selection_is_always_in_range"),
            "the scan read nothing recognisable, so its count means nothing"
        );
        let shipped = module.matches("\n    pub fn ").count();
        assert_eq!(
            shipped,
            refusals.len(),
            "`properties` ships {shipped} properties and this test witnesses \
             {}. Adding a property here means adding the observation it \
             refuses — otherwise nobody has ever seen it fail.",
            refusals.len()
        );
    }

    /// The weaker escape claim admits a two-stage exit, and still refuses a
    /// state with no way out.
    #[test]
    fn the_weaker_escape_claim_admits_a_sub_mode() {
        let p = escape_always_leaves_something();
        let open = view(vec![Row::new("a", ":w").selected()]);
        let closed = view(vec![Row::new("a", "").selected()]);
        assert_eq!(
            p.check(&Observation::Transition {
                from: &open,
                key: Key::Esc,
                to: &closed,
                flow: Flow::Stay,
            }),
            PropertyOutcome::Held
        );
        assert!(refusal(p.check(&Observation::Transition {
            from: &open,
            key: Key::Esc,
            to: &open,
            flow: Flow::Stay,
        }))
        .contains("no way out"));
    }
}
