use crate::commands::account::flag::{self, Action};
use crate::commands::global::NetworkArgs;
use crate::error::Error;

/// Sign an `AccountSet` that clears one flag.
#[derive(Debug, Clone, clap::Args)]
pub struct Cmd {
    /// The seed to use for signing
    #[arg(short, long)]
    pub seed: String,

    /// The flag to clear (e.g., asfRequireAuth, asfDisableMaster, etc.)
    #[arg(short, long)]
    pub flag: String,

    #[command(flatten)]
    pub network: NetworkArgs,
}

impl Cmd {
    pub fn run(&self) -> Result<(), Error> {
        flag::run(
            &self.seed,
            &self.flag,
            &self.network.url_or_mainnet(),
            Action::Clear,
        )
    }
}
