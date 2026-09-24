//! `xrpl tx sign` — attach a signature. Offline; no URL, no network.

use serde_json::Value;
use xrpl::core::binarycodec::encode_for_signing_unframed;
use xrpl::signer::{RawSigner, SignOutcome, SigningDomain};

use crate::commands::tx::args::TxInput;
use crate::commands::tx::{io, signers, EMPTY_SIGNING_PUB_KEY};
use crate::error::Error;
use crate::output;
use crate::signer::SigningArgs;

#[derive(Debug, Clone, clap::Args)]
pub struct Cmd {
    #[command(flatten)]
    pub input: TxInput,

    #[command(flatten)]
    pub signing: SigningArgs,

    /// Add a signature to `Signers` instead of signing the transaction outright.
    ///
    /// For an account whose master key is disabled and whose authority is a
    /// signer list. Repeat the stage — `tx sign --multisign | tx sign
    /// --multisign` — to collect a quorum in one pipeline, or collect
    /// independently and combine the copies with `tx merge`.
    #[arg(long)]
    pub multisign: bool,
}

impl Cmd {
    pub fn run(&self) -> Result<(), Error> {
        // Resolve the key before anything else, and before any runtime exists:
        // an interactive prompt inside a `block_on` panics, and only on the
        // interactive path, so CI would never see it.
        let wallet = self.signing.resolve()?;

        // A plain `s…` seed always derives secp256k1. A seed a user believes is
        // Ed25519 therefore signs validly, as an address they do not control —
        // visible only as `tefBAD_AUTH`, long after the fact. Saying which
        // address is signing is the cheapest possible guard.
        output::note(format!(
            "signing as {} ({:?})",
            wallet.classic_address,
            wallet.algorithm()
        ));

        let mut transactions = io::read_txs(&self.input.tx)?;
        for transaction in &mut transactions {
            if self.multisign {
                multisign_one(transaction, &wallet)?;
            } else {
                sign_one(transaction, &wallet)?;
            }
        }

        io::write_txs(&transactions)
    }
}

/// Sign one transaction in place.
fn sign_one(transaction: &mut Value, signer: &impl RawSigner) -> Result<(), Error> {
    let object = transaction
        .as_object_mut()
        .ok_or_else(|| Error::other("expected a transaction object"))?;

    if object.contains_key("TxnSignature") {
        return Err(Error::other(
            "already signed: `tx sign` would overwrite the signature. \
             Use `tx sign --multisign` to add a signer to a multisigned transaction.",
        ));
    }

    // A multisigned transaction has no `TxnSignature` and an empty
    // `SigningPubKey` — exactly the shape checked above — so without this it
    // would sail through, and setting `SigningPubKey` would quietly re-point
    // the transaction at the master key and strand every signature collected
    // so far. Nothing would report it until `tefBAD_SIGNATURE`.
    if object
        .get("Signers")
        .and_then(Value::as_array)
        .is_some_and(|signers| !signers.is_empty())
    {
        return Err(Error::other(
            "this transaction already carries multisignatures: signing it outright would \
             authorise against the master key and orphan them. Use `tx sign --multisign`.",
        ));
    }

    match object.get("SigningPubKey") {
        Some(Value::String(key)) if key.is_empty() => {}
        Some(Value::String(_)) => {
            return Err(Error::other(
                "SigningPubKey is already set: this transaction was prepared for another key",
            ))
        }
        _ => {
            return Err(Error::other(
                "SigningPubKey is missing. An absent key and an empty one are different \
                 bytes on the wire, so the transaction has to say which it is.",
            ))
        }
    }

    object.insert(
        "SigningPubKey".into(),
        Value::String(signer.public_key().to_string()),
    );

    // The signer frames the payload; this hands it the unframed serialization
    // and a domain. See `xrpl::signer` for why framing lives on that side.
    let unframed = hex::decode(encode_for_signing_unframed(&*object)?)?;
    let outcome = signer
        .sign(SigningDomain::Single, &unframed)
        .map_err(|error| Error::other(error.to_string()))?;

    let signature = match outcome {
        SignOutcome::Signed(signature) => signature,
        SignOutcome::Partial { signature, .. } => signature,
        SignOutcome::SignedAndSubmitted(report) => {
            // A stage that consumes a transaction and produces none cannot sit
            // in a pipe, and submitting again would duplicate it.
            return Err(Error::other(format!(
                "this signer submitted the transaction itself ({}); it cannot be used in a pipeline",
                report.outcome
            )));
        }
        // `SignOutcome` is `#[non_exhaustive]` so backends can add outcomes
        // without breaking every consumer. An unknown one is not a signature.
        other => {
            return Err(Error::other(format!(
                "signer returned an outcome this version does not understand: {other:?}"
            )))
        }
    };

    object.insert("TxnSignature".into(), Value::String(signature));

    Ok(())
}

