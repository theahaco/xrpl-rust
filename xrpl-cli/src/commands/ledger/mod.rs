pub mod data;

use crate::error::Error;

#[derive(Debug, clap::Subcommand)]
pub enum Cmd {
    /// Get ledger data
    Data(data::Cmd),
}

impl Cmd {
    pub fn run(&self) -> Result<(), Error> {
        match self {
            Cmd::Data(cmd) => cmd.run(),
        }
    }
}
