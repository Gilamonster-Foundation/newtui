//! The seam: what a component must declare to be driven in isolation.

use crate::{Key, View};

/// What one key did to a component.
///
/// Two arms, deliberately. A component closes because the operator ACCEPTED
/// something or because they did not, and the boolean is that distinction —
/// not a success code. What an acceptance MEANS is the host's business; a
/// component that decided would be performing an effect, which is exactly what
/// this crate keeps out of components.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Flow {
    /// Still open.
    Stay,
    /// Closed; `true` when the operator explicitly applied.
    Close(bool),
}

impl Flow {
    /// Did this close WITHOUT applying — the shape an escape must produce?
    #[must_use]
    pub fn is_cancel(self) -> bool {
        matches!(self, Self::Close(false))
    }
}

/// A value that is equal for two states iff they behave the same from here on.
///
/// The whole search rests on this. The explorer walks STATES, not paths, and
/// what collapses `4^depth` sequences into a few dozen states is recognising
/// that two of them are the same. Get it too coarse and the search skips real
/// states; too fine and it never terminates.
///
/// The safe default is [`Fingerprint::of_view`]: a component whose future
/// behaviour is fully determined by what it currently shows — which most
/// row-and-value components are — can use it and be right. A component with
/// state the view does NOT show (a pending edit buffer, a sub-mode, a
/// remembered previous value) must fold that in with [`Fingerprint::and`], or
/// the explorer will treat two different states as one and miss whatever only
/// the hidden half can reach.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Fingerprint(String);

impl Fingerprint {
    /// Fingerprint a component by everything it displays.
    #[must_use]
    pub fn of_view(view: &View) -> Self {
        let mut acc = format!("{}\u{1f}{}", view.title, view.footer);
        for row in &view.rows {
            // Every field: a row that differs only in its note is showing the
            // operator something different, and a search that treated those as
            // one state could not tell you the note ever changed.
            use core::fmt::Write as _;
            let _ = write!(
                acc,
                "\u{1e}{}\u{1f}{}\u{1f}{}\u{1f}{}\u{1f}{}",
                row.label, row.value, row.note, row.selected, row.adjustable
            );
        }
        Self(acc)
    }

    /// Start from something other than a view.
    #[must_use]
    pub fn of(seed: impl core::fmt::Display) -> Self {
        Self(seed.to_string())
    }

    /// Fold in state the view does not show.
    #[must_use]
    pub fn and(mut self, more: impl core::fmt::Display) -> Self {
        self.0.push('\u{1d}');
        self.0.push_str(&more.to_string());
        self
    }
}

impl core::fmt::Display for Fingerprint {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        // Control characters are separators, not content; a report that printed
        // them raw would be unreadable in exactly the moment it matters.
        f.write_str(
            &self
                .0
                .replace('\u{1e}', " | ")
                .replace('\u{1f}', "/")
                .replace('\u{1d}', " + "),
        )
    }
}

/// A user interface element that can be driven in isolation.
///
/// Three declarations, and no I/O among them: handle one key, say what you
/// show, say what makes you the same as another state. Everything the harness
/// does is built from those.
pub trait Component {
    /// Apply one key.
    fn handle(&mut self, key: Key) -> Flow;

    /// What the component currently shows.
    ///
    /// Must be PURE — the explorer calls it after every transition, and a
    /// `view` with a side effect would make the search itself the thing under
    /// test.
    fn view(&self) -> View;

    /// What makes two states the same. See [`Fingerprint`].
    fn fingerprint(&self) -> Fingerprint {
        Fingerprint::of_view(&self.view())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;

    #[test]
    fn a_cancel_is_the_shape_an_escape_must_produce() {
        assert!(Flow::Close(false).is_cancel());
        assert!(!Flow::Close(true).is_cancel());
        assert!(!Flow::Stay.is_cancel());
    }

    /// Every displayed field is in the fingerprint. A search that ignored one
    /// would silently merge two states an operator can tell apart.
    #[test]
    fn the_fingerprint_notices_every_displayed_difference() {
        let base = View::titled("t").row(Row::new("a", "1")).footer("f");
        let same = Fingerprint::of_view(&base);

        let variants = [
            View::titled("OTHER").row(Row::new("a", "1")).footer("f"),
            View::titled("t").row(Row::new("OTHER", "1")).footer("f"),
            View::titled("t").row(Row::new("a", "OTHER")).footer("f"),
            View::titled("t")
                .row(Row::new("a", "1").note("n"))
                .footer("f"),
            View::titled("t")
                .row(Row::new("a", "1").selected())
                .footer("f"),
            View::titled("t")
                .row(Row::new("a", "1").adjustable())
                .footer("f"),
            View::titled("t").row(Row::new("a", "1")).footer("OTHER"),
            View::titled("t")
                .row(Row::new("a", "1"))
                .row(Row::new("b", "2"))
                .footer("f"),
        ];
        for (i, variant) in variants.iter().enumerate() {
            assert_ne!(
                Fingerprint::of_view(variant),
                same,
                "variant {i} is a different state and must fingerprint differently"
            );
        }
        assert_eq!(
            Fingerprint::of_view(&View::titled("t").row(Row::new("a", "1")).footer("f")),
            same,
            "the same view is the same state"
        );
    }

    /// Hidden state folds in, and two states that LOOK alike stay apart.
    #[test]
    fn hidden_state_can_be_folded_in() {
        let view = View::titled("t").row(Row::new("a", "1"));
        let editing = Fingerprint::of_view(&view).and("buffer=ab");
        let idle = Fingerprint::of_view(&view).and("buffer=");
        assert_ne!(editing, idle, "the same view, two different states");
    }

    /// Separators do not leak into a report a person has to read.
    #[test]
    fn a_fingerprint_prints_legibly() {
        let printed = Fingerprint::of_view(&View::titled("t").row(Row::new("a", "1"))).to_string();
        assert!(!printed.contains('\u{1f}'), "{printed:?}");
        assert!(printed.contains("a/1"), "{printed:?}");
    }
}
