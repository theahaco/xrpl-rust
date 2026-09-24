//! `xrpl tx sign` — attach a signature. Offline; no URL, no network.

use serde_json::Value;
use xrpl::core::binarycodec::encode_for_signing_unframed;
use xrpl::signer::{RawSigner, SignOutcome, SigningDomain};

use crate::commands::tx::args::TxInput;
use crate::commands::tx::{io, EMPTY_SIGNING_PUB_KEY};
use crate::error::Error;
use crate::output;
use crate::signer::SigningArgs;

#[derive(Debug, Clone, clap::Args)]
pub struct Cmd {
    #[command(flatten)]
    pub input: TxInput,

    #[command(flatten)]
    pub signing: SigningArgs,
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
            sign_one(transaction, &wallet)?;
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
