//! Cursor-driven navigation across two host-owned surfaces.
//!
//! The correspondence is monotone numeric data, never source text. Keyboard
//! movement maps the active cursor once, then keeps both cursors in their logical
//! windows. Unequal regions need not have identically mapped window tops. Focus
//! changes never feed a rounded counterpart back into the original selection.

use core::ops::Range;

use crate::{Component, Flow, Key, Row, View};

/// One of the two surfaces.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PaneSide {
    /// The first surface, conventionally source or old text.
    First,
    /// The second surface, conventionally preview or new text.
    Second,
}

impl PaneSide {
    const fn index(self) -> usize {
        match self {
            Self::First => 0,
            Self::Second => 1,
        }
    }

    /// The other surface.
    #[must_use]
    pub const fn other(self) -> Self {
        match self {
            Self::First => Self::Second,
            Self::Second => Self::First,
        }
    }
}

/// How active cursor movement affects the other surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LinkMode {
    /// Follow the supplied regions, with explicit gap/fallback status.
    Locked,
    /// Align the endpoints of the entire surface counts.
    Proportional,
    /// Leave the other cursor and window unchanged.
    Unlinked,
}

/// Half-open corresponding ranges. An empty range denotes a boundary.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Region {
    /// Range on the first surface.
    pub first: Range<usize>,
    /// Range on the second surface.
    pub second: Range<usize>,
}

impl Region {
    fn on(&self, side: PaneSide) -> &Range<usize> {
        match side {
            PaneSide::First => &self.first,
            PaneSide::Second => &self.second,
        }
    }
}

/// Why a host's numeric correspondence is outside the accepted domain.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum CorrespondenceError {
    /// A range is reversed or extends beyond its declared surface count.
    Range {
        /// Index in the supplied region order.
        region: usize,
        /// Surface containing the invalid range.
        side: PaneSide,
    },
    /// Both ranges are empty, so the region corresponds to no row.
    EmptyRegion {
        /// Index in the supplied region order.
        region: usize,
    },
    /// Ranges overlap or move backwards on this surface.
    Order {
        /// Index in the supplied region order.
        region: usize,
        /// Surface whose monotone order was violated.
        side: PaneSide,
    },
}

impl core::fmt::Display for CorrespondenceError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Range { region, side } => {
                write!(f, "region {region} has an invalid {side:?} range")
            }
            Self::EmptyRegion { region } => write!(f, "region {region} is empty on both surfaces"),
            Self::Order { region, side } => {
                write!(f, "region {region} overlaps or reverses the {side:?} order")
            }
        }
    }
}

impl core::error::Error for CorrespondenceError {}

/// Validated surface counts and monotone, nonoverlapping region order.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Correspondence {
    lengths: [usize; 2],
    regions: Vec<Region>,
}

impl Correspondence {
    /// Validate the caller's order without sorting or renumbering regions.
    /// Crossing moved-block correspondences are outside this monotone domain.
    ///
    /// # Errors
    /// Returns the original region index and invalid range/order condition.
    pub fn new(lengths: [usize; 2], regions: Vec<Region>) -> Result<Self, CorrespondenceError> {
        let mut ends = [0; 2];
        for (index, region) in regions.iter().enumerate() {
            for side in [PaneSide::First, PaneSide::Second] {
                let range = region.on(side);
                if range.start > range.end || range.end > lengths[side.index()] {
                    return Err(CorrespondenceError::Range {
                        region: index,
                        side,
                    });
                }
                if range.start < ends[side.index()] {
                    return Err(CorrespondenceError::Order {
                        region: index,
                        side,
                    });
                }
                ends[side.index()] = range.end;
            }
            if region.first.is_empty() && region.second.is_empty() {
                return Err(CorrespondenceError::EmptyRegion { region: index });
            }
        }
        Ok(Self { lengths, regions })
    }

    /// The number of actual rows on a surface.
    #[must_use]
    pub fn len(&self, side: PaneSide) -> usize {
        self.lengths[side.index()]
    }

    /// Whether a surface contains no actual row.
    #[must_use]
    pub fn is_empty(&self, side: PaneSide) -> bool {
        self.len(side) == 0
    }

    /// The caller's validated region order.
    #[must_use]
    pub fn regions(&self) -> &[Region] {
        &self.regions
    }

