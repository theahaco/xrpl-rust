pub mod channels;
pub mod clear_flag;
pub mod currencies;
pub mod flag;
pub mod info;
pub mod lines;
pub mod nfts;
pub mod objects;
pub mod set_flag;
pub mod tx;

use crate::error::Error;

#[derive(Debug, clap::Subcommand)]
pub enum Cmd {
    /// Get account info
    Info(info::Cmd),

    /// Get account transactions
    Tx(tx::Cmd),

    /// Get account objects (trust lines, offers, etc.)
    Objects(objects::Cmd),

    /// Get account channels
    Channels(channels::Cmd),

    /// Get account currencies
    Currencies(currencies::Cmd),

    /// Get account trust lines
    Lines(lines::Cmd),

    /// Get account NFTs (XLS-20)
    Nfts(nfts::Cmd),

    /// Set an account flag
    SetFlag(set_flag::Cmd),

    /// Clear an account flag
    ClearFlag(clear_flag::Cmd),
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
            Cmd::SetFlag(cmd) => cmd.run(),
            Cmd::ClearFlag(cmd) => cmd.run(),
        }
    }
}
