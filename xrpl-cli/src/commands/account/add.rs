//! `xrpl account add` — record an account.

use crate::error::Error;
use crate::output;
use crate::store::{AccountRecord, Store};

#[derive(Debug, Clone, clap::Args)]
pub struct Cmd {
    /// What to call this account.
    pub alias: String,

    /// The classic r-address. Optional when --key can supply it.
    ///
    /// An account's address derives from its original master public key, so
    /// when that key is one of the ones being recorded the address is already
    /// on disk, and asking for it again only means piping `key show` through
    /// `jq`.
    #[arg(long, value_name = "R_ADDRESS")]
    pub address: Option<String>,

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
    ///
    /// Only needed when there are several. One key is the default by being the
    /// only one, and `account show` says so.
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

        let mut derived = Vec::new();
        for key in &self.keys {
            // Naming the same key twice would leave an account that has one
            // key and cannot tell that it does: the resolution that makes a
            // lone key the default counts entries, so the duplicate turns into
            // "this account has several keys and no default" at signing time.
            if derived
                .iter()
                .any(|(seen, _): &(String, String)| seen == key)
            {
                return Err(Error::other(format!("--key {key} was given twice")));
            }

            // Referring to a key that is not there would produce a record that
            // only fails later, at signing time.
            let record = store.key(key)?;
            derived.push((key.clone(), record.classic_address));
        }

        if let Some(default) = &self.default_signer {
            if !self.keys.iter().any(|key| key == default) {
                return Err(Error::other(format!(
                    "--default-signer {default} is not one of this account's keys"
                )));
            }
        }

        let mut record = AccountRecord::new(self.address(&store, &derived)?);
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

    /// The address to record: the one given, or the one the keys derive to.
    ///
    /// `--address` always wins when it is there, because deriving is only a
    /// default and not a rule. A regular key and a signer-list member each
    /// derive to their own address and not to the account's — which is the
    /// whole reason keys are a list rather than a field — so an account whose
    /// keys disagree is asked about rather than guessed at.
    fn address(&self, store: &Store, derived: &[(String, String)]) -> Result<String, Error> {
        if let Some(address) = &self.address {
            return Ok(address.clone());
        }

        let mut distinct: Vec<&(String, String)> = Vec::new();
        for pair in derived {
            if !distinct.iter().any(|seen| seen.1 == pair.1) {
                distinct.push(pair);
            }
        }

        match distinct.as_slice() {
            [pair] => {
                // Adding a key to an existing account is the obvious reason to
                // re-run this with `--force`, and until `--address` became
                // optional it had to be restated on every such run — so an
                // alias could not change identity by omission. A regular key
                // derives to its own address, so re-recording one that way
                // would silently repoint the alias at an account nobody here
                // controls, and `-q` would swallow the only note saying so.
                if self.force {
                    if let Ok(existing) = store.account(&self.alias) {
                        if existing.address != pair.1 {
                            return Err(Error::other(format!(
                                "{} is recorded as {}, and key {} derives to {}: pass --address \
                                 to say which this account is",
                                self.alias, existing.address, pair.0, pair.1
                            )));
                        }
                    }
                }

                // Where the address came from is never invisible, for the same
                // reason an inherited `--account` says so.
                output::note(format!("address {}, from key {}", pair.1, pair.0));
                Ok(pair.1.clone())
            }
            [] => Err(Error::other(
                "nothing to record: pass --address, or --key naming the key this \
                 account's address derives from",
            )),
            several => {
                let spelled = several
                    .iter()
                    .map(|pair| format!("{} is {}", pair.0, pair.1))
                    .collect::<Vec<_>>()
                    .join(", ");

                Err(Error::other(format!(
                    "these keys derive to different addresses ({spelled}), so which \
                     of them this account is cannot be guessed: pass --address"
                )))
            }
        }
    }
}
