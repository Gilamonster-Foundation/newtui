#![doc = include_str!("../../docs/CATALOG.md")]

//! The interactive components shipped by this crate.
//!
//! [`settings_panel`] is the pilot extraction: a host supplies its vocabulary,
//! drives the keys, draws the resulting view, and carries out the returned
//! intent. The component owns none of those effects.

pub mod settings_panel;
