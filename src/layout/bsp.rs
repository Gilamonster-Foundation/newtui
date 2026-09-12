//! Binary panel geometry whose proportions survive terminal resizing.
//!
//! The one-cell divider contract comes from Gilamonster Foundation's
//! `gilamonster-agent/src/layout.rs` (Apache-2.0, Copyright 2026 Shawn Hartsock /
//! Gilamonster Foundation). Binary traversal and boolean split paths are
//! adapted from `herdrdev/herdr/src/layout.rs` at
//! `09cdd88d0aca35617eb05468c2421b0467656e4f`, licensed under Apache-2.0.
//! Unlike that donor's geometry, this module reserves a divider cell.
//!
//! Only topology, host-supplied IDs, and ratios are stored. Rectangles are
//! derived afresh, so shrinking a terminal cannot overwrite its proportions.

use std::collections::{BTreeMap, BTreeSet};

/// A host-owned locator. The library does not allocate, hash, or mint IDs.
pub type PaneId = usize;

/// The smallest accepted first-child proportion.
pub const MIN_RATIO: f32 = 0.1;
/// The largest accepted first-child proportion.
pub const MAX_RATIO: f32 = 0.9;

/// An area with half-open coordinate edges bounded by [`u16::MAX`].
///
/// Projection clips width to `u16::MAX - x` and height to `u16::MAX - y`.
/// Thus `x == u16::MAX` has zero usable width, and coordinates never wrap.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Rect {
    /// Left edge.
    pub x: u16,
    /// Top edge.
    pub y: u16,
    /// Requested number of columns.
    pub width: u16,
    /// Requested number of rows.
    pub height: u16,
}

impl Rect {
    fn normalized(self) -> Self {
        Self {
            width: self.width.min(u16::MAX - self.x),
            height: self.height.min(u16::MAX - self.y),
            ..self
        }
    }
}

/// The axis along which a split divides its children.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Direction {
    /// First child on the left, second on the right; the divider is vertical.
    Horizontal,
    /// First child above, second below; the divider is horizontal.
    Vertical,
}

/// One structural split, projected for host drawing and pointer hit testing.
///
/// An empty `area` has no hittable divider, but retains its path. Paths locate
/// nodes in the current topology; hosts should refresh them after a split.
#[derive(Debug, Clone, PartialEq)]
pub struct SplitBorder {
    /// Divider x coordinate for a horizontal split, y for a vertical split.
    pub pos: u16,
    /// The split axis, not the orientation of the drawn divider line.
    pub direction: Direction,
    /// Current first-child proportion, always finite and in the accepted band.
    pub ratio: f32,
    /// Normalized area belonging to this split node.
    pub area: Rect,
    /// Root-to-node path: false chooses the first child, true the second.
    pub path: Vec<bool>,
}

#[derive(Debug, Clone, PartialEq)]
enum Node {
    Pane(PaneId),
    Split {
        direction: Direction,
        ratio: f32,
        first: Box<Node>,
        second: Box<Node>,
    },
}

/// Binary space partition of panes, independent of their contents or renderer.
#[derive(Debug, Clone, PartialEq)]
pub struct LayoutTree {
    root: Node,
}

impl LayoutTree {
    /// Begin with one pane, using the host's existing locator.
    #[must_use]
    pub fn single(id: PaneId) -> Self {
        Self {
            root: Node::Pane(id),
        }
    }

    /// Replace a leaf with a split: the existing pane first, `new_id` second.
    ///
    /// Finite ratios clamp to [`MIN_RATIO`]..=[`MAX_RATIO`]. Nonfinite ratios,
    /// unknown targets, or an ID already in this tree return false and leave
    /// the entire tree unchanged. No minimum terminal size restricts topology.
    pub fn split(
        &mut self,
        target: PaneId,
        new_id: PaneId,
        direction: Direction,
        ratio: f32,
    ) -> bool {
        let Some(ratio) = bounded_ratio(ratio) else {
            return false;
        };
        if self.root.contains(new_id) {
            return false;
        }
        let Some(node) = self.root.pane_mut(target) else {
            return false;
        };
        *node = Node::Split {
            direction,
            ratio,
            first: Box::new(Node::Pane(target)),
            second: Box::new(Node::Pane(new_id)),
        };
        true
    }

    /// Project every leaf in first-child order, retaining zero-size panes.
    ///
    /// Each split reserves one cell along its axis when that extent is nonzero.
    /// Of the remaining cells, the first child receives `floor(cells * ratio)`
    /// and the second receives the remainder. Multiplication uses the stored
    /// `f32` proportion; ratios are never widened or rewritten.
    #[must_use]
    pub fn rects(&self, area: Rect) -> Vec<(PaneId, Rect)> {
        let mut panes = Vec::new();
        self.root.collect_panes(area.normalized(), &mut panes);
        panes
    }

