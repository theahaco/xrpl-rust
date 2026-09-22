use std::borrow::Cow;

use xrpl::asynch::transaction::sign;
use xrpl::models::transactions::nftoken_burn::NFTokenBurn;
use xrpl::wallet::Wallet;

use crate::commands::global::NetworkArgs;
use crate::error::Error;
use crate::output;

/// Sign an `NFTokenBurn`.
#[derive(Debug, Clone, clap::Args)]
pub struct Cmd {
    /// The seed to use for signing
    #[arg(short, long)]
    pub seed: String,

    /// NFT Token ID to burn
    #[arg(short, long)]
    pub nftoken_id: String,

    #[command(flatten)]
    pub network: NetworkArgs,
}

impl Cmd {
    pub fn run(&self) -> Result<(), Error> {
        let wallet = Wallet::new(&self.seed, 0)?;

        let mut tx = NFTokenBurn::builder(Cow::Owned(wallet.classic_address.clone()))
            .nftoken_id(self.nftoken_id.clone())
            .build();

        sign(&mut tx, &wallet, false)?;
        let tx_blob = output::signed_tx_blob(&tx)?;
        output::submit_hint(&tx_blob, &self.network.url_or_mainnet());

        Ok(())
    }
}
