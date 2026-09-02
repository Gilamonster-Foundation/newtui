//! What a component SHOWS, as data.
//!
//! A view is not a rendering. It is the description a renderer would draw and
//! a property would judge — plain owned data, comparable, printable, and free
//! of any terminal library. That is what lets the same component be drawn by
//! ratatui here, by something else in another host, and by nothing at all in a
//! headless test that only wants to know what it would have said.
//!
//! The shape is a title, rows, and a footer, because that is what the
//! components this crate was extracted from actually are: a bordered block, a
//! list of label/value pairs with one of them selected, and a hint line. A
//! component that needs a genuinely different shape should say so rather than
//! bend into this one — but three panels bent into it exactly.

/// One line of a component's view.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Row {
    /// What the row is called.
    pub label: String,
    /// What it currently holds.
    pub value: String,
    /// The subordinate column: what the value MEANS, what it was, or why the
    /// row will not move. Empty when there is nothing to add.
    pub note: String,
    /// The cursor is on this row. At most one row in a view may be selected,
    /// and `properties::selection_is_always_in_range` is what holds that true.
    pub selected: bool,
    /// ←→ changes this row's value.
    ///
    /// The distinction is behavioural, not cosmetic: a row that is not
    /// adjustable must not move under a horizontal key, and a renderer uses
    /// the same flag to decide whether to draw the dial chrome that PROMISES
    /// it will. `properties::only_adjustable_rows_move` checks the first;
    /// drawing the chrome on a row that does not move is how an operator
    /// learns not to trust it.
    pub adjustable: bool,
}

impl Row {
    /// A plain row: a label and what it holds.
    pub fn new(label: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            value: value.into(),
            ..Self::default()
        }
    }

    /// ←→ changes this row.
    #[must_use]
    pub fn adjustable(mut self) -> Self {
        self.adjustable = true;
        self
    }

    /// The cursor is here.
    #[must_use]
    pub fn selected(mut self) -> Self {
        self.selected = true;
        self
    }

    /// The subordinate column.
    #[must_use]
    pub fn note(mut self, note: impl Into<String>) -> Self {
        self.note = note.into();
        self
    }
}

/// Everything a component currently shows.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct View {
    /// What the component is called.
    pub title: String,
    /// Its rows, in display order.
    pub rows: Vec<Row>,
    /// The hint or status line. Part of the view because it is part of what
    /// the operator is told — a component that stops explaining itself has
    /// changed behaviour, and a property can say so.
    pub footer: String,
}

impl View {
    #[must_use]
    /// An empty view with a title.
    pub fn titled(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            ..Self::default()
        }
    }

    #[must_use]
    /// Append a row.
    pub fn row(mut self, row: Row) -> Self {
        self.rows.push(row);
        self
    }

    #[must_use]
    /// Set the hint or status line.
    pub fn footer(mut self, footer: impl Into<String>) -> Self {
        self.footer = footer.into();
        self
    }

    /// Which row the cursor is on, if any.
    #[must_use]
    pub fn selected(&self) -> Option<usize> {
        self.rows.iter().position(|row| row.selected)
    }

    /// How many rows claim the cursor. Anything but 0 or 1 is a defect, which
    /// is why this is exposed rather than folded into [`Self::selected`] —
    /// `position` would hide a second one.
    #[must_use]
    pub fn selection_count(&self) -> usize {
        self.rows.iter().filter(|row| row.selected).count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_view_is_built_by_description() {
        let view = View::titled("settings")
            .row(Row::new("tenacity", "auto").adjustable().selected())
            .row(Row::new("backend", "sol").note("Enter: choose"))
            .footer("↑↓ select");
        assert_eq!(view.title, "settings");
        assert_eq!(view.selected(), Some(0));
        assert_eq!(view.rows[1].note, "Enter: choose");
        assert!(!view.rows[1].adjustable, "a door is not a dial");
    }

    /// `selected()` reports the FIRST, so a second claimant would hide behind
    /// it. The count is what a property checks, and this is why it exists.
    #[test]
    fn two_selected_rows_are_visible_as_a_count_not_a_position() {
        let view = View::titled("broken")
            .row(Row::new("a", "1").selected())
            .row(Row::new("b", "2").selected());
        assert_eq!(view.selected(), Some(0), "position hides the second");
        assert_eq!(view.selection_count(), 2, "the count does not");
    }
}