    /// Project one entry per structural split, in preorder, even for empty areas.
    #[must_use]
    pub fn splits(&self, area: Rect) -> Vec<SplitBorder> {
        let mut borders = Vec::new();
        self.root
            .collect_splits(area.normalized(), &mut Vec::new(), &mut borders);
        borders
    }

    /// Update the split selected by a path returned from [`Self::splits`].
    ///
    /// An accepted path returns true, including a no-op or a ratio change that
    /// rounds to the same cells. Invalid paths and nonfinite ratios return
    /// false without modifying any node. Finite inputs clamp to the band.
    pub fn set_ratio_at(&mut self, path: &[bool], ratio: f32) -> bool {
        let Some(ratio) = bounded_ratio(ratio) else {
            return false;
        };
        let mut node = &mut self.root;
        for second_child in path {
            let Node::Split { first, second, .. } = node else {
                return false;
            };
            node = if *second_child { second } else { first };
        }
        let Node::Split {
            ratio: stored_ratio,
            ..
        } = node
        else {
            return false;
        };
        *stored_ratio = ratio;
        true
    }
}

/// Compare two [`LayoutTree::rects`] projections, returning sorted changed IDs.
///
/// A changed rectangle, newly present ID, or removed ID is reported once.
/// Reordering the same pairs or changing a ratio without moving a cell is not
/// a geometry change. Each input must contain at most one rectangle per ID,
/// as a tree's projection does. No content, cache, or timer is consulted.
#[must_use]
pub fn changed_panes(before: &[(PaneId, Rect)], after: &[(PaneId, Rect)]) -> Vec<PaneId> {
    let before: BTreeMap<_, _> = before.iter().copied().collect();
    let after: BTreeMap<_, _> = after.iter().copied().collect();
    let ids: BTreeSet<_> = before.keys().chain(after.keys()).copied().collect();
    ids.into_iter()
        .filter(|id| before.get(id) != after.get(id))
        .collect()
}

fn bounded_ratio(ratio: f32) -> Option<f32> {
    ratio.is_finite().then(|| ratio.clamp(MIN_RATIO, MAX_RATIO))
}

impl Node {
    fn contains(&self, id: PaneId) -> bool {
        match self {
            Self::Pane(existing) => *existing == id,
            Self::Split { first, second, .. } => first.contains(id) || second.contains(id),
        }
    }

    fn pane_mut(&mut self, id: PaneId) -> Option<&mut Self> {
        match self {
            Self::Pane(existing) if *existing == id => Some(self),
            Self::Pane(_) => None,
            Self::Split { first, second, .. } => first.pane_mut(id).or_else(|| second.pane_mut(id)),
        }
    }

    fn collect_panes(&self, area: Rect, panes: &mut Vec<(PaneId, Rect)>) {
        match self {
            Self::Pane(id) => panes.push((*id, area)),
            Self::Split {
                direction,
                ratio,
                first,
                second,
            } => {
                let (first_area, second_area, _) = split_rect(area, *direction, *ratio);
                first.collect_panes(first_area, panes);
                second.collect_panes(second_area, panes);
            }
        }
    }

    fn collect_splits(&self, area: Rect, path: &mut Vec<bool>, borders: &mut Vec<SplitBorder>) {
        if let Self::Split {
            direction,
            ratio,
            first,
            second,
        } = self
        {
            let (first_area, second_area, pos) = split_rect(area, *direction, *ratio);
            borders.push(SplitBorder {
                pos,
                direction: *direction,
                ratio: *ratio,
                area,
                path: path.clone(),
            });
            path.push(false);
            first.collect_splits(first_area, path, borders);
            path.pop();
            path.push(true);
            second.collect_splits(second_area, path, borders);
            path.pop();
        }
    }
}

// Inputs are normalized rectangles and finite ratios in 0.1..=0.9. The floor
// is nonnegative and at most u16::MAX. Every u16 extent is exactly representable
// in f32; multiplication uses the stored proportion before the final floor.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn split_rect(area: Rect, direction: Direction, ratio: f32) -> (Rect, Rect, u16) {
    let extent = match direction {
        Direction::Horizontal => area.width,
        Direction::Vertical => area.height,
    };
    let divider = extent.min(1);
    let available = extent - divider;
    let first_extent = (f32::from(available) * ratio).floor() as u16;
    let second_extent = available - first_extent;
    match direction {
        Direction::Horizontal => (
            Rect {
                width: first_extent,
                ..area
            },
            Rect {
                x: area.x + first_extent + divider,
                width: second_extent,
                ..area
            },
            area.x + first_extent,
        ),
        Direction::Vertical => (
            Rect {
                height: first_extent,
                ..area
            },
            Rect {
                y: area.y + first_extent + divider,
                height: second_extent,
                ..area
            },
            area.y + first_extent,
        ),
    }
}
