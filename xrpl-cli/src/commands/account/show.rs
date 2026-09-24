//! `xrpl account show` — one account record.
//!
//! A **local** record, read from disk. `xrpl account info` is the one that asks
//! the ledger. The two sitting next to each other is the sharpest trap in this
//! command group, which is why both say which side they are on.

use crate::error::Error;
use crate::store::Store;

#[derive(Debug, Clone, clap::Args)]
pub struct Cmd {
    /// The account alias.
    pub alias: String,

    /// Emit JSON.
    #[arg(long)]
    pub json: bool,
}

impl Cmd {
    pub fn run(&self) -> Result<(), Error> {
        let store = Store::from_env()?;
        let record = store.account(&self.alias)?;

        if self.json {
            return crate::output::artifact(&super::describe(&self.alias, &record));
        }

        println!("alias           {}", self.alias);
        println!("address         {}", record.address);
        println!(
            "network         {}",
            record
                .network_id
                .map(|id| id.to_string())
                .unwrap_or_else(|| "unset (treated as mainnet)".into())
        );
        if let Some(tag) = record.tag {
            println!("tag             {tag}");
        }
        println!(
            "keys            {}",
            if record.keys.is_empty() {
                "none (watch-only)".to_string()
            } else {
                record.keys.join(", ")
            }
        );
        if let Some(default) = &record.default_signer {
            println!("default signer  {default}");
        }
        println!("\nThis is the local record. `xrpl account info` asks the ledger.");

        Ok(())
    }
}
