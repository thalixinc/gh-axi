//! gh-axi — AXI-compliant wrapper around the GitHub CLI.
//!
//! Library root. The binary is in `src/bin/gh-axi.rs`; modules are public so
//! integration tests exercise the same code paths as the shipped binary.

pub mod cli;
pub mod commands;
pub mod context;
pub mod error;
pub mod format;
pub mod gh;
pub mod host;
pub mod secret_value;
pub mod stdin;
pub mod suggestions;
pub mod toon;
pub mod version;
