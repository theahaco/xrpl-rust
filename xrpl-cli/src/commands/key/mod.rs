//! `xrpl key` — the local key records.
//!
//! A key record is a public key, its curve, and a pointer to where the secret
//! lives. It never contains the secret itself.
//!
//! Only `watch-only` keys can be created here: a key that can be recognized but
//! not used. The encrypted-file and credential-store backends that hold real
//! secrets arrive next, and they slot in as another `source` rather than as a
//! change to any of these verbs.

pub mod add;
pub mod ls;
pub mod rm;
pub mod show;

use crate::error::Error;

#[derive(Debug, clap::Subcommand)]
pub enum Cmd {
    /// Record a key (local record, no network)
    Add(add::Cmd),
    /// List key records (local records, no network)
    Ls(ls::Cmd),
    /// Show one key record (local record, no network)
    Show(show::Cmd),
    /// Remove a key record (local record, no network)
    Rm(rm::Cmd),
}

impl Cmd {
    pub fn run(&self) -> Result<(), Error> {
        match self {
            Cmd::Add(cmd) => cmd.run(),
            Cmd::Ls(cmd) => cmd.run(),
            Cmd::Show(cmd) => cmd.run(),
            Cmd::Rm(cmd) => cmd.run(),
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
