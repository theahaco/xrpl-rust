//! The `Signers` array: appending one entry, and the ordering rule.
//!
//! Shared by `tx sign --multisign`, which produces one entry, and `tx merge`,
//! which collects entries made independently. Both are the same operation —
//! add to the array, then re-sort it — so both call the same code.
//!
//! Why appending is safe at all: `Signers` is `isSerialized: true,
//! isSigningField: false`, and the multisigning pre-image is built with
//! `signing_only = true`. The array is therefore not part of what any signer
//! signed, so adding a second signature provably cannot invalidate the first.
//! That property is what lets a partially-signed transaction travel down a pipe
//! as ordinary JSON.

use serde_json::{json, Value};
use xrpl::core::addresscodec::decode_classic_address;

use crate::error::Error;

/// The reference fee, in drops. Used only to sanity-check a `Fee` that was
/// committed before the signatures existed; the node is the real authority.
const REFERENCE_FEE_DROPS: u64 = 10;

/// One entry's signing account, or an error naming what is malformed.
pub fn signer_account(entry: &Value) -> Result<&str, Error> {
    entry
        .get("Signer")
        .and_then(|signer| signer.get("Account"))
        .and_then(Value::as_str)
        .ok_or_else(|| Error::other(format!("malformed Signers entry: {entry}")))
}

/// Build one `Signers` entry.
pub fn entry(account: &str, public_key: &str, signature: &str) -> Value {
    json!({
        "Signer": {
            "Account": account,
            "SigningPubKey": public_key,
            "TxnSignature": signature,
        }
    })
}

/// Append an entry to a transaction's `Signers`, then re-sort.
///
/// Refuses a second entry for an account that already signed: rippled rejects
/// the duplicate with `temBAD_SIGNER`, and finding that out at submit time
/// means the ceremony has to be redone.
pub fn append(transaction: &mut Value, new_entry: Value) -> Result<(), Error> {
    let account = signer_account(&new_entry)?.to_string();

    let object = transaction
        .as_object_mut()
        .ok_or_else(|| Error::other("expected a transaction object"))?;

    let list = object
        .entry("Signers")
        .or_insert_with(|| Value::Array(Vec::new()))
        .as_array_mut()
        .ok_or_else(|| Error::other("Signers is present but is not an array"))?;

    for existing in list.iter() {
        if signer_account(existing)? == account {
            return Err(Error::other(format!(
                "{account} has already signed this transaction; \
                 rippled rejects a duplicate signer with temBAD_SIGNER"
            )));
        }
    }

    list.push(new_entry);
    sort(list)
}

/// Sort a `Signers` array into the order the codec requires: ascending by the
/// signer's decoded 20-byte AccountID, which is **not** the same as sorting the
/// base58 text.
pub fn sort(list: &mut [Value]) -> Result<(), Error> {
    // Decode up front so the comparator cannot fail; sorting is infallible and
    // a malformed address has to be an error, not a panic inside `sort_by`.
    let mut decoded = Vec::with_capacity(list.len());
    for entry in list.iter() {
        let account = signer_account(entry)?;
        let bytes = decode_classic_address(account).map_err(|error| {
            Error::other(format!("{account} is not a classic address: {error}"))
        })?;
        decoded.push((bytes, entry.clone()));
    }

    decoded.sort_by(|left, right| left.0.cmp(&right.0));

    for (slot, (_, entry)) in list.iter_mut().zip(decoded) {
        *slot = entry;
    }

    Ok(())
}

