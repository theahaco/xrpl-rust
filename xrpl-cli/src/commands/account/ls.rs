//! `xrpl account ls` — list account records.

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
        let default = store.default_account();
        let aliases = store.account_aliases()?;

        let mut rows = Vec::with_capacity(aliases.len());
        for alias in &aliases {
            // One file read per record and nothing else. No credential store,
            // no device, no network: listing accounts must never prompt.
            rows.push(super::describe(alias, &store.account(alias)?));
        }

        if self.json {
            return crate::output::artifact(&rows);
        }

        if rows.is_empty() {
            crate::output::note("no accounts recorded");
            return Ok(());
        }

        for row in &rows {
            let alias = row["alias"].as_str().unwrap_or_default();
            let marker = if default.as_deref() == Some(alias) {
                "*"
            } else {
                " "
            };
            crate::output::raw_line(&format!(
                "{marker} {:<20} {:<36} {}",
                alias,
                row["address"].as_str().unwrap_or_default(),
                row["keys"].as_array().map(Vec::len).unwrap_or(0),
            ))?;
        }

        Ok(())
    }
}
