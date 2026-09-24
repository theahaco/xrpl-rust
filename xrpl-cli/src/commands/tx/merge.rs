//! `xrpl tx merge` — combine independently signed copies of one transaction.
//!
//! The other half of the multisig ceremony. `tx sign --multisign` collects a
//! quorum serially, in one pipeline, when one person holds every key. `merge`
//! collects it in parallel, when they do not: each signer receives the same
//! prepared transaction, signs their copy wherever they are, and sends it back.
//!
//! Both do the same thing to the same field — append to `Signers`, re-sort —
//! so both call [`crate::commands::tx::signers`]. The difference is only where
//! the entries came from.
//!
//! Offline. Merging is a JSON operation; nothing here needs a node or a key.

use std::ffi::OsString;

use serde_json::Value;

use crate::commands::tx::{io, signers};
use crate::error::Error;
use crate::output;

#[derive(Debug, Clone, clap::Args)]
pub struct Cmd {
    /// The signed copies to combine. Each is transaction JSON, a file
    /// containing it, or `-` for stdin.
    #[arg(value_name = "TX", required = true, num_args = 1..)]
    pub inputs: Vec<OsString>,
}

impl Cmd {
    pub fn run(&self) -> Result<(), Error> {
        let mut copies = Vec::new();
        for input in &self.inputs {
            copies.extend(io::read_txs(&Some(input.clone()))?);
        }

        let merged = merge(copies)?;

        let count = merged["Signers"].as_array().map_or(0, Vec::len);
        output::note(format!("merged {count} signatures"));
        if let Some(warning) = signers::fee_shortfall(&merged, count) {
            output::warn(warning);
        }

        io::write_txs(std::slice::from_ref(&merged))
    }
}

/// Combine signed copies of one transaction into a single transaction.
fn merge(copies: Vec<Value>) -> Result<Value, Error> {
    let mut copies = copies.into_iter();
    let first = copies
        .next()
        .ok_or_else(|| Error::other("no transactions to merge"))?;

    // Everything except `Signers` has to be identical. `Signers` is the only
    // non-signing field involved, so any other difference means the copies were
    // signed over different pre-images and the signatures cannot all verify —
    // which rippled reports as `tefBAD_SIGNATURE`, after the ceremony is over.
    let expected = signing_fields(&first);

    let mut merged = first.clone();
    merged
        .as_object_mut()
        .ok_or_else(|| Error::other("expected a transaction object"))?
        .remove("Signers");

    let mut entries = collect_entries(&first)?;

    for (index, copy) in copies.enumerate() {
        if signing_fields(&copy) != expected {
            return Err(Error::other(format!(
                "input {} is not the same transaction as the first one. \
                 Every copy has to be signed over identical fields — re-distribute \
                 one prepared transaction and have each signer sign that.",
                index + 2
            )));
        }

        entries.extend(collect_entries(&copy)?);
    }

    for entry in entries {
        signers::append(&mut merged, entry)?;
    }

    Ok(merged)
}

/// Everything a signer signed over: the transaction without its `Signers`.
fn signing_fields(transaction: &Value) -> Value {
    let mut fields = transaction.clone();
    if let Some(object) = fields.as_object_mut() {
        object.remove("Signers");
    }

    fields
}