/// How many signers the transaction's `Fee` paid for, if it can be told.
///
/// A multisigned transaction must pay `(N + 1) × reference_fee` for N entries.
/// `Fee` is a signing field and XRPL has no fee-bump wrapper, so it is fixed
/// before the first signature exists — which makes an under-count invisible
/// until `telINSUF_FEE_P` at submit, after the whole collection is done.
///
/// Advisory in one direction only: it assumes the reference fee, so it reports
/// a shortfall it is sure of and stays quiet otherwise.
pub fn fee_shortfall(transaction: &Value, signer_count: usize) -> Option<String> {
    let fee: u64 = transaction.get("Fee")?.as_str()?.parse().ok()?;
    let required = (signer_count as u64 + 1) * REFERENCE_FEE_DROPS;

    (fee < required).then(|| {
        format!(
            "Fee is {fee} drops but {signer_count} signers need at least {required} \
             at the reference fee; rippled will return telINSUF_FEE_P. \
             Rebuild with `tx autofill --signers {signer_count}` — the fee is signed, \
             so it cannot be raised now."
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    // Three accounts whose decoded AccountID order differs from their base58
    // order, so a test that passes here would fail under a naive string sort.
    const ALICE: &str = "rHb9CJAWyB4rj91VRWn96DkukG4bwdtyTh";
    const BOB: &str = "rPT1Sjq2YGrBMTttX4GZHjKu9dyfzbpAYe";
    const CAROL: &str = "rLNaPoKeeBjZe2qs6x52yVPZpZ8td4dc6w";

    fn tx() -> Value {
        json!({"TransactionType": "Payment", "Fee": "30", "SigningPubKey": ""})
    }

    fn accounts(transaction: &Value) -> Vec<String> {
        transaction["Signers"]
            .as_array()
            .expect("Signers")
            .iter()
            .map(|entry| signer_account(entry).expect("account").to_string())
            .collect()
    }

    #[test]
    fn test_entries_sort_by_decoded_account_id_not_base58() {
        let mut transaction = tx();
        for account in [BOB, ALICE, CAROL] {
            append(&mut transaction, entry(account, "ED00", "DEAD")).expect("appends");
        }

        let mut expected = vec![ALICE.to_string(), BOB.to_string(), CAROL.to_string()];
        expected.sort_by_key(|account| decode_classic_address(account).expect("decodes"));

        assert_eq!(accounts(&transaction), expected);
    }

    #[test]
    fn test_order_does_not_depend_on_insertion_order() {
        let mut forwards = tx();
        let mut backwards = tx();

        for account in [ALICE, BOB, CAROL] {
            append(&mut forwards, entry(account, "ED00", "DEAD")).expect("appends");
        }
        for account in [CAROL, BOB, ALICE] {
            append(&mut backwards, entry(account, "ED00", "DEAD")).expect("appends");
        }

        assert_eq!(accounts(&forwards), accounts(&backwards));
    }

    #[test]
    fn test_a_duplicate_signer_is_refused() {
        let mut transaction = tx();
        append(&mut transaction, entry(ALICE, "ED00", "DEAD")).expect("appends");

        let message = append(&mut transaction, entry(ALICE, "ED00", "BEEF"))
            .unwrap_err()
            .to_string();

        assert!(message.contains("already signed"), "{message}");
        assert!(message.contains("temBAD_SIGNER"), "{message}");
    }

    #[test]
    fn test_appending_preserves_the_earlier_entry() {
        let mut transaction = tx();
        append(&mut transaction, entry(ALICE, "ED00", "AAAA")).expect("appends");
        append(&mut transaction, entry(BOB, "ED11", "BBBB")).expect("appends");

        let found = transaction["Signers"]
            .as_array()
            .expect("Signers")
            .iter()
            .find(|entry| signer_account(entry).expect("account") == ALICE)
            .expect("alice survives");

        assert_eq!(found["Signer"]["TxnSignature"], json!("AAAA"));
    }

    #[test]
    fn test_a_fee_that_covers_the_signers_reports_nothing() {
        // 2 signers need (2 + 1) * 10 = 30.
        assert!(fee_shortfall(&tx(), 2).is_none());
    }

    #[test]
    fn test_an_underpaid_fee_is_reported() {
        let message = fee_shortfall(&tx(), 3).expect("3 signers need 40, not 30");
        assert!(message.contains("telINSUF_FEE_P"), "{message}");
    }

    #[test]
    fn test_an_unparseable_fee_is_not_guessed_at() {
        let transaction = json!({"Fee": "not-a-number"});
        assert!(fee_shortfall(&transaction, 9).is_none());
    }
}
