use alloc::borrow::Cow;
use alloc::vec::Vec;
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;

use crate::models::amount::XRPAmount;
use crate::models::{
    transactions::{Memo, Signer, Transaction, TransactionType},
    Model, ValidateCurrencies, XRPLModelException,
};
use crate::models::{FlagCollection, NoFlags};

use crate::core::addresscodec::decode_classic_address;

use super::confidential_mpt_constants::{
    address_is_issuer, validate_hex_length, CIPHERTEXT_LENGTH, COMMITMENT_LENGTH, SEND_PROOF_LENGTH,
};
use super::mptoken_issuance_set::validate_mptoken_issuance_id;
use super::{validate_credential_ids, CommonFields, CommonTransactionBuilder};

/// A `ConfidentialMPTSend` transaction transfers a confidential MPT amount
/// from sender to destination, hiding the amount under EC-ElGamal
/// encryption (XLS-0096 §8). The amount is decrypted only by the recipient
/// (and the issuer / optional auditor via their mirror keys).
///
/// The 946-byte `ZKProof` field carries:
///   - 192 B compact AND-composed sigma proof (ciphertext consistency,
///     Pedersen amount linkage, balance ownership)
///   - 754 B aggregated Bulletproof (range proof on amount AND remainder)
///
/// `CredentialIDs` (XLS-70) are honored when the destination requires
/// pre-authorization.
#[skip_serializing_none]
#[derive(
    Debug,
    Default,
    Serialize,
    Deserialize,
    PartialEq,
    Eq,
    Clone,
    xrpl_rust_macros::ValidateCurrencies,
)]
#[serde(rename_all = "PascalCase")]
pub struct ConfidentialMPTSend<'a> {
    #[serde(flatten)]
    pub common_fields: CommonFields<'a, NoFlags>,

    /// Destination XRPL account.
    pub destination: Cow<'a, str>,

    /// Arbitrary tag that identifies the reason for the transfer, or a hosted
    /// recipient at the destination account.
    pub destination_tag: Option<u32>,

    #[serde(rename = "MPTokenIssuanceID")]
    pub mptoken_issuance_id: Cow<'a, str>,

    /// 66-byte ElGamal ciphertext debited from the sender's `CB_S`.
    pub sender_encrypted_amount: Cow<'a, str>,

    /// 66-byte ElGamal ciphertext credited to the receiver's `CB_IN`.
    pub destination_encrypted_amount: Cow<'a, str>,

    /// 66-byte ElGamal ciphertext used to update both the sender's and
    /// receiver's `IssuerEncryptedBalance` mirrors.
    pub issuer_encrypted_amount: Cow<'a, str>,

    /// 33-byte Pedersen commitment to the transfer amount.
    pub amount_commitment: Cow<'a, str>,

    /// 33-byte Pedersen commitment to the sender's confidential balance.
    pub balance_commitment: Cow<'a, str>,

    /// 946-byte composite ZK proof (192 B compact sigma + 754 B aggregated
    /// Bulletproof).
    #[serde(rename = "ZKProof")]
    pub zk_proof: Cow<'a, str>,

    /// 66-byte ciphertext for the auditor mirror. Required iff the
    /// issuance has an `AuditorEncryptionKey` registered.
    pub auditor_encrypted_amount: Option<Cow<'a, str>>,

    /// XLS-70 credentials presented to satisfy the destination's
    /// `DepositPreauth` / `AuthorizeCredentials` requirement, if any.
    #[serde(rename = "CredentialIDs")]
    pub credential_ids: Option<Vec<Cow<'a, str>>>,
}

impl<'a> Model for ConfidentialMPTSend<'a> {
    fn get_errors(&self) -> crate::models::XRPLModelResult<()> {
        self._get_destination_error()?;
        self._get_field_length_errors()?;
        self._get_issuer_role_error()?;
        validate_credential_ids(&self.credential_ids)?;
        self.validate_currencies()
    }
}

