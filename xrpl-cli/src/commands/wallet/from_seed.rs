//! `xrpl wallet from-seed` — show the address a seed derives.

use serde_json::json;

use xrpl::signer::RawSigner;

use crate::error::Error;
use crate::output;
use crate::signer::SigningArgs;

/// Derive a wallet from a seed.
#[derive(Debug, Clone, clap::Args)]
pub struct Cmd {
    #[command(flatten)]
    pub signing: SigningArgs,
}

impl Cmd {
    pub fn run(&self) -> Result<(), Error> {
        // Through `SigningArgs`, so the address can be checked without the seed
        // ever reaching `argv`.
        let wallet = self.signing.resolve()?;

        output::artifact(&json!({
            "classic_address": wallet.classic_address,
            "public_key": wallet.public_key,
            "algorithm": format!("{:?}", wallet.algorithm()),
        }))
    }
}
