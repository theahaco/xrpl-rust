pub mod fee;
pub mod info;
pub mod subscribe;

use crate::error::Error;

#[derive(Debug, clap::Subcommand)]
pub enum Cmd {
    /// Get current network fee
    Fee(fee::Cmd),

    /// Get server info
    Info(info::Cmd),

    /// Subscribe to ledger events
    Subscribe(subscribe::Cmd),
}

impl Cmd {
    pub fn run(&self) -> Result<(), Error> {
        match self {
            Cmd::Fee(cmd) => cmd.run(),
            Cmd::Info(cmd) => cmd.run(),
            Cmd::Subscribe(cmd) => cmd.run(),
        }
    }
}
