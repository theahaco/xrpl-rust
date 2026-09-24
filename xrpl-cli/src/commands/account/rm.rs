//! `xrpl account rm` — forget an account record.

use crate::error::Error;
use crate::output;
use crate::store::Store;

#[derive(Debug, Clone, clap::Args)]
pub struct Cmd {
    /// The account alias.
    pub alias: String,

    /// Also remove the key records it references.
    ///
    /// Off by default. A key can be referenced by more than one account, and
    /// forgetting one silently is not something to do on the way past.
    #[arg(long)]
    pub keys: bool,
}

impl Cmd {
    pub fn run(&self) -> Result<(), Error> {
        let store = Store::from_env()?;
        let record = store.account(&self.alias)?;

        if self.keys {
            for key in &record.keys {
                let shared: Vec<String> = store
                    .accounts_using_key(key)?
                    .into_iter()
                    .filter(|alias| alias != &self.alias)
                    .collect();

                if shared.is_empty() {
                    store.remove_key(key)?;
                    output::note(format!("forgot key record {key}"));
                } else {
                    output::warn(format!(
                        "keeping {key}: it is also referenced by {}",
                        shared.join(", ")
                    ));
                }
            }
        }

        store.remove_account(&self.alias)?;

        if store.default_account().as_deref() == Some(self.alias.as_str()) {
            store.set_default_account(None)?;
            output::note("it was the default account; there is now no default");
        }

        // What this did and did not do, said out loud. Nothing on the ledger
        // changed, and no secret was touched.
        output::note(format!(
            "forgot the record for {}. Nothing on the ledger changed{}.",
            self.alias,
            if self.keys {
                ""
            } else {
                ", and no key records were removed"
            }
        ));

        Ok(())
    }
}