    fn map(&self, side: PaneSide, cursor: usize, mode: LinkMode) -> Mapping {
        let fallback = |kind| Mapping {
            region: None,
            kind,
            target: proportional(cursor, self.len(side), self.len(side.other())),
        };
        if mode == LinkMode::Proportional {
            return fallback(MatchKind::Proportional);
        }
        let contained = self
            .regions
            .iter()
            .enumerate()
            .find(|(_, region)| region.on(side).contains(&cursor))
            .map(|(index, _)| (index, cursor, MatchKind::Region));
        let selected = contained.or_else(|| {
            self.regions
                .iter()
                .enumerate()
                .filter(|(_, region)| !region.on(side).is_empty())
                .flat_map(|(index, region)| {
                    let source = region.on(side);
                    [source.start, source.end - 1].map(|row| (index, row))
                })
                // Host order is monotone. Original indices break equidistant
                // gap ties towards the previous region, without renumbering.
                .min_by_key(|(index, row)| (cursor.abs_diff(*row), *index))
                .map(|(index, row)| (index, row, MatchKind::Gap))
        });
        let Some((index, row, kind)) = selected else {
            return fallback(MatchKind::NoCorrespondence);
        };
        let region = &self.regions[index];
        let source = region.on(side);
        let target = region.on(side.other());
        let location = if target.is_empty() {
            MappedLocation::Boundary(target.start)
        } else {
            match proportional(row - source.start, source.len(), target.len()) {
                MappedLocation::Row(relative) => MappedLocation::Row(target.start + relative),
                MappedLocation::Boundary(_) => unreachable!("nonempty destination range"),
            }
        };
        Mapping {
            region: Some(index),
            kind,
            target: location,
        }
    }
}

/// What a mapped counterpart means. A boundary must not highlight a real row.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MappedLocation {
    /// An actual zero-based row.
    Row(usize),
    /// An insertion/deletion boundary, including the end of a surface.
    Boundary(usize),
}

/// The evidence used for a counterpart location.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MatchKind {
    /// The selected row is inside a supplied nonempty source region.
    Region,
    /// The nearest actual source endpoint was used outside supplied regions.
    Gap,
    /// Entire surface counts were used in proportional mode.
    Proportional,
    /// Locked mode has no nonempty source region; counts supply the fallback.
    NoCorrespondence,
}

/// A single, directional mapping. It is never fed back on a focus change.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Mapping {
    /// Original host region index, when a region supplied the mapping.
    pub region: Option<usize>,
    /// Whether this is an exact region or an explicit fallback.
    pub kind: MatchKind,
    /// Actual counterpart row or boundary.
    pub target: MappedLocation,
}

/// The authoritative side and the last mapping produced by its movement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LinkRelation {
    /// Cursor whose movement supplied this mapping.
    pub source: PaneSide,
    /// Counterpart on the other side.
    pub mapping: Mapping,
}

/// Cursor and window state for one host surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PanePosition {
    /// Actual selected row, or no cursor on an empty surface.
    pub cursor: Option<usize>,
    /// First logical row of the window; zero for an empty surface.
    pub offset: usize,
    /// Host-provided visible row capacity; zero makes no visibility claim.
    pub height: usize,
}

/// Numeric navigation shared by source/preview and old/new surfaces.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct LinkedPanes {
    correspondence: Correspondence,
    positions: [PanePosition; 2],
    focus: PaneSide,
    mode: LinkMode,
    relation: Option<LinkRelation>,
}

impl LinkedPanes {
    /// Start on the first surface in locked mode. Zero-height windows use a
    /// one-row logical capacity for bounds, without claiming visible cells.
    #[must_use]
    pub fn new(correspondence: Correspondence, heights: [usize; 2]) -> Self {
        let positions = [PaneSide::First, PaneSide::Second].map(|side| PanePosition {
            cursor: (!correspondence.is_empty(side)).then_some(0),
            offset: 0,
            height: heights[side.index()],
        });
        let mut result = Self {
            correspondence,
            positions,
            focus: PaneSide::First,
            mode: LinkMode::Locked,
            relation: None,
        };
        result.synchronize();
        result
    }

    /// The active keyboard surface.
    #[must_use]
    pub fn focus(&self) -> PaneSide {
        self.focus
    }

    /// Current link policy.
    #[must_use]
    pub fn mode(&self) -> LinkMode {
        self.mode
    }

    /// Current state of one surface.
    #[must_use]
    pub fn position(&self, side: PaneSide) -> PanePosition {
        self.positions[side.index()]
    }

    /// Last counterpart evidence, absent in unlinked mode or with no source row.
    /// A boundary can have a clamped viewport cursor but is never an actual
    /// counterpart row. Hosts use this status when highlighting correspondence.
    #[must_use]
    pub fn relation(&self) -> Option<LinkRelation> {
        self.relation
    }

    /// The numeric domain retained by this component.
    #[must_use]
    pub fn correspondence(&self) -> &Correspondence {
        &self.correspondence
    }

    /// Change focus only. Rounded mappings never run in reverse implicitly.
    pub fn set_focus(&mut self, side: PaneSide) {
        self.focus = side;
    }

    /// Select a policy, using the active side as authority exactly once.
    /// Requesting the existing policy deliberately reasserts that authority.
    pub fn set_mode(&mut self, mode: LinkMode) {
        self.mode = mode;
        self.synchronize();
    }

