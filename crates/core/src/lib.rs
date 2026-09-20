//! Engine-agnostic core logic and data structures for Evergreen Browser.
//!
//! This crate contains settings management, tab state modeling, typed IPC protocol
//! definitions, environment detection routines, and the [`EngineHost`] trait that decouples
//! the browser shell from any specific rendering engine implementation.

pub mod engine;
pub mod env;
pub mod ipc;
pub mod settings;
pub mod tabs;
pub mod updater;
