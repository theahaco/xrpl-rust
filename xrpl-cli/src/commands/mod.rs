//! One module per command group; one file per command inside it.
//!
//! Every leaf command is a `Cmd` struct that owns its `clap` arguments and a
//! `run` method, so the flag definitions and the code reading them sit next to
//! each other. Group modules only route.

pub mod account;
pub mod global;
pub mod ledger;
pub mod rpc;
pub mod server;
pub mod tx;
pub mod wallet;

use crate::error::Error;

#[derive(Debug, clap::Parser)]
#[command(name = "xrpl", about = "XRPL command line utility", version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    /// Silence human-facing output on stderr. The machine artifact on stdout is
    /// never silenced — it is the command's output, not its commentary.
    #[arg(short = 'q', long, global = true)]
    pub quiet: bool,
}

impl Cli {
    pub fn run(&self) -> Result<(), Error> {
        crate::output::set_quiet(self.quiet);
        self.command.run()
    }
}

#[derive(Debug, clap::Subcommand)]
pub enum Commands {
    /// Wallet operations
    #[command(subcommand)]
    Wallet(wallet::Cmd),

    /// Account operations
    #[command(subcommand)]
    Account(account::Cmd),

    /// Server operations
    #[command(subcommand)]
    Server(server::Cmd),

    /// Ledger operations
    #[command(subcommand)]
    Ledger(ledger::Cmd),

    /// Call any rippled RPC directly
    Rpc(rpc::Cmd),

    /// Build, sign and submit transactions through a pipeline
    #[command(subcommand)]
    Tx(tx::Cmd),
}

impl Commands {
    pub fn run(&self) -> Result<(), Error> {
        match self {
            Commands::Wallet(cmd) => cmd.run(),
            Commands::Account(cmd) => cmd.run(),
            Commands::Server(cmd) => cmd.run(),
            Commands::Ledger(cmd) => cmd.run(),
            Commands::Rpc(cmd) => cmd.run(),
            Commands::Tx(cmd) => cmd.run(),
        }
    }
}