    /// Move the active cursor to a bounded row and update its counterpart once.
    /// A request that leaves the cursor unchanged is a no-op, including a
    /// saturated arrow/page/home/end request after a focus change.
    pub fn select(&mut self, row: usize) {
        let length = self.correspondence.len(self.focus);
        let next = length.checked_sub(1).map(|end| row.min(end));
        if next == self.position(self.focus).cursor {
            return;
        }
        self.positions[self.focus.index()].cursor = next;
        self.keep_visible(self.focus);
        self.synchronize();
    }

    /// Resize windows without changing either selection or link evidence.
    pub fn resize(&mut self, heights: [usize; 2]) {
        for side in [PaneSide::First, PaneSide::Second] {
            self.positions[side.index()].height = heights[side.index()];
            self.keep_visible(side);
        }
    }

    /// Full bounded keyboard vocabulary, including ignored-key witnesses.
    #[must_use]
    pub fn alphabet() -> Vec<Key> {
        vec![
            Key::Up,
            Key::Down,
            Key::PageUp,
            Key::PageDown,
            Key::Home,
            Key::End,
            Key::Tab,
            Key::BackTab,
            Key::Char('l'),
            Key::Esc,
            Key::Enter,
            Key::Left,
            Key::Right,
            Key::Backspace,
            Key::Char('x'),
            Key::Ctrl('x'),
            Key::Other,
        ]
    }

    fn synchronize(&mut self) {
        self.relation = None;
        if self.mode == LinkMode::Unlinked {
            return;
        }
        let Some(cursor) = self.position(self.focus).cursor else {
            return;
        };
        let mapping = self.correspondence.map(self.focus, cursor, self.mode);
        let target = match mapping.target {
            MappedLocation::Row(row) | MappedLocation::Boundary(row) => row,
        };
        let other = self.focus.other();
        self.positions[other.index()].cursor = self
            .correspondence
            .len(other)
            .checked_sub(1)
            .map(|end| target.min(end));
        self.keep_visible(other);
        self.relation = Some(LinkRelation {
            source: self.focus,
            mapping,
        });
    }

    fn keep_visible(&mut self, side: PaneSide) {
        let position = &mut self.positions[side.index()];
        let Some(cursor) = position.cursor else {
            position.offset = 0;
            return;
        };
        let capacity = position.height.max(1);
        let max_offset = self.correspondence.len(side).saturating_sub(capacity);
        position.offset = position.offset.min(max_offset);
        if cursor < position.offset {
            position.offset = cursor;
        }
        if cursor.saturating_sub(position.offset) >= capacity {
            position.offset = cursor.saturating_sub(capacity - 1);
        }
    }
}

impl Component for LinkedPanes {
    fn handle(&mut self, key: Key) -> Flow {
        let position = self.position(self.focus);
        let cursor = position.cursor.unwrap_or(0);
        match key {
            Key::Esc => return Flow::Close(false),
            Key::Up => self.select(cursor.saturating_sub(1)),
            Key::Down => self.select(cursor.saturating_add(1)),
            Key::PageUp => self.select(cursor.saturating_sub(position.height.max(1))),
            Key::PageDown => self.select(cursor.saturating_add(position.height.max(1))),
            Key::Home => self.select(0),
            Key::End => self.select(usize::MAX),
            Key::Tab | Key::BackTab => self.set_focus(self.focus.other()),
            Key::Char('l') => self.set_mode(match self.mode {
                LinkMode::Locked => LinkMode::Proportional,
                LinkMode::Proportional => LinkMode::Unlinked,
                LinkMode::Unlinked => LinkMode::Locked,
            }),
            _ => (),
        }
        Flow::Stay
    }

    fn view(&self) -> View {
        let mut view = View::titled("Linked panes");
        for side in [PaneSide::First, PaneSide::Second] {
            let position = self.position(side);
            let mut row = Row::new(
                format!("{side:?} cursor"),
                position
                    .cursor
                    .map_or_else(|| "empty".into(), |row| row.to_string()),
            )
            .note(format!(
                "offset={} height={} count={}",
                position.offset,
                position.height,
                self.correspondence.len(side)
            ));
            row.selected = self.focus == side;
            view = view.row(row);
        }
        view = view
            .row(Row::new("Link mode", format!("{:?}", self.mode)))
            .row(Row::new("Correspondence", format!("{:?}", self.relation)));
        for (index, region) in self.correspondence.regions.iter().enumerate() {
            view = view.row(Row::new(
                format!("Region {index}"),
                format!("{:?} -> {:?}", region.first, region.second),
            ));
        }
        view.footer("arrows/page/home/end: cursor; tab: focus; l: link; Esc: close")
    }
}

fn proportional(cursor: usize, source: usize, target: usize) -> MappedLocation {
    let Some(last) = target.checked_sub(1) else {
        return MappedLocation::Boundary(0);
    };
    if source <= 1 {
        return MappedLocation::Row(0);
    }
    let row = (cursor as u128 * last as u128) / (source - 1) as u128;
    // The product is evaluated in u128; the quotient cannot exceed target - 1.
    MappedLocation::Row(usize::try_from(row).expect("a bounded proportional row fits usize"))
}
