//! `xrpl account` — everything addressed by an account.
//!
//! Every verb here is a **ledger query**: it reaches a node and reports what the
//! ledger says. The account and key records that arrive with the local store are
//! the other half of this group, and the distinction matters enough that each
//! verb's help says which side it is on — `account show` reading a local record
//! beside `account info` reading the ledger is a sharper trap than `xrpl tx`
//! beside `xrpl account tx`.
//!
//! Setting an account flag is `xrpl tx new AccountSet --set-flag <name>`, which
//! goes through the same autofill, signing and submission as every other
//! transaction rather than through a bespoke one-shot command.

pub mod channels;
pub mod currencies;
pub mod info;
pub mod lines;
pub mod nfts;
pub mod objects;
pub mod tx;

use crate::error::Error;

#[derive(Debug, clap::Subcommand)]
pub enum Cmd {
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
