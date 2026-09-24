//! `xrpl key show` — one key record.
//!
//! # Three shapes, one record
//!
//! The default is the human block. `--json` is the whole record for a script
//! that wants all of it. A part flag — `--address`, `--public-key` — is the
//! one field, bare on stdout and nothing else, for the far commoner case where
//! a script wants a single value and would otherwise reach for `jq`.
//!
//! That last shape is the other half of `account add --key`: the address was
//! always in the key record, and every script that wanted it had to run a JSON
//! parser to get it out — the ceremony in `demo/` did exactly that.

use crate::error::Error;
use crate::output;
use crate::store::{KeyRecord, Store};

#[derive(Debug, Clone, clap::Args)]
pub struct Cmd {
    /// The key id.
    pub id: String,

    /// Emit JSON.
    #[arg(long, conflicts_with = "part")]
    pub json: bool,

    /// Print only the address this key derives to.
    #[arg(long, group = "part")]
    pub address: bool,

    /// Print only the public key.
    #[arg(long, group = "part")]
    pub public_key: bool,

    /// Print only the curve: secp256k1 or ed25519.
    #[arg(long, group = "part")]
    pub algorithm: bool,

    /// Print only where the secret lives.
    #[arg(long, group = "part")]
    pub source: bool,
}

impl Cmd {
    pub fn run(&self) -> Result<(), Error> {
        let store = Store::from_env()?;
        let record = store.key(&self.id)?;

        if let Some(part) = self.part(&record) {
            return output::raw_line(&part);
        }

        if self.json {
            return output::artifact(&super::describe(&self.id, &record));
        }

        output::raw_line(&format!("id              {}", self.id))?;
        output::raw_line(&format!("public key      {}", record.public_key))?;
        output::raw_line(&format!("algorithm       {}", record.algorithm))?;
        output::raw_line(&format!("address         {}", record.classic_address))?;
        output::raw_line(&format!("secret          {}", record.source.label()))?;

        Ok(())
    }

    /// The single field asked for, if one was.
    ///
    /// Every one of these is on the record already, so this reaches no
    /// backend, no credential store and no network — the same guarantee that
    /// keeps `key ls` from prompting once per row.
    fn part(&self, record: &KeyRecord) -> Option<String> {
        if self.address {
            return Some(record.classic_address.clone());
        }
        if self.public_key {
            return Some(record.public_key.clone());
        }
        if self.algorithm {
            return Some(record.algorithm.clone());
        }
        if self.source {
            return Some(record.source.label().to_string());
        }

        None
    }
}
