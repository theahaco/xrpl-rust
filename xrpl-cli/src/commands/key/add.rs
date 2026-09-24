//! `xrpl key add` — record a public key.

use xrpl::core::keypairs::derive_classic_address;

use crate::error::Error;
use crate::output;
use crate::store::{KeyRecord, KeySource, Store};

#[derive(Debug, Clone, clap::Args)]
pub struct Cmd {
    /// What to call this key. You choose the name; `--sign-with` uses it.
    pub id: String,

    /// The public key, as uppercase hex.
    #[arg(long, value_name = "HEX")]
    pub public_key: String,

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

        let public_key = self.public_key.to_uppercase();

        // Derived rather than asked for, and cached, so listing records never
        // has to compute anything.
        let classic_address = derive_classic_address(&public_key).map_err(|error| {
            Error::other(format!(
                "{:?} is not a public key: {error}",
                self.public_key
            ))
        })?;

        // The curve is read from the key's own prefix here because the key is
        // in hand. A backend that never exposes one carries it explicitly,
        // which is why the record stores it rather than re-deriving it later.
        let algorithm = if public_key.starts_with("ED") {
            "ed25519"
        } else {
            "secp256k1"
        };

        let record = KeyRecord::new(
            public_key,
            algorithm.into(),
            classic_address,
            // The only source that can be created without a secret backend.
            KeySource::WatchOnly,
        );

        store.write_key(&self.id, &record)?;
        output::note(format!(
            "recorded {} as watch-only: it can be recognized, not used",
            self.id
        ));

        output::artifact(&super::describe(&self.id, &record))
    }
}
