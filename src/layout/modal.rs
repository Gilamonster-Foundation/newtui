//! The height policy of a modal viewport: grow, shrink, zoom, and drag.
//!
//! No terminal and no I/O. The host owns the screen and the layout, reports
//! the height it actually granted, and asks [`ModalSize::apply`] what height
//! to request next. Every modal a host opens (settings, a diff viewer, a
//! Markdown pane) then sizes the same way under the same keys.
//!
//! Ported from newt-agent's `newt-tui/src/modal_size.rs` with identical semantics.

/// The operator's sizing intents. The host decides which keys or pointer
/// gestures produce them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SizeKey {
    /// One row taller than the granted height.
    Grow,
    /// One row shorter than the granted height, never below [`MIN_ROWS`].
    Shrink,
    /// Fill the screen; a second zoom returns to the height before zooming.
    Zoom,
    /// Exactly this many rows, never below [`MIN_ROWS`]: the target of a
    /// pointer drag on the modal's edge, decoded by the host.
    To(u16),
}

/// Border, one content row, the hint line, border: the least a modal can be
/// and still say how to leave.
pub const MIN_ROWS: u16 = 4;

/// The requested height while zoomed: a fill request the host clamps to the
/// screen.
pub const FILL: u16 = u16::MAX;

/// A modal's requested height, plus the height to restore when zoom ends.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ModalSize {
    /// The height to ask the host for. [`FILL`] while zoomed.
    requested: u16,
    /// The granted height to return to when zoom is toggled off.
    before_zoom: Option<u16>,
}

impl ModalSize {
    /// Start by requesting `requested` rows, raised to [`MIN_ROWS`].
    #[must_use]
    pub fn new(requested: u16) -> Self {
        Self {
            requested: requested.max(MIN_ROWS),
            before_zoom: None,
        }
    }

    /// The height to ask the host for; [`FILL`] while zoomed.
    #[must_use]
    pub fn requested(self) -> u16 {
        self.requested
    }

    /// Whether a second [`SizeKey::Zoom`] would restore an earlier height.
    #[must_use]
    pub fn zoomed(self) -> bool {
        self.before_zoom.is_some()
    }

    /// Apply one intent against the height the host actually `granted`
    /// (which the screen may have clamped). Returns the new request when it
    /// changed.
    ///
    /// Grow and Shrink step from what is ON SCREEN, not from the request, so
    /// holding Shift-Up at full height does not bank rows the operator then
    /// has to shrink back through. Grow, Shrink and To all leave zoom.
    pub fn apply(&mut self, key: SizeKey, granted: u16) -> Option<u16> {
        let before = self.requested;
        match key {
            SizeKey::Grow => {
                self.before_zoom = None;
                self.requested = granted.saturating_add(1).max(MIN_ROWS);
            }
            SizeKey::Shrink => {
                self.before_zoom = None;
                self.requested = granted.saturating_sub(1).max(MIN_ROWS);
            }
            SizeKey::To(rows) => {
                self.before_zoom = None;
                self.requested = rows.max(MIN_ROWS);
            }
            SizeKey::Zoom => {
                if let Some(previous) = self.before_zoom.take() {
                    self.requested = previous;
                } else {
                    self.before_zoom = Some(granted.max(MIN_ROWS));
                    self.requested = FILL;
                }
            }
        }
        (self.requested != before).then_some(self.requested)
    }
}
