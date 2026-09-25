//! `xrpl account show` — one account record.
//!
//! A **local** record, read from disk. `xrpl account info` is the one that asks
//! the ledger. The two sitting next to each other is the sharpest trap in this
//! command group, which is why both say which side they are on.
//!
//! The part flags — `--address`, `--keys` — are the same shape as `key show`'s:
//! one field, bare on stdout, for a script that wants a value rather than a
//! document. A field that is *set to nothing* prints nothing and says so on
//! stderr, so `$(xrpl account show alice --tag)` is empty rather than the word
//! "none". `--default-signer` is the exception and fails instead, because an
//! account with no key and an account with several and no default have no
//! answer to give rather than an empty one.
//!
//! `--address` here is a part flag and takes no value, while `--address
//! <R_ADDRESS>` on the ledger-query verbs is the hidden deprecated alias for
//! `--account` (see `subject.rs`). The two live in the same help output for as
//! long as that alias does, which is one minor.

use crate::error::Error;
use crate::output;
use crate::store::{AccountRecord, Store};

#[derive(Debug, Clone, clap::Args)]
pub struct Cmd {
    /// The account alias.
    pub alias: String,

    /// Emit JSON.
    #[arg(long, conflicts_with = "part")]
    pub json: bool,

    /// Print only the classic r-address.
    #[arg(long, group = "part")]
    pub address: bool,

    /// Print only the network id.
    #[arg(long, group = "part")]
    pub network_id: bool,

    /// Print only the destination tag.
    #[arg(long, group = "part")]
    pub tag: bool,

    /// Print only the key ids, one per line.
    #[arg(long, group = "part")]
    pub keys: bool,

    /// Print only the key this record names as its default.
    #[arg(long, group = "part")]
    pub default_signer: bool,
}

impl Cmd {
    pub fn run(&self) -> Result<(), Error> {
        let store = Store::from_env()?;
        let record = store.account(&self.alias)?;

        if let Some(lines) = self.part(&record)? {
            for line in lines {
                output::raw_line(&line)?;
            }
            return Ok(());
        }

        if self.json {
            return output::artifact(&super::describe(&self.alias, &record));
        }

        output::raw_line(&format!("alias           {}", self.alias))?;
        output::raw_line(&format!("address         {}", record.address))?;
        output::raw_line(&format!(
            "network         {}",
            record
                .network_id
                .map(|id| id.to_string())
                .unwrap_or_else(|| "unset (treated as mainnet)".into())
        ))?;
        if let Some(tag) = record.tag {
            output::raw_line(&format!("tag             {tag}"))?;
        }
        output::raw_line(&format!(
            "keys            {}",
            if record.keys.is_empty() {
                "none (watch-only)".to_string()
            } else {
                record.keys.join(", ")
            }
        ))?;

        // The key this record names, not only the one it records. An account
        // with a single key has no `default_signer` field and that key is
        // still the answer, which is the common case — saying nothing here
        // made the commonest record look like it named no key at all. The
        // ambiguous record is named too: it is the one a person most needs to
        // be told to choose for.
        match record.signer_key(None) {
            Ok(signer) if record.default_signer.is_some() => {
                output::raw_line(&format!("default signer  {signer}"))?;
            }
            Ok(signer) => {
                output::raw_line(&format!("default signer  {signer} (the only key)"))?;
            }
            // Watch-only. The keys line above already said so.
            Err(_) if record.keys.is_empty() => {}
            Err(_) => {
                output::raw_line("default signer  unset, and this account has several keys")?;
            }
        }

        // A cross-reference, not a field of the record, so it belongs on the
        // channel a human reads and `-q` silences.
        output::note("\nThis is the local record. `xrpl account info` asks the ledger.");

        Ok(())
    }

    /// The single field asked for, as the lines to print, if one was asked for.
    ///
    /// `Some(vec![])` is a field that is set to nothing — a real answer, and a
    /// different thing from `None`, which is "no part was asked for".
    fn part(&self, record: &AccountRecord) -> Result<Option<Vec<String>>, Error> {
        if self.address {
            return Ok(Some(vec![record.address.clone()]));
        }

        if self.network_id {
            return Ok(Some(match record.network_id {
                Some(id) => vec![id.to_string()],
                None => {
                    output::note("network is unset, and an unset network is treated as mainnet");
                    Vec::new()
                }
            }));
        }

        if self.tag {
            return Ok(Some(match record.tag {
                Some(tag) => vec![tag.to_string()],
                None => {
                    output::note("no destination tag is recorded for this account");
                    Vec::new()
                }
            }));
        }

        if self.keys {
            if record.keys.is_empty() {
                output::note("this account is watch-only: it has no keys");
            }
            return Ok(Some(record.keys.clone()));
        }

        if self.default_signer {
            // The same default-or-sole-key rule used by automatic tx signing.
            // Signing additionally checks address identity and ambiguity.
            return match record.signer_key(None) {
                Ok(id) => Ok(Some(vec![id.to_string()])),
                // `signer_key`'s own message for this case tells the caller to
                // pass `--key`, which is a `tx sign` flag: sound advice
                // where it was written, a dead end from a command that has no
                // such flag and is only being asked to read a record.
                Err(_) if !record.keys.is_empty() => Err(Error::other(format!(
                    "{} has {} keys and names none of them as its default: record one \
                     with `account add {} --force --default-signer <KEY_ID>`",
                    self.alias,
                    record.keys.len(),
                    self.alias,
                ))),
                Err(error) => Err(error),
            };
        }

        Ok(None)
    }
}
