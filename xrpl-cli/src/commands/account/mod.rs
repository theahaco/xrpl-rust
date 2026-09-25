//! `xrpl account` — everything addressed by an account.
//!
//! Local records sit beside ledger queries; each verb's help identifies which
//! it operates on. `fund` requests test XRP for an existing public address and
//! confirms it on the ledger, without accessing any signing key.
//!
//! Setting an account flag is `xrpl tx new AccountSet --set-flag <name>`, which
//! goes through the same autofill, signing and submission as every other
//! transaction rather than through a bespoke one-shot command.

pub mod add;
pub mod channels;
pub mod currencies;
pub mod doctor;
pub mod fund;
pub mod info;
pub mod lines;
pub mod ls;
pub mod nfts;
pub mod objects;
pub mod rm;
pub mod show;
pub mod subject;
pub mod tx;
pub mod r#use;

use crate::error::Error;

#[derive(Debug, clap::Subcommand)]
pub enum Cmd {
    // -- local records: no network, ever -------------------------------------
    /// Record an account (local record, no network)
    Add(add::Cmd),

    /// List account records (local records, no network)
    Ls(ls::Cmd),

    /// Show one account record (local record, no network)
    Show(show::Cmd),

    /// Forget an account record (local record, no network)
    Rm(rm::Cmd),

    /// Choose the default account (local record, no network)
    Use(r#use::Cmd),

    /// Report what is wrong with a record (local; --ledger adds a node check)
    Doctor(doctor::Cmd),

    /// Fund an existing account from a test-network faucet (no keys required)
    Fund(fund::Cmd),

    // -- ledger queries ------------------------------------------------------
    /// Get account info from the ledger (ledger query)
    Info(info::Cmd),

    /// Get account transactions (ledger query)
    ///
    /// Aliased as `history`: `xrpl account tx` sits next to `xrpl tx`, and
    /// while clap never has to disambiguate them, a reader does.
    #[command(alias = "history")]
    Tx(tx::Cmd),

    /// Get account objects: trust lines, offers, signer lists (ledger query)
    Objects(objects::Cmd),

    /// Get account payment channels (ledger query)
    Channels(channels::Cmd),

    /// Get the currencies an account can send or receive (ledger query)
    Currencies(currencies::Cmd),

    /// Get account trust lines (ledger query)
    Lines(lines::Cmd),

    /// Get account NFTs, XLS-20 (ledger query)
    Nfts(nfts::Cmd),
}

impl Cmd {
    pub fn run(&self) -> Result<(), Error> {
        match self {
            Cmd::Add(cmd) => cmd.run(),
            Cmd::Ls(cmd) => cmd.run(),
            Cmd::Show(cmd) => cmd.run(),
            Cmd::Rm(cmd) => cmd.run(),
            Cmd::Use(cmd) => cmd.run(),
            Cmd::Doctor(cmd) => cmd.run(),
            Cmd::Fund(cmd) => cmd.run(),
            Cmd::Info(cmd) => cmd.run(),
            Cmd::Tx(cmd) => cmd.run(),
            Cmd::Objects(cmd) => cmd.run(),
            Cmd::Channels(cmd) => cmd.run(),
            Cmd::Currencies(cmd) => cmd.run(),
            Cmd::Lines(cmd) => cmd.run(),
            Cmd::Nfts(cmd) => cmd.run(),
        }
    }
}

/// Render an account record for a human or for a script.
pub fn describe(alias: &str, record: &crate::store::AccountRecord) -> serde_json::Value {
    serde_json::json!({
        "alias": alias,
        "address": record.address,
        "network_id": record.network_id,
        "tag": record.tag,
        "keys": record.keys,
        "default_signer": record.default_signer,
        "watch_only": record.is_watch_only(),
    })
}
