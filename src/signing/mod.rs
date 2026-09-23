//! Pure cryptographic transaction signing.
//!
//! These functions don't touch the network — they only need the wallet's
//! private key plus the transaction. They live here (rather than under
//! `asynch::transaction`) so they compile and unit-test without enabling the
//! `helpers`/`json-rpc`/`websocket` features that pull in async client code.
//!
//! Re-exported from the legacy locations (`asynch::transaction::sign`,
//! `transaction::multisign`) for backward compatibility.

pub mod exceptions;

use core::fmt::Debug;

use alloc::string::String;
use alloc::string::ToString;
use alloc::vec::Vec;
use serde::Serialize;
use serde::{de::DeserializeOwned, Deserialize};
use strum::IntoEnumIterator;

use crate::asynch::exceptions::XRPLHelperResult;
use crate::core::{
    addresscodec::{decode_classic_address, is_valid_xaddress, xaddress_to_classic_address},
    binarycodec::{encode_for_multisigning, encode_for_signing},
    keypairs::sign as keypairs_sign,
};
use crate::models::{
    transactions::{Signer, Transaction},
    Model,
};
use crate::utils::transactions::{
    get_transaction_field_value, set_transaction_field_value, validate_transaction_has_field,
};
use crate::wallet::Wallet;

use exceptions::{XRPLMultisignException, XRPLSignTransactionException};

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
enum AccountFieldType {
    Account,
    Destination,
}

/// Sign a transaction with the given wallet's key.
///
/// Pure crypto — does not contact the network. When `multisign` is true the
/// signature is appended as a `Signer` entry; otherwise it goes into
/// `TxnSignature` directly.
pub fn sign<'a, T, F>(transaction: &mut T, wallet: &Wallet, multisign: bool) -> XRPLHelperResult<()>
where
    F: IntoEnumIterator + Serialize + Debug + PartialEq,
    T: Transaction<'a, F> + Model + Serialize + DeserializeOwned + Clone + Debug,
{
    transaction.validate()?;

    if multisign {
        // A multisigned transaction carries an empty `SigningPubKey`, and an
        // absent one is *different bytes* in the STObject (`7300` versus
        // nothing). This branch never calls `prepare_transaction`, so without
        // this the field is whatever the caller left — `Some("")` from the
        // builder, `None` from `Default` and from deserialization — and two
        // signers of the same logical transaction sign two different digests.
        transaction.get_mut_common_fields().signing_pub_key = Some("".into());

        let serialized_for_signing =
            encode_for_multisigning(transaction, wallet.classic_address.clone().into())?;
        let serialized_bytes = hex::decode(serialized_for_signing)?;
        let signature = keypairs_sign(&serialized_bytes, &wallet.private_key)?;
        let signer = Signer::new(
            wallet.classic_address.clone(),
            signature,
            wallet.public_key.clone(),
        );

        // Append, do not overwrite. Signing the same transaction twice with two
        // different keys is the whole point of multisigning; replacing the array
        // silently discarded every earlier signature and produced a below-quorum
        // transaction that failed on-ledger with an opaque error.
        let common_fields = transaction.get_mut_common_fields();
        let mut signers = common_fields.signers.take().unwrap_or_default();
        // Re-signing with the same key replaces that signer's entry rather than
        // adding a second one for the same account, which rippled rejects.
        signers.retain(|existing| existing.account != signer.account);
        signers.push(signer);
        sort_signers(&mut signers)?;
        common_fields.signers = Some(signers);

        Ok(())
    } else {
        prepare_transaction(transaction, wallet)?;
        let serialized_for_signing = encode_for_signing(transaction)?;
        let serialized_bytes = hex::decode(serialized_for_signing)?;
        let signature = keypairs_sign(&serialized_bytes, &wallet.private_key)?;
        transaction.get_mut_common_fields().txn_signature = Some(signature.into());

        Ok(())
    }
}

/// Combine signer-signed copies of `transaction` into a single multisigned
/// transaction. `tx_list` must contain copies of `transaction` each signed by
/// a different signer.
pub fn multisign<'a, T, F>(transaction: &mut T, tx_list: &'a Vec<T>) -> XRPLHelperResult<()>
where
    F: IntoEnumIterator + Serialize + Debug + PartialEq + 'a,
    T: Transaction<'a, F>,
{
    let mut decoded_tx_signers = Vec::new();
    for tx in tx_list {
        let tx_signers = match tx.get_common_fields().signers.as_ref() {
            Some(signers) => signers,
            None => return Err(XRPLMultisignException::NoSigners.into()),
        };
        let tx_signer = match tx_signers.first() {
            Some(signer) => signer,
            None => return Err(XRPLMultisignException::NoSigners.into()),
        };
        decoded_tx_signers.push(tx_signer.clone());
    }
    sort_signers(&mut decoded_tx_signers)?;
    transaction.get_mut_common_fields().signers = Some(decoded_tx_signers);
    transaction.get_mut_common_fields().signing_pub_key = Some("".into());

    Ok(())
}

