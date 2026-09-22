pub mod faucet;
pub mod from_seed;
pub mod generate;
pub mod validate;

use crate::error::Error;

#[derive(Debug, clap::Subcommand)]
pub enum Cmd {
    /// Generate a new wallet
    Generate(generate::Cmd),

    /// Get wallet info from seed
    FromSeed(from_seed::Cmd),

    /// Generate a wallet funded by a faucet
    Faucet(faucet::Cmd),

    /// Validate an address
    Validate(validate::Cmd),
}

impl Cmd {
    pub fn run(&self) -> Result<(), Error> {
        match self {
            Cmd::Generate(cmd) => cmd.run(),
            Cmd::FromSeed(cmd) => cmd.run(),
            Cmd::Faucet(cmd) => cmd.run(),
            Cmd::Validate(cmd) => cmd.run(),
        }
    }
}