/// The `Signers` entries one copy carries.
fn collect_entries(transaction: &Value) -> Result<Vec<Value>, Error> {
    match transaction.get("Signers") {
        None => Ok(Vec::new()),
        Some(Value::Array(entries)) => {
            // Reject malformed entries here rather than letting them through to
            // a sort that would have to guess.
            for entry in entries {
                signers::signer_account(entry)?;
            }

            Ok(entries.clone())
        }
        Some(other) => Err(Error::other(format!(
            "Signers is present but is not an array: {other}"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const ALICE: &str = "rHb9CJAWyB4rj91VRWn96DkukG4bwdtyTh";
    const BOB: &str = "rPT1Sjq2YGrBMTttX4GZHjKu9dyfzbpAYe";

    fn prepared() -> Value {
        json!({
            "TransactionType": "Payment",
            "Account": "rLNaPoKeeBjZe2qs6x52yVPZpZ8td4dc6w",
            "Destination": BOB,
            "Amount": "1000000",
            "Fee": "30",
            "Sequence": 1,
            "SigningPubKey": ""
        })
    }

    fn signed_by(account: &str, signature: &str) -> Value {
        let mut copy = prepared();
        copy.as_object_mut().unwrap().insert(
            "Signers".into(),
            json!([signers::entry(account, "ED00", signature)]),
        );
        copy
    }

    fn accounts(transaction: &Value) -> Vec<String> {
        transaction["Signers"]
            .as_array()
            .expect("Signers")
            .iter()
            .map(|entry| signers::signer_account(entry).unwrap().to_string())
            .collect()
    }

    #[test]
    fn test_two_copies_combine_into_one_transaction() {
        let merged = merge(vec![signed_by(ALICE, "AAAA"), signed_by(BOB, "BBBB")]).expect("merges");

        assert_eq!(merged["Signers"].as_array().expect("Signers").len(), 2);
        assert!(accounts(&merged).contains(&ALICE.to_string()));
        assert!(accounts(&merged).contains(&BOB.to_string()));
    }

    #[test]
    fn test_merging_is_order_independent() {
        let forwards = merge(vec![signed_by(ALICE, "AAAA"), signed_by(BOB, "BBBB")]).expect("a");
        let backwards = merge(vec![signed_by(BOB, "BBBB"), signed_by(ALICE, "AAAA")]).expect("b");

        // Byte-identical, because the codec's order is a property of the
        // signers, not of the order the copies came back in.
        assert_eq!(forwards, backwards);
    }

    #[test]
    fn test_the_merged_transaction_keeps_the_signed_fields() {
        let merged = merge(vec![signed_by(ALICE, "AAAA"), signed_by(BOB, "BBBB")]).expect("merges");

        for field in ["TransactionType", "Account", "Destination", "Amount", "Fee"] {
            assert_eq!(merged[field], prepared()[field], "{field} changed");
        }
        assert_eq!(merged["SigningPubKey"], json!(""));
    }

    #[test]
    fn test_copies_of_different_transactions_are_refused() {
        let mut tampered = signed_by(BOB, "BBBB");
        tampered.as_object_mut().unwrap().insert(
            "Amount".into(),
            // The attack this rejects: a co-signer returns a copy paying more.
            json!("9000000"),
        );

        let message = merge(vec![signed_by(ALICE, "AAAA"), tampered])
            .unwrap_err()
            .to_string();

        assert!(message.contains("not the same transaction"), "{message}");
    }

    #[test]
    fn test_the_same_signer_twice_is_refused() {
        let message = merge(vec![signed_by(ALICE, "AAAA"), signed_by(ALICE, "CCCC")])
            .unwrap_err()
            .to_string();

        assert!(message.contains("already signed"), "{message}");
    }

    #[test]
    fn test_a_single_copy_merges_to_itself() {
        let merged = merge(vec![signed_by(ALICE, "AAAA")]).expect("merges");
        assert_eq!(accounts(&merged), vec![ALICE.to_string()]);
    }

    #[test]
    fn test_an_unsigned_copy_contributes_nothing_but_is_not_an_error() {
        // Distributing the prepared transaction to three people and getting two
        // back signed is the normal case in a 2-of-3.
        let merged = merge(vec![prepared(), signed_by(ALICE, "AAAA")]).expect("merges");
        assert_eq!(accounts(&merged), vec![ALICE.to_string()]);
    }

    #[test]
    fn test_merging_nothing_is_an_error() {
        assert!(merge(Vec::new()).is_err());
    }
}