/// Sort a `Signers` array into the order the ledger requires: ascending by the
/// signer's decoded 20-byte AccountID, not by its base58 spelling.
///
/// Fallible because the accounts are caller-supplied. This used to `.unwrap()`
/// the decode, so a malformed address reaching `multisign` aborted the process.
fn sort_signers(signers: &mut [Signer]) -> XRPLHelperResult<()> {
    let mut decoded = Vec::with_capacity(signers.len());
    for signer in signers.iter() {
        let account_id = decode_classic_address(signer.account.as_ref()).map_err(|_| {
            XRPLMultisignException::InvalidSignerAccount(signer.account.to_string())
        })?;
        decoded.push(account_id);
    }

    // Decode once per signer, then sort on the decoded keys.
    let mut order: Vec<usize> = (0..signers.len()).collect();
    order.sort_by(|&a, &b| decoded[a].cmp(&decoded[b]));

    let sorted: Vec<Signer> = order.iter().map(|&i| signers[i].clone()).collect();
    signers.clone_from_slice(&sorted);

    Ok(())
}

pub(crate) fn prepare_transaction<'a, T, F>(
    transaction: &mut T,
    wallet: &Wallet,
) -> XRPLHelperResult<()>
where
    F: IntoEnumIterator + Serialize + Debug + PartialEq,
    T: Transaction<'a, F> + Serialize + DeserializeOwned + Clone,
{
    let common_fields = transaction.get_mut_common_fields();
    common_fields.signing_pub_key = Some(wallet.public_key.clone().into());

    validate_account_xaddress(transaction, AccountFieldType::Account)?;
    if validate_transaction_has_field(transaction, "Destination").is_ok() {
        validate_account_xaddress(transaction, AccountFieldType::Destination)?;
    }

    let _ = convert_to_classic_address(transaction, "Unauthorize");
    let _ = convert_to_classic_address(transaction, "Authorize");
    // EscrowCancel, EscrowFinish
    let _ = convert_to_classic_address(transaction, "Owner");
    // SetRegularKey
    let _ = convert_to_classic_address(transaction, "RegularKey");

    Ok(())
}

fn validate_account_xaddress<'a, T, F>(
    prepared_transaction: &mut T,
    account_field: AccountFieldType,
) -> XRPLHelperResult<()>
where
    F: IntoEnumIterator + Serialize + Debug + PartialEq,
    T: Transaction<'a, F> + Serialize + DeserializeOwned + Clone,
{
    let (account_field_name, tag_field_name) = match account_field {
        AccountFieldType::Account => ("Account", "SourceTag"),
        AccountFieldType::Destination => ("Destination", "DestinationTag"),
    };
    let account_address = match account_field {
        AccountFieldType::Account => prepared_transaction.get_common_fields().account.clone(),
        AccountFieldType::Destination => {
            get_transaction_field_value(prepared_transaction, "Destination")?
        }
    };

    if is_valid_xaddress(&account_address) {
        let (address, tag, _) = xaddress_to_classic_address(&account_address)?;
        validate_transaction_has_field(prepared_transaction, account_field_name)?;
        set_transaction_field_value(prepared_transaction, account_field_name, address)?;

        if validate_transaction_has_field(prepared_transaction, tag_field_name).is_ok()
            && get_transaction_field_value(prepared_transaction, tag_field_name).unwrap_or(Some(0))
                != tag
        {
            Err(XRPLSignTransactionException::TagFieldMismatch(tag_field_name.to_string()).into())
        } else {
            set_transaction_field_value(prepared_transaction, tag_field_name, tag)?;

            Ok(())
        }
    } else {
        Ok(())
    }
}

fn convert_to_classic_address<'a, T, F>(
    transaction: &mut T,
    field_name: &str,
) -> XRPLHelperResult<()>
where
    F: IntoEnumIterator + Serialize + Debug + PartialEq,
    T: Transaction<'a, F> + Serialize + DeserializeOwned + Clone,
{
    let address = get_transaction_field_value::<F, _, String>(transaction, field_name)?;
    if is_valid_xaddress(&address) {
        let classic_address = xaddress_to_classic_address(&address)?.0;
        Ok(set_transaction_field_value(
            transaction,
            field_name,
            classic_address,
        )?)
    } else {
        Ok(())
    }
}

#[cfg(all(test, feature = "wallet", feature = "models"))]
mod tests {
    use super::*;
    use crate::models::transactions::payment::Payment;
    use crate::models::transactions::{
        CommonFields, CommonTransactionBuilder as _, Transaction as _, TransactionType,
    };
    use crate::models::Amount;
    use alloc::borrow::Cow;
    // `vec!` is not in the prelude under no_std, and CI builds this test cfg
    // against the embassy feature set.
    use alloc::vec;

