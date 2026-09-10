//! `PyO3` face for `newtui`, kept outside the leaf core crate.

#![forbid(unsafe_code)]
#![warn(missing_docs, clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]

mod explore;
mod module;
mod settings;
mod types;

pub use module::add_to_module;