impl<'a> ConfidentialMPTSend<'a> {
    /// rippled rejects a malformed destination or a self-send (`temMALFORMED`).
    fn _get_destination_error(&self) -> crate::models::XRPLModelResult<()> {
        if decode_classic_address(self.destination.as_ref()).is_err() {
            return Err(XRPLModelException::InvalidValueFormat {
                field: "destination".into(),
                format: "classic XRPL address".into(),
                found: self.destination.as_ref().into(),
            });
        }
        if self.destination == self.common_fields.account {
            return Err(XRPLModelException::ValueEqualsValue {
                field1: "destination".into(),
                field2: "account".into(),
            });
        }
        Ok(())
    }

    /// rippled bans the issuer as either party of a confidential send: a
    /// `ConfidentialMPTSend` only moves value holder↔holder, so `Account` and
    /// `Destination` must both differ from the issuance's issuer (`temMALFORMED`,
    /// `ConfidentialMPTSend.cpp` preflight).
    fn _get_issuer_role_error(&self) -> crate::models::XRPLModelResult<()> {
        let issuance_id = self.mptoken_issuance_id.as_ref();
        if address_is_issuer(issuance_id, self.common_fields.account.as_ref()) {
            return Err(XRPLModelException::ValueEqualsValue {
                field1: "account".into(),
                field2: "issuer".into(),
            });
        }
        if address_is_issuer(issuance_id, self.destination.as_ref()) {
            return Err(XRPLModelException::ValueEqualsValue {
                field1: "destination".into(),
                field2: "issuer".into(),
            });
        }
        Ok(())
    }

    /// Ciphertext, commitment and proof lengths (`temBAD_CIPHERTEXT` /
    /// `temMALFORMED` in rippled's preflight).
    fn _get_field_length_errors(&self) -> crate::models::XRPLModelResult<()> {
        validate_mptoken_issuance_id(self.mptoken_issuance_id.as_ref())?;
        validate_hex_length(
            "sender_encrypted_amount",
            self.sender_encrypted_amount.as_ref(),
            CIPHERTEXT_LENGTH,
        )?;
        validate_hex_length(
            "destination_encrypted_amount",
            self.destination_encrypted_amount.as_ref(),
            CIPHERTEXT_LENGTH,
        )?;
        validate_hex_length(
            "issuer_encrypted_amount",
            self.issuer_encrypted_amount.as_ref(),
            CIPHERTEXT_LENGTH,
        )?;
        if let Some(auditor) = self.auditor_encrypted_amount.as_deref() {
            validate_hex_length("auditor_encrypted_amount", auditor, CIPHERTEXT_LENGTH)?;
        }
        validate_hex_length(
            "amount_commitment",
            self.amount_commitment.as_ref(),
            COMMITMENT_LENGTH,
        )?;
        validate_hex_length(
            "balance_commitment",
            self.balance_commitment.as_ref(),
            COMMITMENT_LENGTH,
        )?;
        validate_hex_length("zk_proof", self.zk_proof.as_ref(), SEND_PROOF_LENGTH)
    }
}

impl<'a> Transaction<'a, NoFlags> for ConfidentialMPTSend<'a> {
    fn get_transaction_type(&self) -> &TransactionType {
        self.common_fields.get_transaction_type()
    }

    fn get_common_fields(&self) -> &CommonFields<'_, NoFlags> {
        self.common_fields.get_common_fields()
    }

    fn get_mut_common_fields(&mut self) -> &mut CommonFields<'a, NoFlags> {
        self.common_fields.get_mut_common_fields()
    }
}

impl<'a> CommonTransactionBuilder<'a, NoFlags> for ConfidentialMPTSend<'a> {
    fn get_mut_common_fields(&mut self) -> &mut CommonFields<'a, NoFlags> {
        &mut self.common_fields
    }

    fn into_self(self) -> Self {
        self
    }
}