    /// Two unrelated seeds, so the derived signers are two distinct accounts.
    const SEED_A: &str = "sEdSKaCy2JT7JaM7v95H9SxkhP9wS2r";
    const SEED_B: &str = "sp5fghtJtpUorTwvof1NpDXAzNwf5";
    const SEED_C: &str = "snoPBrXtMeMyMHUVTgbuqAfg1SUTb";

    fn payment(account: &str) -> Payment<'static> {
        Payment {
            common_fields: CommonFields {
                account: Cow::Owned(account.to_string()),
                transaction_type: TransactionType::Payment,
                ..Default::default()
            },
            amount: Amount::XRPAmount("1000000".into()),
            destination: "ra5nK24KXen9AHvsdFTKHSANinZseWnPcX".into(),
            ..Default::default()
        }
        .with_fee("30")
        .with_sequence(1)
    }

    #[test]
    fn test_multisign_appends_rather_than_overwriting() {
        let alice = Wallet::new(SEED_A, 0).expect("wallet a");
        let bob = Wallet::new(SEED_B, 0).expect("wallet b");

        let mut tx = payment(&alice.classic_address);
        sign(&mut tx, &alice, true).expect("alice signs");
        sign(&mut tx, &bob, true).expect("bob signs");

        let signers = Transaction::get_common_fields(&tx)
            .signers
            .as_ref()
            .expect("signers present");

        // Before this fix the second call replaced the array, leaving one entry
        // and a transaction that failed quorum on-ledger with no local error.
        assert_eq!(signers.len(), 2, "both signatures must survive");
        assert!(signers.iter().any(|s| s.account == alice.classic_address));
        assert!(signers.iter().any(|s| s.account == bob.classic_address));
    }

    #[test]
    fn test_multisign_sorts_by_decoded_account_id() {
        let wallets: Vec<Wallet> = [SEED_A, SEED_B, SEED_C]
            .iter()
            .map(|seed| Wallet::new(seed, 0).expect("wallet"))
            .collect();

        let mut tx = payment(&wallets[0].classic_address);
        for wallet in &wallets {
            sign(&mut tx, wallet, true).expect("sign");
        }

        let signers = Transaction::get_common_fields(&tx)
            .signers
            .as_ref()
            .expect("signers");
        assert_eq!(signers.len(), 3);

        // The ledger requires ascending order by the decoded 20-byte AccountID,
        // which is not the same as ascending base58.
        let decoded: Vec<_> = signers
            .iter()
            .map(|s| decode_classic_address(s.account.as_ref()).expect("decodes"))
            .collect();
        let mut expected = decoded.clone();
        expected.sort();
        assert_eq!(decoded, expected, "signers must be sorted by AccountID");
    }

    #[test]
    fn test_multisign_replaces_a_repeated_signer() {
        let alice = Wallet::new(SEED_A, 0).expect("wallet a");

        let mut tx = payment(&alice.classic_address);
        sign(&mut tx, &alice, true).expect("first");
        sign(&mut tx, &alice, true).expect("second");

        let signers = Transaction::get_common_fields(&tx)
            .signers
            .as_ref()
            .expect("signers");
        // rippled rejects two entries for one account; re-signing replaces.
        assert_eq!(signers.len(), 1);
    }

    #[test]
    fn test_multisign_forces_an_empty_signing_pub_key() {
        let alice = Wallet::new(SEED_A, 0).expect("wallet a");

        // Start from the deserialization shape, where `signing_pub_key` is None.
        let mut tx = payment(&alice.classic_address);
        Transaction::get_mut_common_fields(&mut tx).signing_pub_key = None;

        sign(&mut tx, &alice, true).expect("alice signs");

        // Absent and empty are different bytes in the STObject (`7300` versus
        // nothing), so two signers must agree on which one they signed over.
        assert_eq!(
            Transaction::get_common_fields(&tx)
                .signing_pub_key
                .as_deref(),
            Some(""),
            "multisigning must normalize SigningPubKey to the empty string"
        );
    }

    #[test]
    fn test_sort_signers_rejects_a_malformed_account() {
        let mut signers = vec![Signer::new(
            "not-an-address".to_string(),
            "DEADBEEF".to_string(),
            "ED00".to_string(),
        )];

        // This used to `.unwrap()`, aborting the process on caller-supplied input.
        assert!(sort_signers(&mut signers).is_err());
    }

    #[test]
    fn test_encode_for_multisigning_rejects_a_malformed_account() {
        let alice = Wallet::new(SEED_A, 0).expect("wallet a");
        let tx = payment(&alice.classic_address);

        assert!(encode_for_multisigning(&tx, "not-an-address".into()).is_err());
    }
}
