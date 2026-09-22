//! Command line interface for the XRP Ledger.
//!
//! The binary lives in `main.rs`; everything it does is reachable from here so
//! individual commands can be exercised from tests without spawning a process.
//!
//! Layout:
//!
//! - [`commands`] — one module per command group, one file per command. Each
//!   command owns its `clap` arguments and its `run` method.
//! - [`client`] — URL parsing, client construction, Tokio runtime.
//! - [`output`] — printing responses and signed transaction blobs.
//! - [`error`] — the single error type every command returns.

pub mod client;
pub mod commands;
pub mod error;
pub mod output;

pub use error::Error;
