//! `xrpl key` — the local key records.
//!
//! A key record is a public key, its curve, and a pointer to where the secret
//! lives. It never contains the secret itself.
//!
//! Where a secret lives is a `source` on the record — `encrypted-file`,
//! `secure-store`, or `watch-only` for a key that can be recognized but never
//! used. Adding a backend is adding a `source`, not changing any of these
//! verbs; `xrpl key backends` lists the ones this binary was built with.

pub mod add;
pub mod backends;
pub mod enrol;
pub mod export;
pub mod generate;
pub mod ls;
pub mod rm;
pub mod show;

use crate::error::Error;

#[derive(Debug, clap::Subcommand)]
pub enum Cmd {
    /// Record a key (local record, no network)
    Add(add::Cmd),
    /// Generate a key and enrol it (local record, no network)
    Generate(generate::Cmd),
    /// Print a key's seed (local record, no network)
    Export(export::Cmd),
    /// List key records (local records, no network)
    Ls(ls::Cmd),
    /// Show one key record (local record, no network)
    Show(show::Cmd),
    /// Remove a key record (local record, no network)
    Rm(rm::Cmd),
    /// List the signing backends this binary was built with (no network)
    Backends(backends::Cmd),
}

impl Cmd {
    pub fn run(&self) -> Result<(), Error> {
        match self {
            Cmd::Add(cmd) => cmd.run(),
            Cmd::Generate(cmd) => cmd.run(),
            Cmd::Export(cmd) => cmd.run(),
            Cmd::Ls(cmd) => cmd.run(),
            Cmd::Show(cmd) => cmd.run(),
            Cmd::Rm(cmd) => cmd.run(),
            Cmd::Backends(cmd) => cmd.run(),
        }
    }
}

/// Render a key record for a human or for a script.
pub fn describe(id: &str, record: &crate::store::KeyRecord) -> serde_json::Value {
    serde_json::json!({
        "id": id,
        "public_key": record.public_key,
        "algorithm": record.algorithm,
        "classic_address": record.classic_address,
        "source": record.source.label(),
    })
}
