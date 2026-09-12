//! The key vocabulary an acceptance corpus is written in.
//!
//! **Deliberately not `crossterm::event::KeyCode`.** A host decodes its own
//! terminal library's events and maps them here, which costs one small match
//! and buys three things: this crate stays a leaf (no terminal backend in a
//! consumer's dependency closure), a component can be driven with no terminal
//! at all, and a recorded corpus survives its host swapping terminal libraries
//! — the keys a component must answer for are not a property of who decoded
//! them.
//!
//! The set is small on purpose. It covers what an interactive component can
//! meaningfully be held to: move, adjust, commit, back out, type, and the
//! modifier that changes what those mean. [`Key::Other`] carries an unmapped
//! host key when its arrival matters, such as a confirmation that declines on
//! any key except an explicit yes.

/// One key press, as a component sees it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[non_exhaustive]
pub enum Key {
    /// Move the cursor up.
    Up,
    /// Move the cursor down.
    Down,
    /// Adjust down / step back.
    Left,
    /// Adjust up / step forward.
    Right,
    /// Commit — what a form calls "apply" and a menu calls "pick".
    Enter,
    /// Back out. The one key every component in this line must answer for; see
    /// `properties::escape_always_closes_without_applying`.
    Esc,
    /// Delete backwards in a text field.
    Backspace,
    /// Next field or pane.
    Tab,
    /// Previous field or pane.
    BackTab,
    /// Jump to the start.
    Home,
    /// Jump to the end.
    End,
    /// Scroll back a screenful.
    PageUp,
    /// Scroll forward a screenful.
    PageDown,
    /// A printable character.
    Char(char),
    /// A character with the control modifier held.
    Ctrl(char),
    /// A host key outside this vocabulary. It still reached the component:
    /// the component decides whether to ignore it or, for example, decline a
    /// pending confirmation. It is neither a character nor an escape key.
    Other,
}

impl Key {
    /// The keys most row-and-value components need, in a stable order.
    ///
    /// A convenience for the common alphabet, not a rule: a component with a
    /// text field passes its own set including the `Char`s it accepts, and one
    /// with no list passes fewer. The order is fixed so an exploration report
    /// reads the same on every run.
    #[must_use]
    pub fn navigation() -> Vec<Self> {
        vec![
            Self::Up,
            Self::Down,
            Self::Left,
            Self::Right,
            Self::Enter,
            Self::Esc,
        ]
    }

    /// Whether this key is the operator's way OUT.
    ///
    /// Named rather than matched inline because the escape property is the one
    /// every component shares, and a second opinion about what counts as an
    /// escape is how a component ends up with a state you cannot leave.
    #[must_use]
    pub fn is_escape(self) -> bool {
        matches!(self, Self::Esc)
    }
}

impl core::fmt::Display for Key {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Up => f.write_str("Up"),
            Self::Down => f.write_str("Down"),
            Self::Left => f.write_str("Left"),
            Self::Right => f.write_str("Right"),
            Self::Enter => f.write_str("Enter"),
            Self::Esc => f.write_str("Esc"),
            Self::Backspace => f.write_str("Backspace"),
            Self::Tab => f.write_str("Tab"),
            Self::BackTab => f.write_str("BackTab"),
            Self::Home => f.write_str("Home"),
            Self::End => f.write_str("End"),
            Self::PageUp => f.write_str("PageUp"),
            Self::PageDown => f.write_str("PageDown"),
            Self::Char(c) => write!(f, "'{c}'"),
            Self::Ctrl(c) => write!(f, "Ctrl-{c}"),
            Self::Other => f.write_str("Other"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_key_path_reads_as_the_operator_typed_it() {
        let path = [Key::Down, Key::Right, Key::Char('x'), Key::Ctrl('s')];
        let rendered: Vec<String> = path.iter().map(ToString::to_string).collect();
        assert_eq!(rendered.join(" "), "Down Right 'x' Ctrl-s");
    }

    // GUARD: key::tests::every_named_key_has_a_distinct_rendering_and_escape_identity — this is a guard; tests/mutations.rs must show it red.
    #[test]
    fn every_named_key_has_a_distinct_rendering_and_escape_identity() {
        let keys = [
            (Key::Up, "Up"),
            (Key::Down, "Down"),
            (Key::Left, "Left"),
            (Key::Right, "Right"),
            (Key::Enter, "Enter"),
            (Key::Esc, "Esc"),
            (Key::Backspace, "Backspace"),
            (Key::Tab, "Tab"),
            (Key::BackTab, "BackTab"),
            (Key::Home, "Home"),
            (Key::End, "End"),
            (Key::PageUp, "PageUp"),
            (Key::PageDown, "PageDown"),
            (Key::Char('x'), "'x'"),
            (Key::Ctrl('x'), "Ctrl-x"),
            (Key::Other, "Other"),
        ];
        for (key, name) in keys {
            assert_eq!(key.to_string(), name);
            assert_eq!(key.is_escape(), key == Key::Esc);
            for (other, other_name) in keys {
                assert_eq!(key == other, name == other_name);
            }
        }
    }

    /// Escape is asked about through one predicate, so a component and the
    /// property that judges it cannot disagree about what "out" means.
    #[test]
    fn only_escape_is_the_way_out() {
        assert!(Key::Esc.is_escape());
        for other in [Key::Enter, Key::Up, Key::Char('q'), Key::Ctrl('c')] {
            assert!(!other.is_escape(), "{other} is not the escape key");
        }
    }
}
