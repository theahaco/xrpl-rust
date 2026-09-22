use xrpl::wallet::Wallet;

use crate::error::Error;

/// Derive a wallet from a seed.
#[derive(Debug, Clone, clap::Args)]
pub struct Cmd {
    /// The seed to use
    #[arg(long)]
    pub seed: String,

    /// The sequence number
    #[arg(long, default_value_t = 0)]
    pub sequence: u64,
}

impl Cmd {
    pub fn run(&self) -> Result<(), Error> {
        let wallet = Wallet::new(&self.seed, self.sequence)?;
        println!("Wallet from seed: {wallet:#?}");
        Ok(())
    }
}
