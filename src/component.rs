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
///
/// # Why this holds STRUCTURE and not a string
///
/// It used to be one `String`: the view's fields joined with U+001F and U+001E,
/// [`Fingerprint::and`] appending U+001D, with no escaping, no length prefix
/// and no type tag. That is a hand-rolled canonical encoding, and it had the
/// flaw hand-rolled canonical encodings have — an AMBIGUOUS one. Two different
/// states could serialise to the same bytes:
///
/// ```text
/// View::titled("a\u{1f}b").footer("c")  ==  View::titled("a").footer("b\u{1f}c")
/// of("x").and("y").and("z")            ==  of("x").and("y\u{1d}z")
/// ```
///
/// That is not a probabilistic hash collision a wider digest would fix, and it
/// is not fixed by choosing rarer separators — that only moves which inputs
/// collide. It was DETERMINISTIC ambiguity in the one value the whole search
/// rests on: `seen.insert(component.fingerprint())` is what decides whether a
/// reached state is recorded and enqueued, so a collision drops that state and
/// everything reachable only through it — while the queue empties normally and
/// the report says `exhausted: true`. The false-completeness class, in the
/// DEFAULT path.
///
/// So identity is structural: the [`View`] itself, or the caller's opaque seed,
/// plus each `and` as its own element. Two arms that never compare equal, and a
/// `Vec` whose elements cannot run together. There is no encoding to get wrong.
/// [`Display`](core::fmt::Display) still renders it legibly, and being lossy
/// costs nothing now that the printed form is not the identity. If bytes are
/// ever needed for persistence, encode them AFTERWARDS, with explicit tags and
/// length prefixes.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Fingerprint {
    base: FingerprintBase,
    /// State the view does not show, in the order it was folded in. A `Vec`
    /// and not a joined string: `of("x").and("y")` must not equal
    /// `of("x\u{1d}y")`.
    extra: Vec<String>,
}

/// What a fingerprint is built from.
///
/// Two arms so that a view-derived identity and a caller-supplied one are
/// different KINDS of value, not two strings that a control character in a row
/// label could make equal.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum FingerprintBase {
    /// Everything the component displays.
    View(View),
    /// A caller's own summary of what makes this state itself.
    Opaque(String),
}

impl Fingerprint {
    /// Fingerprint a component by everything it displays.
    ///
    /// Every field of every row, because a row that differs only in its note is
    /// showing the operator something different, and a search that treated
    /// those as one state could not tell you the note ever changed.
    #[must_use]
    pub fn of_view(view: &View) -> Self {
        Self {
            base: FingerprintBase::View(view.clone()),
            extra: Vec::new(),
        }
    }

    /// Start from something other than a view.
    #[must_use]
    pub fn of(seed: impl core::fmt::Display) -> Self {
        Self {
            base: FingerprintBase::Opaque(seed.to_string()),
            extra: Vec::new(),
        }
    }

    /// Fold in state the view does not show.
    ///
    /// Each call is its own element. An accumulator that CONCATENATED is the
    /// half of the old flaw that needs no control character to bite: it is
    /// handed to every implementer, and `.and(indices.join(""))` alone would
    /// merge `[1, 23]` with `[12, 3]`.
    #[must_use]
    pub fn and(mut self, more: impl core::fmt::Display) -> Self {
        self.extra.push(more.to_string());
        self
    }
}

impl core::fmt::Display for Fingerprint {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        // Legibility only. This rendering is AMBIGUOUS — `a/b` + `c` prints the
        // same as `a` + `b/c` — and that is now harmless, because it is not
        // what two fingerprints are compared by. A report a person reads in the
        // moment it matters should not be full of control characters.
        match &self.base {
            FingerprintBase::Opaque(seed) => f.write_str(seed)?,
            FingerprintBase::View(view) => {
                write!(f, "{}/{}", view.title, view.footer)?;
                for row in &view.rows {
                    write!(
                        f,
                        " | {}/{}/{}/{}/{}",
                        row.label, row.value, row.note, row.selected, row.adjustable
                    )?;
                }
            }
        }
        for more in &self.extra {
            write!(f, " + {more}")?;
        }
        Ok(())
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

    /// **No two distinct states share a fingerprint** — the identity is
    /// structural, so there is no encoding for a separator to escape from.
    ///
    /// Every pair below was EQUAL while the fingerprint was one joined string,
    /// and `seen.insert(component.fingerprint())` is what decides whether a
    /// reached state is recorded and enqueued — so each of them silently
    /// dropped a state and everything reachable only through it, in a walk that
    /// went on to report `exhausted: true`.
    ///
    /// The pairs are deliberately not all built from the OLD separators. The
    /// defect was never those particular characters: any flattening of a
    /// structure into one string has a pair that collides in it, so picking
    /// rarer separators relocates the bug rather than fixing it. These pairs
    /// collide under the old encoding, under the legible rendering that
    /// replaced it, and under any other join a future refactor might reach for.
    // GUARD: component::tests::no_two_distinct_states_can_share_a_fingerprint — this is a guard; tests/mutations.rs must show it red.
    #[test]
    fn no_two_distinct_states_can_share_a_fingerprint() {
        let pairs = [
            // The old field separator, inside a title and inside a footer.
            (
                Fingerprint::of_view(&View::titled("a\u{1f}b").footer("c")),
                Fingerprint::of_view(&View::titled("a").footer("b\u{1f}c")),
            ),
            // The same split under the rendering that replaced it.
            (
                Fingerprint::of_view(&View::titled("a/b").footer("c")),
                Fingerprint::of_view(&View::titled("a").footer("b/c")),
            ),
            // A whole ROW impersonated from inside a footer.
            (
                Fingerprint::of_view(&View::titled("t").row(Row::new("a", "1")).footer("f")),
                Fingerprint::of_view(&View::titled("t").footer("f | a/1//false/false")),
            ),
            // The unframed accumulator: one fold, or two.
            (
                Fingerprint::of("x").and("y").and("z"),
                Fingerprint::of("x").and("y\u{1d}z"),
            ),
            (
                Fingerprint::of("x").and("y").and("z"),
                Fingerprint::of("x").and("y + z"),
            ),
            // No control character needed at all — `.and(indices.join(""))`.
            (
                Fingerprint::of("i").and("1").and("23"),
                Fingerprint::of("i").and("12").and("3"),
            ),
            // A view-derived identity is not a caller's string that renders the
            // same way. Different KINDS of claim, and the arms enforce it.
            (
                Fingerprint::of_view(&View::titled("t").footer("f")),
                Fingerprint::of("t/f"),
            ),
        ];
        for (n, (left, right)) in pairs.iter().enumerate() {
            assert_ne!(
                left, right,
                "pair {n} is two different states and must not merge into one: \
                 a fingerprint collision drops the second state and everything \
                 reachable only through it, and the walk still says exhausted"
            );
        }
    }

    /// The review's reproduction, verbatim: a separator that is CONTENT.
    #[test]
    fn a_separator_in_content_collides_two_distinct_views() {
        let a = View::titled("a\u{1f}b").footer("c");
        let b = View::titled("a").footer("b\u{1f}c");
        assert_ne!(Fingerprint::of_view(&a), Fingerprint::of_view(&b));
    }

    /// The other entry point, which the view repro does not cover: a seed that
    /// impersonates a fold. A refactor could re-break just this half.
    #[test]
    fn hidden_state_can_impersonate_a_different_split() {
        assert_ne!(Fingerprint::of("x\u{1d}y"), Fingerprint::of("x").and("y"));
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
