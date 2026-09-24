//! `xrpl account add` — record an account.

use crate::error::Error;
use crate::output;
use crate::store::{AccountRecord, Store};

#[derive(Debug, Clone, clap::Args)]
pub struct Cmd {
    /// What to call this account.
    pub alias: String,

    /// The classic r-address.
    #[arg(long, value_name = "R_ADDRESS")]
    pub address: String,

    /// Which network it is on. Mainnet is 0.
    ///
    /// Replay protection rather than bookkeeping: `NetworkID` is omitted for a
    /// network at or below 1024 and required above it, and an implicit default
    /// account is refused on mainnet — both decided from this.
    #[arg(long)]
    pub network_id: Option<u32>,

    /// A destination tag carried with the address.
    #[arg(long)]
    pub tag: Option<u32>,

    /// A key record that can sign for it. Repeatable.
    #[arg(long = "key", value_name = "KEY_ID")]
    pub keys: Vec<String>,

    /// Which key to use when none is named.
    #[arg(long, value_name = "KEY_ID")]
    pub default_signer: Option<String>,

    /// Replace an existing record of the same name.
    #[arg(long)]
    pub force: bool,
}

impl Cmd {
    pub fn run(&self) -> Result<(), Error> {
        let store = Store::from_env()?;
        store.locator().warn_if_versioned();

        if !self.force && store.account(&self.alias).is_ok() {
            return Err(Error::other(format!(
                "an account named {:?} already exists; pass --force to replace it",
                self.alias
            )));
        }

        for key in &self.keys {
            // Referring to a key that is not there would produce a record that
            // only fails later, at signing time.
            store.key(key)?;
        }

        if let Some(default) = &self.default_signer {
            if !self.keys.iter().any(|key| key == default) {
                return Err(Error::other(format!(
                    "--default-signer {default} is not one of this account's keys"
                )));
            }
        }

        let mut record = AccountRecord::new(self.address.clone());
        record.network_id = self.network_id;
        record.tag = self.tag;
        record.keys = self.keys.clone();
        record.default_signer = self.default_signer.clone();

        store.write_account(&self.alias, &record)?;

        if record.is_watch_only() {
            output::note(format!(
                "{} is watch-only: it can be named and queried, not signed for",
                self.alias
            ));
        }

        output::artifact(&super::describe(&self.alias, &record))
    }
}