#[bon::bon]
impl<'a> ConfidentialMPTSend<'a> {
    #[allow(clippy::too_many_arguments)]
    #[builder]
    pub fn new(
        #[builder(start_fn, into)] account: Cow<'a, str>,
        #[builder(into)] account_txn_id: Option<Cow<'a, str>>,
        #[builder(into)] fee: Option<XRPAmount<'a>>,
        last_ledger_sequence: Option<u32>,
        #[builder(into)] memos: Option<Vec<Memo>>,
        sequence: Option<u32>,
        #[builder(into)] signers: Option<Vec<Signer>>,
        source_tag: Option<u32>,
        ticket_sequence: Option<u32>,
        #[builder(into)] destination: Cow<'a, str>,
        destination_tag: Option<u32>,
        #[builder(into)] mptoken_issuance_id: Cow<'a, str>,
        #[builder(into)] sender_encrypted_amount: Cow<'a, str>,
        #[builder(into)] destination_encrypted_amount: Cow<'a, str>,
        #[builder(into)] issuer_encrypted_amount: Cow<'a, str>,
        #[builder(into)] amount_commitment: Cow<'a, str>,
        #[builder(into)] balance_commitment: Cow<'a, str>,
        #[builder(into)] zk_proof: Cow<'a, str>,
        #[builder(into)] auditor_encrypted_amount: Option<Cow<'a, str>>,
        #[builder(into)] credential_ids: Option<Vec<Cow<'a, str>>>,
    ) -> Self {
        Self {
            common_fields: CommonFields::builder(account)
                .transaction_type(TransactionType::ConfidentialMPTSend)
                .maybe_account_txn_id(account_txn_id)
                .maybe_fee(fee)
                .flags(FlagCollection::default())
                .maybe_last_ledger_sequence(last_ledger_sequence)
                .maybe_memos(memos)
                .maybe_sequence(sequence)
                .maybe_signers(signers)
                .maybe_source_tag(source_tag)
                .maybe_ticket_sequence(ticket_sequence)
                .build(),
            destination,
            destination_tag,
            mptoken_issuance_id,
            sender_encrypted_amount,
            destination_encrypted_amount,
            issuer_encrypted_amount,
            amount_commitment,
            balance_commitment,
            zk_proof,
            auditor_encrypted_amount,
            credential_ids,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serialize() {
        let tx = ConfidentialMPTSend {
            common_fields: CommonFields {
                account: "rSenderAccount11111111111111111".into(),
                transaction_type: TransactionType::ConfidentialMPTSend,
                ..Default::default()
            },
            destination: "rRecipientAccount111111111111".into(),
            destination_tag: None,
            mptoken_issuance_id: "610F33".repeat(8).into(),
            sender_encrypted_amount: "AD".repeat(66).into(),
            destination_encrypted_amount: "DF".repeat(66).into(),
            issuer_encrypted_amount: "BC".repeat(66).into(),
            amount_commitment: "04".repeat(33).into(),
            balance_commitment: "03".repeat(33).into(),
            zk_proof: "84".repeat(946).into(),
            auditor_encrypted_amount: None,
            credential_ids: None,
        };

        let json = serde_json::to_string(&tx).unwrap();
        assert!(json.contains("\"TransactionType\":\"ConfidentialMPTSend\""));
        assert!(json.contains("\"Destination\":\"rRecipientAccount"));
        assert!(json.contains("\"AmountCommitment\""));
        assert!(json.contains("\"BalanceCommitment\""));
        assert!(json.contains("\"ZKProof\""));

        let round_tripped: ConfidentialMPTSend = serde_json::from_str(&json).unwrap();
        assert_eq!(round_tripped, tx);
    }

    #[test]
    fn test_new_builder_and_accessors() {
        let mut tx = ConfidentialMPTSend::builder("rHb9CJAWyB4rj91VRWn96DkukG4bwdtyTh")
            .destination("rLSn6Z3T8uCxbcd1oxwfGQN1Fdn5CyGujK")
            .mptoken_issuance_id("610F33".repeat(8))
            .sender_encrypted_amount("AD".repeat(66))
            .destination_encrypted_amount("DF".repeat(66))
            .issuer_encrypted_amount("BC".repeat(66))
            .amount_commitment("04".repeat(33))
            .balance_commitment("03".repeat(33))
            .zk_proof("84".repeat(946))
            .build()
            .with_fee("15000")
            .with_sequence(9);

        assert_eq!(tx.get_common_fields().sequence, Some(9));
        assert_eq!(tx.get_common_fields().fee, Some(XRPAmount::from("15000")));
        assert_eq!(
            tx.get_transaction_type(),
            &TransactionType::ConfidentialMPTSend
        );
        assert!(tx.get_errors().is_ok());

        let common =
            <ConfidentialMPTSend as Transaction<'_, NoFlags>>::get_mut_common_fields(&mut tx);
        assert_eq!(common.sequence, Some(9));
    }

    // Two valid, distinct classic addresses with known AccountIDs:
    // ACCT -> B5F762..37E8, DEST -> D528B6..705F.
    const ACCT: &str = "rHb9CJAWyB4rj91VRWn96DkukG4bwdtyTh";
    const DEST: &str = "rLSn6Z3T8uCxbcd1oxwfGQN1Fdn5CyGujK";
    // Issuance IDs whose issuer AccountID (bytes 4..24) is ACCT / DEST.
    const ISS_OF_ACCT: &str = "00000001B5F762798A53D543A014CAF8B297CFF8F2F937E8";
    const ISS_OF_DEST: &str = "00000001D528B62DC7AF16417C9F44AAD8C04D920A3A705F";

    fn valid_send() -> ConfidentialMPTSend<'static> {
        ConfidentialMPTSend {
            common_fields: CommonFields {
                account: ACCT.into(),
                transaction_type: TransactionType::ConfidentialMPTSend,
                ..Default::default()
            },
            destination: DEST.into(),
            destination_tag: None,
            // Arbitrary issuance whose issuer is neither ACCT nor DEST.
            mptoken_issuance_id: "610F33".repeat(8).into(),
            sender_encrypted_amount: "AD".repeat(66).into(),
            destination_encrypted_amount: "DF".repeat(66).into(),
            issuer_encrypted_amount: "BC".repeat(66).into(),
            amount_commitment: "04".repeat(33).into(),
            balance_commitment: "03".repeat(33).into(),
            zk_proof: "84".repeat(946).into(),
            auditor_encrypted_amount: None,
            credential_ids: None,
        }
    }

    #[test]
    fn test_valid_send_passes() {
        assert!(valid_send().get_errors().is_ok());
    }

    #[test]
    fn test_self_send_rejected() {
        let mut tx = valid_send();
        tx.destination = ACCT.into();
        assert!(tx.get_errors().is_err());
    }

    #[test]
    fn test_malformed_destination_rejected() {
        let mut tx = valid_send();
        tx.destination = "not_a_classic_address".into();
        assert!(tx.get_errors().is_err());
    }

    #[test]
    fn test_account_is_issuer_rejected() {
        let mut tx = valid_send();
        tx.mptoken_issuance_id = ISS_OF_ACCT.into();
        assert!(tx.get_errors().is_err());
    }

    #[test]
    fn test_destination_is_issuer_rejected() {
        let mut tx = valid_send();
        tx.mptoken_issuance_id = ISS_OF_DEST.into();
        assert!(tx.get_errors().is_err());
    }

    #[test]
    fn test_wrong_length_ciphertext_rejected() {
        let mut tx = valid_send();
        tx.sender_encrypted_amount = "AD".repeat(10).into();
        assert!(tx.get_errors().is_err());
    }
}
