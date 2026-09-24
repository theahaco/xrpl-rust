//! `xrpl key ls` — list key records.

use crate::error::Error;
use crate::store::Store;

#[derive(Debug, Clone, clap::Args)]
pub struct Cmd {
    /// Emit JSON.
    #[arg(long)]
    pub json: bool,
}

impl Cmd {
    pub fn run(&self) -> Result<(), Error> {
        let store = Store::from_env()?;
        let ids = store.key_ids()?;

        let mut rows = Vec::with_capacity(ids.len());
        for id in &ids {
            // Reading a record touches one file and nothing else: no credential
            // store, no device, no network. Listing must never prompt.
            rows.push(super::describe(id, &store.key(id)?));
        }

        if self.json {
            return crate::output::artifact(&rows);
        }

        if rows.is_empty() {
            crate::output::note("no keys recorded");
            return Ok(());
        }

        for row in &rows {
            // The public key is shown alongside the id because two people can
            // both name a key `issuer` and mean different keys.
            println!(
                "{:<24} {:<12} {:<36} {}",
                row["id"].as_str().unwrap_or_default(),
                row["algorithm"].as_str().unwrap_or_default(),
                row["classic_address"].as_str().unwrap_or_default(),
                row["source"].as_str().unwrap_or_default(),
            );
        }

        Ok(())
    }
}
