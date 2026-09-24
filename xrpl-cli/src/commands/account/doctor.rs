//! `xrpl account doctor` — what is wrong with a record.
//!
//! Three kinds of problem, and the point of the command is that they are
//! genuinely different:
//!
//! - **A dangling key reference.** The account names a key record that is not
//!   here. Because key ids are names people choose, this is what a rename looks
//!   like from the account's side.
//! - **A secret that is unreachable from this machine.** The key record is
//!   present and whatever it points at is not — a credential-store entry made
//!   somewhere else, a file that did not sync. Records sync between machines and
//!   secrets deliberately do not, so in a ceremony this is the normal state, not
//!   a fault.
//! - **A local record that disagrees with the ledger.** Only a node can answer
//!   this, so it needs `--url`. An offline record cannot know that the account
//!   rotated its `RegularKey`, changed its `SignerList` or disabled its master
//!   key — and when it has, signing keeps succeeding locally and submission
//!   fails `tefBAD_AUTH`.

use serde_json::{json, Value};
use xrpl::asynch::clients::{AsyncJsonRpcClient, XRPLAsyncClient};
use xrpl::models::requests::generic_request::GenericRequest;

use crate::client;
use crate::commands::global::NetworkArgs;
use crate::error::Error;
use crate::store::{KeySource, Store};

#[derive(Debug, Clone, clap::Args)]
pub struct Cmd {
    /// The account to check. Omit to check every record.
    pub alias: Option<String>,

    /// Also reconcile against the ledger. Requires a node.
    #[arg(long)]
    pub ledger: bool,

    #[command(flatten)]
    pub network: NetworkArgs,

    /// Emit JSON.
    #[arg(long)]
    pub json: bool,
}

impl Cmd {
    pub fn run(&self) -> Result<(), Error> {
        let store = Store::from_env()?;

        let aliases = match &self.alias {
            Some(alias) => vec![alias.clone()],
            None => store.account_aliases()?,
        };

        let mut reports = Vec::with_capacity(aliases.len());
        for alias in &aliases {
            reports.push(self.check(&store, alias)?);
        }

        if self.json {
            return crate::output::artifact(&reports);
        }

        if reports.is_empty() {
            crate::output::note("no accounts recorded");
            return Ok(());
        }

        let mut problems = 0;
        for report in &reports {
            println!("{}", report["alias"].as_str().unwrap_or_default());
            for finding in report["findings"].as_array().into_iter().flatten() {
                problems += 1;
                println!("  {}", finding.as_str().unwrap_or_default());
            }
            if report["findings"].as_array().is_some_and(Vec::is_empty) {
                println!("  ok");
            }
        }

        if problems > 0 {
            crate::output::note(format!("{problems} finding(s)"));
        }

        Ok(())
    }

    fn check(&self, store: &Store, alias: &str) -> Result<Value, Error> {
        let record = store.account(alias)?;
        let mut findings: Vec<String> = Vec::new();

        for key_id in &record.keys {
            match store.key(key_id) {
                Err(_) => findings.push(format!(
                    "key {key_id} is referenced but has no record. Key ids are names \
                     you chose, so this is what renaming one looks like from here."
                )),
                Ok(key) => match &key.source {
                    KeySource::WatchOnly => findings.push(format!(
                        "key {key_id} is watch-only: it can be recognized, not used"
                    )),
                    KeySource::EncryptedFile { path } if !std::path::Path::new(path).exists() => {
                        findings.push(format!(
                            "key {key_id}: the record is here and {path} is not. Records \
                             sync between machines and secrets do not, so in a ceremony \
                             this is expected."
                        ))
                    }
                    _ => {}
                },
            }
        }

        if let Some(default) = &record.default_signer {
            if !record.keys.iter().any(|key| key == default) {
                findings.push(format!(
                    "default signer {default} is not one of this account's keys"
                ));
            }
        }

        if self.ledger {
            findings.extend(self.reconcile(&record.address)?);
        }

        Ok(json!({
            "alias": alias,
            "address": record.address,
            "findings": findings,
        }))
    }

    /// Ask the ledger what it thinks authorizes this account.
    fn reconcile(&self, address: &str) -> Result<Vec<String>, Error> {
        let url = self.network.url_or_mainnet();
        let runtime = client::runtime()?;

        let result = runtime.block_on(async {
            let node = AsyncJsonRpcClient::connect(client::parse_url(&url)?);
            let mut params = serde_json::Map::new();
            params.insert("account".into(), Value::String(address.to_string()));
            params.insert("signer_lists".into(), Value::Bool(true));
            params.insert("ledger_index".into(), Value::String("validated".into()));

            let response = node
                .request(
                    GenericRequest::builder("account_info")
                        .params(params)
                        .build()
                        .into(),
                )
                .await
                .map_err(Error::Client)?;

            client::result_value(&response)
        })?;

        let mut findings = Vec::new();

        if let Some(error) = result["error"].as_str() {
            findings.push(format!(
                "the ledger does not know this account yet ({error})"
            ));
            return Ok(findings);
        }

        let data = &result["account_data"];

        // `lsfDisableMaster` is 0x00100000. Once it is set the master key cannot
        // sign, and a local record that still names it keeps signing happily.
        const DISABLE_MASTER: u64 = 0x0010_0000;
        if data["Flags"].as_u64().unwrap_or(0) & DISABLE_MASTER != 0 {
            findings.push(
                "the master key is disabled on-ledger: only the regular key or the \
                 signer list can authorize this account"
                    .into(),
            );
        }

        if let Some(regular) = data["RegularKey"].as_str() {
            findings.push(format!(
                "a regular key is set on-ledger ({regular}); signing with the master \
                 key may not be what this account wants"
            ));
        }

        if let Some(lists) = result["account_data"]["signer_lists"]
            .as_array()
            .or_else(|| result["signer_lists"].as_array())
        {
            for list in lists {
                let quorum = list["SignerQuorum"].as_u64().unwrap_or(0);
                let entries = list["SignerEntries"].as_array().map(Vec::len).unwrap_or(0);
                findings.push(format!(
                    "a signer list is set on-ledger: {entries} entries, quorum {quorum}"
                ));
            }
        }

        Ok(findings)
    }
}
