use alloc::borrow::Cow;
use alloc::vec::Vec;

use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;

use crate::models::amount::XRPAmount;
use crate::models::transactions::CommonFields;
use crate::models::{
    transactions::{Transaction, TransactionType},
    Model,
};
use crate::models::{FlagCollection, NoFlags, ValidateCurrencies};

use super::{CommonTransactionBuilder, Memo, Signer};

/// Cancels an Escrow and returns escrowed XRP to the sender.
///
/// See EscrowCancel:
/// `<https://xrpl.org/docs/references/protocol/transactions/types/escrowcancel>`
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
pub struct EscrowCancel<'a> {
    /// The base fields for all transaction models.
    ///
    /// See Transaction Common Fields:
    /// `<https://xrpl.org/transaction-common-fields.html>`
    #[serde(flatten)]
    pub common_fields: CommonFields<'a, NoFlags>,
    /// Address of the source account that funded the escrow payment.
    pub owner: Cow<'a, str>,
    /// Transaction sequence (or Ticket number) of EscrowCreate transaction that created the escrow to cancel.
    pub offer_sequence: u32,
}

impl<'a> Model for EscrowCancel<'a> {
    fn get_errors(&self) -> crate::models::XRPLModelResult<()> {
        self.validate_currencies()
    }
}

impl<'a> Transaction<'a, NoFlags> for EscrowCancel<'a> {
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

impl<'a> CommonTransactionBuilder<'a, NoFlags> for EscrowCancel<'a> {
    fn get_mut_common_fields(&mut self) -> &mut CommonFields<'a, NoFlags> {
        &mut self.common_fields
    }

    fn into_self(self) -> Self {
        self
    }
}

#[bon::bon]
impl<'a> EscrowCancel<'a> {
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
        #[builder(into)] owner: Cow<'a, str>,
        offer_sequence: u32,
    ) -> Self {
        Self {
            common_fields: CommonFields::builder(account)
                .transaction_type(TransactionType::EscrowCancel)
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
            owner,
            offer_sequence,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serde() {
        let default_txn = EscrowCancel {
            common_fields: CommonFields {
                account: "rf1BiGeXwwQoi8Z2ueFYTEXSwuJYfV2Jpn".into(),
                transaction_type: TransactionType::EscrowCancel,
                signing_pub_key: Some("".into()),
                ..Default::default()
            },
            owner: "rf1BiGeXwwQoi8Z2ueFYTEXSwuJYfV2Jpn".into(),
            offer_sequence: 7,
        };

        let default_json_str = r#"{"Account":"rf1BiGeXwwQoi8Z2ueFYTEXSwuJYfV2Jpn","TransactionType":"EscrowCancel","Flags":0,"SigningPubKey":"","Owner":"rf1BiGeXwwQoi8Z2ueFYTEXSwuJYfV2Jpn","OfferSequence":7}"#;

        let default_json_value = serde_json::to_value(default_json_str).unwrap();
        let serialized_string = serde_json::to_string(&default_txn).unwrap();
        let serialized_value = serde_json::to_value(&serialized_string).unwrap();
        assert_eq!(serialized_value, default_json_value);

        let deserialized: EscrowCancel = serde_json::from_str(default_json_str).unwrap();
        assert_eq!(default_txn, deserialized);
    }

    #[test]
    fn test_builder_pattern() {
        let escrow_cancel = EscrowCancel {
            common_fields: CommonFields {
                account: "rf1BiGeXwwQoi8Z2ueFYTEXSwuJYfV2Jpn".into(),
                transaction_type: TransactionType::EscrowCancel,
                ..Default::default()
            },
            owner: "rf1BiGeXwwQoi8Z2ueFYTEXSwuJYfV2Jpn".into(),
            offer_sequence: 7,
        }
        .with_fee("12")
        .with_sequence(123)
        .with_last_ledger_sequence(7108682)
        .with_source_tag(12345);

        assert_eq!(escrow_cancel.owner, "rf1BiGeXwwQoi8Z2ueFYTEXSwuJYfV2Jpn");
        assert_eq!(escrow_cancel.offer_sequence, 7);
        assert_eq!(escrow_cancel.common_fields.fee.as_ref().unwrap().0, "12");
        assert_eq!(escrow_cancel.common_fields.sequence, Some(123));
        assert_eq!(
            escrow_cancel.common_fields.last_ledger_sequence,
            Some(7108682)
        );
        assert_eq!(escrow_cancel.common_fields.source_tag, Some(12345));
    }

    #[test]
    fn test_default() {
        let escrow_cancel = EscrowCancel {
            common_fields: CommonFields {
                account: "rf1BiGeXwwQoi8Z2ueFYTEXSwuJYfV2Jpn".into(),
                transaction_type: TransactionType::EscrowCancel,
                ..Default::default()
            },
            owner: "rf1BiGeXwwQoi8Z2ueFYTEXSwuJYfV2Jpn".into(),
            offer_sequence: 7,
        };

        assert_eq!(
            escrow_cancel.common_fields.account,
            "rf1BiGeXwwQoi8Z2ueFYTEXSwuJYfV2Jpn"
        );
        assert_eq!(
            escrow_cancel.common_fields.transaction_type,
            TransactionType::EscrowCancel
        );
        assert_eq!(escrow_cancel.owner, "rf1BiGeXwwQoi8Z2ueFYTEXSwuJYfV2Jpn");
        assert_eq!(escrow_cancel.offer_sequence, 7);
    }

    #[test]
    fn test_new_constructor_and_trait_impls() {
        let txn = EscrowCancel::builder("rf1BiGeXwwQoi8Z2ueFYTEXSwuJYfV2Jpn")
            .fee("12")
            .last_ledger_sequence(8_000_000)
            .sequence(456)
            .owner("rOwner")
            .offer_sequence(7)
            .build();
        assert_eq!(txn.get_transaction_type(), &TransactionType::EscrowCancel);
        assert_eq!(txn.get_common_fields().sequence, Some(456));
        assert!(txn.get_errors().is_ok());
        let mut t = txn;
        t.common_fields.source_tag = Some(11);
        assert_eq!(t.common_fields.source_tag, Some(11));
    }
}
