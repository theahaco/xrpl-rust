use std::borrow::Cow;

use xrpl::asynch::transaction::sign;
use xrpl::models::transactions::nftoken_mint::{NFTokenMint, NFTokenMintFlag};
use xrpl::wallet::Wallet;

use crate::commands::global::NetworkArgs;
use crate::error::Error;
use crate::output;

/// Sign an `NFTokenMint`.
#[derive(Debug, Clone, clap::Args)]
pub struct Cmd {
    /// The seed to use for signing
    #[arg(short, long)]
    pub seed: String,

    /// URI for the NFT (hex-encoded)
    #[arg(long)]
    pub uri: String,

    /// Flags bitmask (optional)
    #[arg(long)]
    pub flags: Option<u32>,

    /// Transfer fee (optional)
    #[arg(long)]
    pub transfer_fee: Option<u16>,

    #[command(flatten)]
    pub network: NetworkArgs,
}

impl Cmd {
    pub fn run(&self) -> Result<(), Error> {
        let wallet = Wallet::new(&self.seed, 0)?;
        // A bitmask expands into the individual flags the model carries.
        let flags = self.flags.map(NFTokenMintFlag::from_bits);

        let mut tx = NFTokenMint::builder(Cow::Owned(wallet.classic_address.clone()))
            .maybe_flags(flags)
            .nftoken_taxon(0u32)
            .maybe_transfer_fee(self.transfer_fee.map(u32::from))
            .uri(self.uri.clone())
            .build();

        sign(&mut tx, &wallet, false)?;
        let tx_blob = output::signed_tx_blob(&tx)?;
        output::submit_hint(&tx_blob, &self.network.url_or_mainnet());

        Ok(())
    }
}
