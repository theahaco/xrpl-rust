//! `xrpl key export` — get a seed back out.
//!
//! Gated behind an explicit acknowledgement, because it is the one command that
//! turns an encrypted record back into the thing the encryption was for.
//!
//! It exists because the alternative is worse. The store is the only copy: a
//! reset credential store, a wiped home directory, or a machine that dies takes
//! the account with it, and this crate has no mnemonic convention, so the
//! 16-byte family seed *is* the backup. A store with no way out strands anyone
//! migrating machines.

use serde_json::json;
use zeroize::Zeroize;

use crate::error::Error;
use crate::output;
use crate::signer::stored::PASSPHRASE_ENV;
use crate::store::{secret, Store};
use crate::tty;

#[derive(Debug, Clone, clap::Args)]
pub struct Cmd {
    /// The key id.
    pub id: String,

    /// Required. Says out loud what this does.
    #[arg(long = "i-understand-this-prints-a-secret")]
    pub acknowledged: bool,
}

impl Cmd {
    pub fn run(&self) -> Result<(), Error> {
        if !self.acknowledged {
            return Err(Error::other(
                "this prints key material on stdout. Pass \
                 --i-understand-this-prints-a-secret if that is what you want.",
            ));
        }

        let store = Store::from_env()?;
        let record = store.key(&self.id)?;

        let blob = store.read_key_secret(&self.id, &record)?;

        let passphrase = match std::env::var(PASSPHRASE_ENV) {
            Ok(value) if !value.is_empty() => value,
            _ => tty::prompt_secret(&format!("passphrase for {}", self.id))?,
        };

        let mut seed = secret::decrypt(&blob, &passphrase)?;
        let value = json!({
            "id": self.id,
            "classic_address": record.classic_address,
            "seed": seed,
        });
        seed.zeroize();

        output::warn("this output contains key material; it is not in your shell history, but it is on your screen");

        output::artifact(&value)
    }
}