/// Add one signer's contribution to a multisigned transaction.
///
/// The transaction-level `SigningPubKey` stays empty — that is what tells
/// rippled to authorise against the signer list rather than the master key —
/// and the signature goes into `Signers` instead of `TxnSignature`.
fn multisign_one(transaction: &mut Value, signer: &impl RawSigner) -> Result<(), Error> {
    refuse_multisigning_a_master_key_action(transaction)?;

    let object = transaction
        .as_object_mut()
        .ok_or_else(|| Error::other("expected a transaction object"))?;

    if object.contains_key("TxnSignature") {
        return Err(Error::other(
            "this transaction is single-signed: a multisigned transaction carries its \
             signatures in Signers and must leave TxnSignature absent",
        ));
    }

    // Must be present and empty. Absent and empty are different bytes in the
    // STObject, so two signers disagreeing here sign two different digests and
    // nothing verifies until `tefBAD_SIGNATURE`.
    match object.get("SigningPubKey") {
        Some(Value::String(key)) if key.is_empty() => {}
        Some(Value::String(_)) => {
            return Err(Error::other(
                "SigningPubKey is set: that authorises against the master key, not a \
                 signer list. Rebuild the transaction to multisign it.",
            ))
        }
        _ => {
            return Err(Error::other(
                "SigningPubKey is missing. An absent key and an empty one are different \
                 bytes on the wire, so the transaction has to say which it is.",
            ))
        }
    }

    // The signer's own address, not the transaction's `Account`: in a 2-of-3
    // ceremony those are different, and only the signing key's address belongs
    // in the multisigning pre-image.
    let account = signer
        .classic_address()
        .map_err(|error| Error::other(error.to_string()))?;

    let unframed = hex::decode(encode_for_signing_unframed(&*object)?)?;
    let outcome = signer
        .sign(SigningDomain::MultiAs(&account), &unframed)
        .map_err(|error| Error::other(error.to_string()))?;

    let signature = match outcome {
        SignOutcome::Signed(signature) => signature,
        SignOutcome::Partial { signature, .. } => signature,
        SignOutcome::SignedAndSubmitted(report) => {
            return Err(Error::other(format!(
            "this signer submitted the transaction itself ({}); it cannot be used in a pipeline",
            report.outcome
        )))
        }
        other => {
            return Err(Error::other(format!(
                "signer returned an outcome this version does not understand: {other:?}"
            )))
        }
    };

    signers::append(
        transaction,
        signers::entry(&account, signer.public_key(), &signature),
    )?;

    // Advisory: the fee was fixed before any signature existed, so a shortfall
    // is only discoverable now, and only against the reference fee.
    let count = transaction["Signers"].as_array().map_or(0, Vec::len);
    if let Some(warning) = signers::fee_shortfall(transaction, count) {
        output::warn(warning);
    }

    Ok(())
}

