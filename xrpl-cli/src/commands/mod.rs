//! One module per command group; one file per command inside it.
//!
//! Every leaf command is a `Cmd` struct that owns its `clap` arguments and a
//! `run` method, so the flag definitions and the code reading them sit next to
//! each other. Group modules only route.

pub mod account;
pub mod global;
pub mod key;
pub mod ledger;
pub mod rpc;
pub mod server;
pub mod tx;
pub mod wallet;

use crate::error::Error;

/// Every `XRPL_*` variable, listed where `--help` will show it.
///
/// Most of these cannot be flags. A passphrase in `argv` is visible to `ps` and
/// the shell history, which is the whole thing this CLI avoids; the store
/// directories are resolved before any command parses. So they are documented
/// here rather than declared, and the ones that *can* be flags carry
/// `env = ` so they lose to an explicit flag and show up in its help.
const ENVIRONMENT: &str = "\
Environment:
  XRPL_NETWORK      mainnet | testnet | devnet | local, for the query commands.
                    Loses to --network and --url. `account fund` and the `tx`
                    pipeline stages require an explicit --network or --url.
  XRPL_SEED         a seed for one invocation. Loses to --seed-file.
  XRPL_PASSPHRASE   unlocks an enrolled key without a prompt. The non-interactive
                    path, because stdin carries the transaction.
  XRPL_ACCOUNT      the account to act as, when --account is not given.
  XRPL_DATA_DIR     where account and key records live.
                    Default ~/.local/share/xrpl, %LOCALAPPDATA%\\xrpl on Windows.
  XRPL_CONFIG_DIR   where config.toml lives. Default ~/.config/xrpl.
  XRPL_EDITOR       the editor `tx edit` opens. Then EDITOR, VISUAL, vi.

None of these is ever written by this CLI, and none carries key material to
disk. Exit codes: 0 success, 1 usage, 2 network, 3 ledger failure,
4 configuration not found, 5 declined, 6 signer unavailable.";

#[derive(Debug, clap::Parser)]
#[command(
    name = "xrpl",
    about = "XRPL command line utility",
    version,
    after_long_help = ENVIRONMENT
)]
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
    /// Wallet operations (deprecated: use `xrpl key` and `xrpl account`)
    ///
    /// Hidden because the account and key records replace it. `wallet generate`
    /// becomes `key generate`, and `wallet from-seed` becomes `key add`. It
    /// still works, and will be removed.
    #[command(subcommand, hide = true)]
    Wallet(wallet::Cmd),

    /// Account operations
    #[command(subcommand)]
    Account(account::Cmd),

    /// Local key records
    #[command(subcommand)]
    Key(key::Cmd),

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
            Commands::Key(cmd) => cmd.run(),
            Commands::Server(cmd) => cmd.run(),
            Commands::Ledger(cmd) => cmd.run(),
            Commands::Rpc(cmd) => cmd.run(),
            Commands::Tx(cmd) => cmd.run(),
        }
    }
}
