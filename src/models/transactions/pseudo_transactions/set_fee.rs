use alloc::borrow::Cow;
use alloc::vec::Vec;

use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;

use crate::models::amount::XRPAmount;
use crate::models::transactions::{CommonFields, Memo, Signer};
use crate::models::{
    transactions::{Transaction, TransactionType},
    Model,
};
use crate::models::{FlagCollection, NoFlags};

/// See SetFee:
/// `<https://xrpl.org/setfee.html>`
#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct SetFee<'a> {
    // The base fields for all transaction models.
    //
    // See Transaction Types:
    // `<https://xrpl.org/transaction-types.html>`
    //
    // See Transaction Common Fields:
    // `<https://xrpl.org/transaction-common-fields.html>`
    /// The type of transaction.
    #[serde(flatten)]
    pub common_fields: CommonFields<'a, NoFlags>,
    /// The custom fields for the SetFee model.
    ///
    /// See SetFee fields:
    /// `<https://xrpl.org/setfee.html#setfee-fields>`
    pub base_fee: XRPAmount<'a>,
    pub reference_fee_units: u32,
    pub reserve_base: u32,
    pub reserve_increment: u32,
    pub ledger_sequence: u32,
}

impl<'a> Model for SetFee<'a> {}

impl<'a> Transaction<'a, NoFlags> for SetFee<'a> {
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

#[bon::bon]
impl<'a> SetFee<'a> {
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
        #[builder(into)] base_fee: XRPAmount<'a>,
        reference_fee_units: u32,
        reserve_base: u32,
        reserve_increment: u32,
        ledger_sequence: u32,
    ) -> Self {
        Self {
            common_fields: CommonFields::builder(account)
                .transaction_type(TransactionType::SetFee)
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
            base_fee,
            reference_fee_units,
            reserve_base,
            reserve_increment,
            ledger_sequence,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serde_round_trip() {
        let txn = SetFee::builder("rrrrrrrrrrrrrrrrrrrrrhoLvTp")
            .base_fee("10")
            .reference_fee_units(10)
            .reserve_base(20_000_000)
            .reserve_increment(5_000_000)
            .ledger_sequence(56865245)
            .build();
        let serialized = serde_json::to_string(&txn).unwrap();
        let deserialized: SetFee = serde_json::from_str(&serialized).unwrap();
        assert_eq!(txn, deserialized);
        assert!(serialized.contains("\"TransactionType\":\"SetFee\""));
        assert!(serialized.contains("\"BaseFee\":\"10\""));
        assert!(serialized.contains("\"ReferenceFeeUnits\":10"));
        assert!(serialized.contains("\"ReserveBase\":20000000"));
        assert_eq!(txn.get_transaction_type(), &TransactionType::SetFee);
    }
}
