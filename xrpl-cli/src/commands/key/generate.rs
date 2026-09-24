//! `xrpl key generate` — make a key and enrol it.

use serde_json::json;
use zeroize::Zeroize;

use crate::error::Error;
use crate::output;
use crate::store::Store;

#[derive(Debug, Clone, clap::Args)]
pub struct Cmd {
    /// What to call the key.
    pub id: String,

    /// Print the seed on stdout.
    ///
    /// The documented non-interactive path, and it is never removed: a
    /// confirmation prompt is unusable from a script, and a script that cannot
    /// record its own key is a script that loses it.
    #[arg(long)]
    pub show_secret: bool,

    /// Which curve to use.
    #[arg(long, value_parser = ["secp256k1", "ed25519"])]
    pub algorithm: Option<String>,

    /// Replace an existing record of the same name.
    #[arg(long)]
    pub force: bool,
}

impl Cmd {
    pub fn run(&self) -> Result<(), Error> {
        let store = Store::from_env()?;
        store.locator().warn_if_versioned();

        if !self.force && store.key(&self.id).is_ok() {
            return Err(Error::other(format!(
                "a key named {:?} already exists; pass --force to replace it",
                self.id
            )));
        }

        let algorithm = match self.algorithm.as_deref() {
            Some("ed25519") => Some(xrpl::constants::CryptoAlgorithm::ED25519),
            Some("secp256k1") => Some(xrpl::constants::CryptoAlgorithm::SECP256K1),
            _ => None,
        };

        let mut seed = xrpl::core::keypairs::generate_seed(None, algorithm)?;
        let record = super::enrol::enrol(&store, &self.id, &seed)?;

        // The 16-byte family seed *is* the backup: this crate has no mnemonic
        // convention, so there is nothing else human-transcribable. Show it
        // once, or say plainly that it is gone.
        if self.show_secret {
            let mut value = json!({
                "id": self.id,
                "classic_address": record.classic_address,
                "public_key": record.public_key,
                "algorithm": record.algorithm,
                "seed": seed,
            });
            seed.zeroize();

            let printed = output::artifact(&value);
            value["seed"] = json!(null);

            return printed;
        }

        seed.zeroize();
        output::warn(format!(
            "the seed was not printed. It is encrypted in the store and nowhere else: \
             if you lose the passphrase, {} is gone. Re-run with --show-secret to \
             record it, or use `xrpl key export`.",
            self.id
        ));

        output::artifact(&super::describe(&self.id, &record))
    }
}