/// Refuse to multisign an action rippled will only accept from the master key.
///
/// `AccountSet` with `SetFlag: asfDisableMaster` (4) is the one that matters:
/// it is how an account hands authority to its signer list, and it is exactly
/// the transaction someone reaches for once a signer list exists. Multisigning
/// it wastes a whole quorum collection on a `tecNO_PERMISSION`.
fn refuse_multisigning_a_master_key_action(transaction: &Value) -> Result<(), Error> {
    const ASF_DISABLE_MASTER: u64 = 4;

    let is_account_set =
        transaction.get("TransactionType").and_then(Value::as_str) == Some("AccountSet");
    let disables_master =
        transaction.get("SetFlag").and_then(Value::as_u64) == Some(ASF_DISABLE_MASTER);

    if is_account_set && disables_master {
        return Err(Error::other(
            "asfDisableMaster must be signed by the master key it disables, so this \
             cannot be multisigned. Sign it with `tx sign` using the account's own seed, \
             and do it after the SignerListSet, not before.",
        ));
    }

    Ok(())
}

/// Restore an unsigned transaction's empty `SigningPubKey`. Used by the tests
/// and by anything that needs to re-derive a pre-image.
pub fn clear_signature(transaction: &mut Value) {
    if let Some(object) = transaction.as_object_mut() {
        object.remove("TxnSignature");
        object.insert(
            "SigningPubKey".into(),
            Value::String(EMPTY_SIGNING_PUB_KEY.into()),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use xrpl::wallet::Wallet;

    const SEED: &str = "snoPBrXtMeMyMHUVTgbuqAfg1SUTb";

    fn unsigned() -> Value {
        json!({
            "TransactionType": "Payment",
            "Account": "rHb9CJAWyB4rj91VRWn96DkukG4bwdtyTh",
            "Destination": "rPT1Sjq2YGrBMTttX4GZHjKu9dyfzbpAYe",
            "Amount": "1000000",
            "Fee": "12",
            "Sequence": 1,
            "SigningPubKey": ""
        })
    }

    #[test]
    fn test_signing_sets_both_fields() {
        let wallet = Wallet::new(SEED, 0).expect("wallet");
        let mut tx = unsigned();

        sign_one(&mut tx, &wallet).expect("signs");

        assert_eq!(tx["SigningPubKey"], json!(wallet.public_key));
        assert!(tx["TxnSignature"].as_str().is_some_and(|s| !s.is_empty()));
    }

    #[test]
    fn test_a_transaction_missing_signing_pub_key_is_refused() {
        let wallet = Wallet::new(SEED, 0).expect("wallet");
        let mut tx = unsigned();
        tx.as_object_mut().unwrap().remove("SigningPubKey");

        let message = sign_one(&mut tx, &wallet).unwrap_err().to_string();
        assert!(message.contains("SigningPubKey is missing"), "{message}");
    }

    #[test]
    fn test_signing_twice_is_refused_rather_than_overwriting() {
        let wallet = Wallet::new(SEED, 0).expect("wallet");
        let mut tx = unsigned();
        sign_one(&mut tx, &wallet).expect("signs");

        assert!(sign_one(&mut tx, &wallet).is_err());
    }

    /// Two signers whose keys are generated fresh, so the test does not depend
    /// on a fixture's ordering.
    fn two_signers() -> (Wallet, Wallet) {
        (
            Wallet::create(None).expect("first wallet"),
            Wallet::create(None).expect("second wallet"),
        )
    }

    /// Verify one `Signers` entry the way rippled would: rebuild that signer's
    /// own pre-image and check the signature against it.
    fn entry_verifies(transaction: &Value, account: &str) -> bool {
        let entry = transaction["Signers"]
            .as_array()
            .expect("Signers")
            .iter()
            .find(|entry| entry["Signer"]["Account"] == json!(account))
            .expect("entry for account");

        let signature = entry["Signer"]["TxnSignature"].as_str().expect("signature");
        let public_key = entry["Signer"]["SigningPubKey"].as_str().expect("key");

        let framed = xrpl::core::binarycodec::encode_for_multisigning(transaction, account.into())
            .expect("encodes");
        let bytes = hex::decode(framed).expect("hex");

        xrpl::core::keypairs::is_valid_message(&bytes, signature, public_key)
    }

    #[test]
    fn test_a_multisignature_verifies_against_its_own_pre_image() {
        let (alice, _) = two_signers();
        let mut tx = unsigned();

        multisign_one(&mut tx, &alice).expect("multisigns");

        assert!(
            entry_verifies(&tx, &alice.classic_address),
            "the signature does not verify against the pre-image it was made for"
        );
    }

    #[test]
    fn test_a_second_signer_does_not_invalidate_the_first() {
        let (alice, bob) = two_signers();
        let mut tx = unsigned();

        multisign_one(&mut tx, &alice).expect("alice signs");
        multisign_one(&mut tx, &bob).expect("bob signs");

        // The property the whole pipe format rests on: `Signers` is a
        // non-signing field, so appending to it cannot invalidate what is
        // already there. If this ever fails, a collected quorum dies as
        // `tefBAD_SIGNATURE` with nothing to point at.
        assert!(entry_verifies(&tx, &alice.classic_address), "alice broke");
        assert!(entry_verifies(&tx, &bob.classic_address), "bob broke");
    }

    #[test]
    fn test_multisigning_leaves_the_single_signature_fields_alone() {
        let (alice, _) = two_signers();
        let mut tx = unsigned();

        multisign_one(&mut tx, &alice).expect("multisigns");

        // An empty transaction-level key is what selects signer-list authority;
        // a set one would authorise against the master key instead.
        assert_eq!(tx["SigningPubKey"], json!(""));
        assert!(tx.get("TxnSignature").is_none());
    }

    #[test]
    fn test_multisigning_a_master_key_action_is_refused() {
        let (alice, _) = two_signers();
        let mut tx = json!({
            "TransactionType": "AccountSet",
            "Account": "rHb9CJAWyB4rj91VRWn96DkukG4bwdtyTh",
            "SetFlag": 4,
            "Fee": "12",
            "Sequence": 1,
            "SigningPubKey": ""
        });

        let message = multisign_one(&mut tx, &alice).unwrap_err().to_string();

        assert!(message.contains("asfDisableMaster"), "{message}");
        // Refused before any network call and before any signature is attached.
        assert!(tx.get("Signers").is_none());
    }

    #[test]
    fn test_multisigning_an_already_single_signed_transaction_is_refused() {
        let (alice, bob) = two_signers();
        let mut tx = unsigned();
        sign_one(&mut tx, &alice).expect("single-signs");

        let message = multisign_one(&mut tx, &bob).unwrap_err().to_string();
        assert!(message.contains("single-signed"), "{message}");
    }

    #[test]
    fn test_single_signing_a_multisigned_transaction_is_refused() {
        let (alice, bob) = two_signers();
        let mut tx = unsigned();
        multisign_one(&mut tx, &alice).expect("multisigns");

        // `sign_one` would otherwise set SigningPubKey, silently switching the
        // transaction from signer-list authority to master-key authority.
        let message = sign_one(&mut tx, &bob).unwrap_err().to_string();
        assert!(
            message.contains("already carries multisignatures"),
            "{message}"
        );

        // And alice's signature is still intact and verifying.
        assert!(entry_verifies(&tx, &alice.classic_address));
    }

    #[test]
    fn test_the_signature_verifies_against_the_pre_image() {
        let wallet = Wallet::new(SEED, 0).expect("wallet");
        let mut tx = unsigned();
        sign_one(&mut tx, &wallet).expect("signs");

        let signature = tx["TxnSignature"].as_str().expect("signature").to_string();
        let public_key = tx["SigningPubKey"].as_str().expect("key").to_string();

        // Re-derive the framed pre-image exactly as a verifier would.
        let framed = xrpl::core::binarycodec::encode_for_signing(&tx).expect("encodes");
        let bytes = hex::decode(framed).expect("hex");

        assert!(
            xrpl::core::keypairs::is_valid_message(&bytes, &signature, &public_key),
            "signature does not verify against the transaction it was made for"
        );
    }
}
