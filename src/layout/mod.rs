//! Pure layout primitives. Hosts provide pane identities and draw the result.
//!
//! Geometry describes rectangles and dividers; it owns neither pane content
//! nor a terminal, data source, focus model, or resize timer. [`bsp`] splits
//! an area into panes; [`modal`] decides how tall a modal viewport asks to be.
//!
//! ```
//! use newtui::layout::{changed_panes, Direction, LayoutTree, Rect};
//!
//! let mut layout = LayoutTree::single(7);
//! assert!(layout.split(7, 42, Direction::Horizontal, 0.8));
//! let area = Rect { x: 0, y: 0, width: 200, height: 24 };
//! let before = layout.rects(area);
//! assert_eq!((before[0].1.width, before[1].1.width), (159, 40));
//! let smaller = layout.rects(Rect { width: 100, ..area });
//! assert_eq!((smaller[0].1.width, smaller[1].1.width), (79, 20));
//! assert_eq!(changed_panes(&before, &smaller), vec![7, 42]);
//! assert_eq!(layout.rects(area), before);
//! ```

pub mod bsp;
pub mod modal;

pub use bsp::{
    changed_panes, Direction, LayoutTree, PaneId, Rect, SplitBorder, MAX_RATIO, MIN_RATIO,
};
pub use modal::{ModalSize, SizeKey, FILL, MIN_ROWS};
