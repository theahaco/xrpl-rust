//! `xrpl key rm` — forget a key record.

use crate::error::Error;
use crate::output;
use crate::store::Store;

#[derive(Debug, Clone, clap::Args)]
pub struct Cmd {
    /// The key id.
    pub id: String,

    /// Remove it even though accounts still reference it.
    #[arg(long)]
    pub force: bool,

    /// Also destroy the encrypted secret this record points at.
    ///
    /// Irreversible. The 16-byte family seed is the only backup there is —
    /// this crate has no mnemonic convention — so unless it was written down,
    /// the key is gone.
    #[arg(long)]
    pub delete_secret: bool,
}

impl Cmd {
    pub fn run(&self) -> Result<(), Error> {
        let store = Store::from_env()?;

        // A dangling reference is a state `account doctor` reports, but leaving
        // one behind silently is not the same as being told about it.
        let using = store.accounts_using_key(&self.id)?;
        if !using.is_empty() && !self.force {
            return Err(Error::other(format!(
                "{} is still referenced by {}; pass --force to remove it anyway \
                 (those accounts will then have a dangling key reference, which \
                 `xrpl account doctor` reports)",
                self.id,
                using.join(", ")
            )));
        }

        // Read before removing: once the record is gone there is nothing left
        // saying where its secret lives, and a secure-store entry nobody can
        // name is invisible rather than merely orphaned.
        let record = store.key(&self.id).ok();

        if self.delete_secret {
            match &record {
                Some(record) => {
                    store.remove_key_secret(record)?;
                    output::warn(format!(
                        "destroyed the secret for {}. If the seed was not written down, \
                         that key is gone.",
                        self.id
                    ));
                }
                None => {
                    return Err(Error::other(format!(
                        "cannot destroy the secret for {}: its record cannot be read, \
                         so there is nothing saying where the secret is",
                        self.id
                    )))
                }
            }
        }

        store.remove_key(&self.id)?;

        if !self.delete_secret {
            // Forgetting the record is all this does by default. Whatever it
            // pointed at is untouched — destroying a secret is a separate,
            // irreversible act, and `--delete-secret` is how you ask for it.
            output::note(format!(
                "forgot the record for {}. Anything it pointed at is untouched; \
                 pass --delete-secret to destroy it too.",
                self.id
            ));
        }

        Ok(())
    }
}
