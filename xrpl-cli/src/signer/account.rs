//! Offline signer selection from the transaction's Account, never the CLI default.

use serde_json::Value;

use crate::error::Error;
use crate::store::Store;

/// Infer one key for the whole stream before unlocking or signing anything.
/// `None` means no local account is recorded, allowing the legacy seed prompt.
pub(super) fn default_key(store: &Store, transactions: &[Value]) -> Result<Option<String>, Error> {
    let mut address = None;
    for transaction in transactions {
        let current = transaction
            .get("Account")
            .and_then(Value::as_str)
            .filter(|value| xrpl::core::addresscodec::is_valid_classic_address(value))
            .ok_or_else(|| {
                Error::other("automatic signer selection requires a classic Account address")
            })?;
        if address.is_some_and(|previous| previous != current) {
            return Err(Error::other(
                "automatic signer selection requires one Account throughout the stream; \
                 sign each account separately or choose an explicit --key (-k) or seed source",
            ));
        }
        address = Some(current);
    }
    let address = address.ok_or_else(|| Error::other("no transaction to sign"))?;

    // tx new has already replaced an alias with its address. Looking up a key
    // by the Account text or the default account would select the wrong identity.
    let mut matching = Vec::new();
    for alias in store.account_aliases()? {
        let record = store.account(&alias)?;
        if record.address == address {
            matching.push((alias, record));
        }
    }
    let (alias, record) = match matching.as_slice() {
        [] => return Ok(None),
        [only] => only,
        _ => {
            return Err(Error::other(format!(
                "several saved accounts match {address} ({}); choose --key <KEY_ID> (-k)",
                matching
                    .iter()
                    .map(|(alias, _)| alias.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            )));
        }
    };
    let id = record.signer_key(None)?;
    // Also validate hand-edited records: a default must belong to this account.
    record.signer_key(Some(id))?;
    let key = store.key(id)?;
    if key.classic_address != address
        || xrpl::core::keypairs::derive_classic_address(&key.public_key)? != address
    {
        return Err(Error::other(format!(
            "{alias}'s key {id} derives a different address from Account; \
             regular keys require explicit --key {id} (-k). \
             For a signer-list member, also pass --multisign",
        )));
    }
    Ok(Some(id.to_string()))
}
