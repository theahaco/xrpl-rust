use alloc::borrow::Cow;
use alloc::vec::Vec;

use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};
use serde_with::skip_serializing_none;
use strum_macros::{AsRefStr, Display, EnumIter};

use crate::models::transactions::{CommonFields, Memo, Signer};
use crate::models::{
    amount::XRPAmount,
    transactions::{Transaction, TransactionType},
    Model,
};
use crate::models::{FlagCollection, NoFlags};

#[derive(
    Debug, Eq, PartialEq, Clone, Serialize_repr, Deserialize_repr, Display, AsRefStr, EnumIter,
)]
#[repr(u32)]
pub enum UNLModifyDisabling {
    Disable = 0,
    Enable = 1,
}

/// See UNLModify:
/// `<https://xrpl.org/unlmodify.html>`
#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct UNLModify<'a> {
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
    /// The custom fields for the UNLModify model.
    ///
    /// See UNLModify fields:
    /// `<https://xrpl.org/unlmodify.html#unlmodify-fields>`
    pub ledger_sequence: u32,
    pub unlmodify_disabling: UNLModifyDisabling,
    pub unlmodify_validator: Cow<'a, str>,
}

impl<'a> Model for UNLModify<'a> {}

impl<'a> Transaction<'a, NoFlags> for UNLModify<'a> {
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
impl<'a> UNLModify<'a> {
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
        ledger_sequence: u32,
        #[builder(into)] unlmodify_disabling: UNLModifyDisabling,
        #[builder(into)] unlmodify_validator: Cow<'a, str>,
    ) -> Self {
        Self {
            common_fields: CommonFields::builder(account)
                .transaction_type(TransactionType::UNLModify)
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
            ledger_sequence,
            unlmodify_disabling,
            unlmodify_validator,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serde_round_trip() {
        let txn = UNLModify::builder("rrrrrrrrrrrrrrrrrrrrrhoLvTp")
            .ledger_sequence(56865245)
            .unlmodify_disabling(UNLModifyDisabling::Enable)
            .unlmodify_validator(
                "ED1234567890ABCDEF1234567890ABCDEF1234567890ABCDEF1234567890ABCDEF",
            )
            .build();
        let serialized = serde_json::to_string(&txn).unwrap();
        let deserialized: UNLModify = serde_json::from_str(&serialized).unwrap();
        assert_eq!(txn, deserialized);
        assert!(serialized.contains("\"TransactionType\":\"UNLModify\""));
        assert!(serialized.contains("\"UnlmodifyDisabling\":1"));
        assert_eq!(txn.get_transaction_type(), &TransactionType::UNLModify);
    }

    #[test]
    fn test_disabling_disable() {
        let txn = UNLModify::builder("rrrrrrrrrrrrrrrrrrrrrhoLvTp")
            .ledger_sequence(1)
            .unlmodify_disabling(UNLModifyDisabling::Disable)
            .unlmodify_validator("ED00")
            .build();
        let serialized = serde_json::to_string(&txn).unwrap();
        assert!(serialized.contains("\"UnlmodifyDisabling\":0"));
    }
}
