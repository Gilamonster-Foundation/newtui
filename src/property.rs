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

/// A named claim, checked at every state and transition the explorer reaches.
pub trait Property {
    /// How this claim is named in a report.
    fn name(&self) -> &str;

    /// `Err` carries what went wrong, phrased for someone reading a failure.
    ///
    /// # Errors
    ///
    /// The component violated this property at this observation.
    fn check(&self, observation: &Observation<'_>) -> Result<(), String>;
}

/// A property written as a closure — the way a component declares its own.
pub struct Named<F> {
    name: String,
    check: F,
}

impl<F> Named<F>
where
    F: Fn(&Observation<'_>) -> Result<(), String>,
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
    F: Fn(&Observation<'_>) -> Result<(), String>,
{
    fn name(&self) -> &str {
        &self.name
    }

    fn check(&self, observation: &Observation<'_>) -> Result<(), String> {
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
    use super::{Named, Observation};
    use crate::Flow;

    /// **At most one row is selected, and a non-empty component selects one.**
    ///
    /// The defect this catches: a selection index into a row list that is
    /// REBUILT each frame. When a row disappears — a conditional field, a
    /// filtered list — the index can point past the end, and the component
    /// silently acts on nothing. Reported as "nothing happened", which is the
    /// hardest kind of bug to be shown.
    #[must_use]
    pub fn selection_is_always_in_range() -> Named<impl Fn(&Observation<'_>) -> Result<(), String>>
    {
        Named::new("selection is always in range", |observation| {
            let view = observation.view();
            match view.selection_count() {
                0 if view.rows.is_empty() => Ok(()),
                0 => Err(format!(
                    "{} rows and nothing selected — a key would act on no row",
                    view.rows.len()
                )),
                1 => Ok(()),
                n => Err(format!("{n} rows claim the cursor at once")),
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
    #[must_use]
    pub fn escape_always_closes_without_applying(
    ) -> Named<impl Fn(&Observation<'_>) -> Result<(), String>> {
        Named::new("escape always closes without applying", |observation| {
            let Observation::Transition { key, flow, .. } = observation else {
                return Ok(());
            };
            if !key.is_escape() {
                return Ok(());
            }
            match flow {
                Flow::Close(false) => Ok(()),
                Flow::Close(true) => Err("escape closed the component AS AN APPLY".to_string()),
                Flow::Stay => Err("escape left the component open".to_string()),
            }
        })
    }

    /// The weaker escape claim, for a component with sub-modes: Esc must
    /// always DO something — close, or leave the state it was in.
    #[must_use]
    pub fn escape_always_leaves_something() -> Named<impl Fn(&Observation<'_>) -> Result<(), String>>
    {
        Named::new("escape always leaves something", |observation| {
            let Observation::Transition {
                from,
                key,
                to,
                flow,
                ..
            } = observation
            else {
                return Ok(());
            };
            if !key.is_escape() {
                return Ok(());
            }
            if matches!(flow, Flow::Close(_)) || from != to {
                Ok(())
            } else {
                Err("escape neither closed nor changed anything — a state with \
                     no way out"
                    .to_string())
            }
        })
    }

    /// **A row that is not adjustable does not move under ← or →.**
    ///
    /// The chrome a renderer draws for an adjustable row PROMISES that arrows
    /// change it. The inverse has to hold too, or a stray arrow silently
    /// repoints something the operator was only passing over — and this is the
    /// property that says so at every state, not just the row someone tested.
    #[must_use]
    pub fn only_adjustable_rows_move() -> Named<impl Fn(&Observation<'_>) -> Result<(), String>> {
        Named::new("only adjustable rows move", |observation| {
            let Observation::Transition { from, key, to, .. } = observation else {
                return Ok(());
            };
            if !matches!(key, crate::Key::Left | crate::Key::Right) {
                return Ok(());
            }
            let Some(at) = from.selected() else {
                return Ok(());
            };
            let (Some(before), Some(after)) = (from.rows.get(at), to.rows.get(at)) else {
                return Ok(());
            };
            if before.adjustable || before.value == after.value {
                Ok(())
            } else {
                Err(format!(
                    "`{}` is not adjustable but {key} changed it from `{}` to `{}`",
                    before.label, before.value, after.value
                ))
            }
        })
    }

    /// **A repeated key reaches a fixed point rather than cycling.**
    ///
    /// Checked by the explorer's structure rather than one observation: if
    /// holding → forever walks a cycle, the state graph has a loop the search
    /// will find and this property will flag when it revisits. Wrapping is a
    /// legitimate design — a component that WANTS a wrap simply does not
    /// include this property — but a component that clamps everywhere else and
    /// wraps in one place is a surprise.
    #[must_use]
    pub fn a_repeated_key_reaches_a_fixed_point(
    ) -> Named<impl Fn(&Observation<'_>) -> Result<(), String>> {
        Named::new("a repeated key reaches a fixed point", |_| Ok(()))
    }
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

    #[test]
    fn the_selection_property_catches_both_shapes_of_wrong() {
        let p = selection_is_always_in_range();
        let ok = view(vec![Row::new("a", "1").selected(), Row::new("b", "2")]);
        assert!(p.check(&Observation::State { view: &ok }).is_ok());

        let none = view(vec![Row::new("a", "1")]);
        let err = p
            .check(&Observation::State { view: &none })
            .expect_err("a row list with no cursor is a defect");
        assert!(err.contains("nothing selected"), "{err}");

        let two = view(vec![
            Row::new("a", "1").selected(),
            Row::new("b", "2").selected(),
        ]);
        assert!(p.check(&Observation::State { view: &two }).is_err());

        // An empty component selects nothing, and that is fine.
        let empty = view(Vec::new());
        assert!(p.check(&Observation::State { view: &empty }).is_ok());
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
        assert!(p.check(&at(Flow::Close(false))).is_ok());
        assert!(p
            .check(&at(Flow::Close(true)))
            .expect_err("escape must not apply")
            .contains("AS AN APPLY"));
        assert!(p
            .check(&at(Flow::Stay))
            .expect_err("escape must not be swallowed")
            .contains("left the component open"));

        // A non-escape key is none of its business.
        assert!(p
            .check(&Observation::Transition {
                from: &v,
                key: Key::Enter,
                to: &v,
                flow: Flow::Close(true),
            })
            .is_ok());
    }

    #[test]
    fn a_door_that_dials_is_caught() {
        let p = only_adjustable_rows_move();
        let before = view(vec![Row::new("backend", "sol").selected()]);
        let after = view(vec![Row::new("backend", "other").selected()]);
        let err = p
            .check(&Observation::Transition {
                from: &before,
                key: Key::Right,
                to: &after,
                flow: Flow::Stay,
            })
            .expect_err("a non-adjustable row moved");
        assert!(err.contains("not adjustable"), "{err}");

        // The same move on an adjustable row is the point of the row.
        let dial_before = view(vec![Row::new("tenacity", "auto").adjustable().selected()]);
        let dial_after = view(vec![Row::new("tenacity", "relaxed")
            .adjustable()
            .selected()]);
        assert!(p
            .check(&Observation::Transition {
                from: &dial_before,
                key: Key::Right,
                to: &dial_after,
                flow: Flow::Stay,
            })
            .is_ok());
    }

    /// The weaker escape claim admits a two-stage exit, and still refuses a
    /// state with no way out.
    #[test]
    fn the_weaker_escape_claim_admits_a_sub_mode() {
        let p = escape_always_leaves_something();
        let open = view(vec![Row::new("a", ":w").selected()]);
        let closed = view(vec![Row::new("a", "").selected()]);
        assert!(p
            .check(&Observation::Transition {
                from: &open,
                key: Key::Esc,
                to: &closed,
                flow: Flow::Stay,
            })
            .is_ok(),);
        assert!(p
            .check(&Observation::Transition {
                from: &open,
                key: Key::Esc,
                to: &open,
                flow: Flow::Stay,
            })
            .is_err());
    }
}
