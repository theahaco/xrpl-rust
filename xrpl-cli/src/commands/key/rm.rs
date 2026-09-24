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

        store.remove_key(&self.id)?;

        // Forgetting the record is all this does. Whatever the record pointed
        // at is untouched — deleting a secret is a separate, irreversible act.
        output::note(format!(
            "forgot the record for {}. Anything it pointed at is untouched.",
            self.id
        ));

        Ok(())
    }
}
