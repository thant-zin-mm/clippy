//! Silence Cutter backend library.
//!
//! The binary in `main.rs` is a thin wrapper; testable logic lives here so
//! integration tests can exercise the router directly (see `tests/`).

pub mod api;
pub mod audio;
pub mod config;
pub mod error;
pub mod jobs;
pub mod state;
