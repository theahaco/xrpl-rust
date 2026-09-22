use alloc::{borrow::Cow, vec::Vec};
use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};
use serde_with::skip_serializing_none;
use strum_macros::{AsRefStr, Display, EnumIter};

use crate::models::{
    transactions::exceptions::XRPLXChainModifyBridgeException, Amount, FlagCollection, Model,
    ValidateCurrencies, XChainBridge, XRPAmount, XRPLModelResult, XRP,
};

use super::{CommonFields, Memo, Signer, Transaction, TransactionType};

#[derive(
    Debug, Eq, PartialEq, Clone, Serialize_repr, Deserialize_repr, Display, AsRefStr, EnumIter,
)]
#[repr(u32)]
pub enum XChainModifyBridgeFlags {
    /// Clears the MinAccountCreateAmount of the bridge.
    TfClearAccountCreateAmount = 0x00010000,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, xrpl_rust_macros::ValidateCurrencies)]
#[serde(rename_all = "PascalCase")]
pub struct XChainModifyBridge<'a> {
    #[serde(flatten)]
    pub common_fields: CommonFields<'a, XChainModifyBridgeFlags>,
    #[serde(rename = "XChainBridge")]
    pub xchain_bridge: XChainBridge<'a>,
    pub min_account_create_amount: Option<Amount<'a>>,
    pub signature_reward: Option<Amount<'a>>,
}

impl Model for XChainModifyBridge<'_> {
    fn get_errors(&self) -> XRPLModelResult<()> {
        self.validate_currencies()?;
        self.get_must_change_or_clear_error()?;
        self.get_account_door_mismatch_error()?;
        self.get_cannot_have_min_account_create_amount()?;
        Ok(())
    }
}

impl<'a> Transaction<'a, XChainModifyBridgeFlags> for XChainModifyBridge<'a> {
    fn get_common_fields(&self) -> &CommonFields<'_, XChainModifyBridgeFlags> {
        &self.common_fields
    }

    fn get_mut_common_fields(&mut self) -> &mut CommonFields<'a, XChainModifyBridgeFlags> {
        &mut self.common_fields
    }

    fn get_transaction_type(&self) -> &super::TransactionType {
        self.common_fields.get_transaction_type()
    }
}

#[bon::bon]
impl<'a> XChainModifyBridge<'a> {
    #[builder]
    pub fn new(
        #[builder(start_fn, into)] account: Cow<'a, str>,
        #[builder(into)] account_txn_id: Option<Cow<'a, str>>,
        #[builder(into)] fee: Option<XRPAmount<'a>>,
        #[builder(into)] flags: Option<FlagCollection<XChainModifyBridgeFlags>>,
        last_ledger_sequence: Option<u32>,
        #[builder(into)] memos: Option<Vec<Memo>>,
        sequence: Option<u32>,
        #[builder(into)] signers: Option<Vec<Signer>>,
        source_tag: Option<u32>,
        ticket_sequence: Option<u32>,
        #[builder(into)] xchain_bridge: XChainBridge<'a>,
        #[builder(into)] min_account_create_amount: Option<Amount<'a>>,
        #[builder(into)] signature_reward: Option<Amount<'a>>,
    ) -> XChainModifyBridge<'a> {
        XChainModifyBridge {
            common_fields: CommonFields::builder(account)
                .transaction_type(TransactionType::XChainModifyBridge)
                .maybe_account_txn_id(account_txn_id)
                .maybe_fee(fee)
                .flags(flags.unwrap_or_default())
                .maybe_last_ledger_sequence(last_ledger_sequence)
                .maybe_memos(memos)
                .maybe_sequence(sequence)
                .maybe_signers(signers)
                .maybe_source_tag(source_tag)
                .maybe_ticket_sequence(ticket_sequence)
                .build(),
            xchain_bridge,
            min_account_create_amount,
            signature_reward,
        }
    }

    fn get_must_change_or_clear_error(&self) -> XRPLModelResult<()> {
        if self.signature_reward.is_none()
            && self.min_account_create_amount.is_none()
            && !self.has_flag(&XChainModifyBridgeFlags::TfClearAccountCreateAmount)
        {
            Err(XRPLXChainModifyBridgeException::MustChangeOrClear.into())
        } else {
            Ok(())
        }
    }

    fn get_account_door_mismatch_error(&self) -> XRPLModelResult<()> {
        let bridge = &self.xchain_bridge;
        if ![&bridge.locking_chain_door, &bridge.issuing_chain_door]
            .contains(&&self.get_common_fields().account)
        {
            Err(XRPLXChainModifyBridgeException::AccountDoorMismatch.into())
        } else {
            Ok(())
        }
    }

    fn get_cannot_have_min_account_create_amount(&self) -> XRPLModelResult<()> {
        let bridge = &self.xchain_bridge;
        if self.min_account_create_amount.is_some()
            && bridge.locking_chain_issue != XRP::new().into()
        {
            Err(XRPLXChainModifyBridgeException::CannotHaveMinAccountCreateAmount.into())
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod test_xchain_modify_bridge {
    use super::XChainModifyBridge;
    use crate::models::{Amount, IssuedCurrency, Model, XChainBridge, XRPAmount, XRP};
    use alloc::borrow::Cow;

    const ACCOUNT: &str = "r9LqNeG6qHxjeUocjvVki2XR35weJ9mZgQ";
    const ACCOUNT2: &str = "rpZc4mVfWUif9CRoHRKKcmhu1nx2xktxBo";
    const FEE: &str = "0.00001";
    const SEQUENCE: u32 = 19048;
    const ISSUER: &str = "rGWrZyQqhTp9Xu7G5Pkayo7bXjH4k4QYpf";
    const GENESIS: &str = "rHb9CJAWyB4rj91VRWn96DkukG4bwdtyTh";

    fn xrp_bridge<'a>() -> XChainBridge<'a> {
        XChainBridge {
            locking_chain_door: Cow::Borrowed(ACCOUNT),
            locking_chain_issue: XRP::new().into(),
            issuing_chain_door: Cow::Borrowed(GENESIS),
            issuing_chain_issue: XRP::new().into(),
        }
    }

    fn iou_bridge<'a>() -> XChainBridge<'a> {
        XChainBridge {
            locking_chain_door: Cow::Borrowed(ACCOUNT),
            locking_chain_issue: IssuedCurrency {
                currency: Cow::Borrowed("USD"),
                issuer: Cow::Borrowed(ISSUER),
            }
            .into(),
            issuing_chain_door: Cow::Borrowed(ACCOUNT2),
            issuing_chain_issue: IssuedCurrency {
                currency: Cow::Borrowed("USD"),
                issuer: Cow::Borrowed(ACCOUNT2),
            }
            .into(),
        }
    }

    #[test]
    fn test_successful_modify_bridge() {
        let txn = XChainModifyBridge::builder(Cow::Borrowed(ACCOUNT))
            .fee(XRPAmount::from(FEE))
            .sequence(SEQUENCE)
            .xchain_bridge(xrp_bridge())
            .min_account_create_amount(XRPAmount::from("1000000"))
            .signature_reward(XRPAmount::from("200"))
            .build();
        assert!(txn.validate().is_ok());
    }

    #[test]
    fn test_successful_modify_bridge_only_signature_reward() {
        let txn = XChainModifyBridge::builder(Cow::Borrowed(ACCOUNT))
            .fee(XRPAmount::from(FEE))
            .sequence(SEQUENCE)
            .xchain_bridge(iou_bridge())
            .signature_reward(XRPAmount::from("200"))
            .build();
        assert!(txn.validate().is_ok());
    }

    #[test]
    fn test_successful_modify_bridge_only_min_account_create_amount() {
        let txn = XChainModifyBridge::builder(Cow::Borrowed(ACCOUNT))
            .fee(XRPAmount::from(FEE))
            .sequence(SEQUENCE)
            .xchain_bridge(xrp_bridge())
            .min_account_create_amount(XRPAmount::from("1000000"))
            .build();
        assert!(txn.validate().is_ok());
    }

    #[test]
    #[should_panic]
    fn test_modify_bridge_empty() {
        let txn = XChainModifyBridge::builder(Cow::Borrowed(ACCOUNT))
            .fee(XRPAmount::from(FEE))
            .sequence(SEQUENCE)
            .xchain_bridge(iou_bridge())
            .build();
        txn.validate().unwrap();
    }

    #[test]
    #[should_panic]
    fn test_account_not_in_bridge() {
        let txn = XChainModifyBridge::builder(Cow::Borrowed(ACCOUNT2))
            .fee(XRPAmount::from(FEE))
            .sequence(SEQUENCE)
            .xchain_bridge(xrp_bridge())
            .signature_reward(XRPAmount::from("200"))
            .build();
        txn.validate().unwrap();
    }

    #[test]
    #[should_panic]
    fn test_iou_iou_min_account_create_amount() {
        let txn = XChainModifyBridge::builder(Cow::Borrowed(ACCOUNT))
            .fee(XRPAmount::from(FEE))
            .sequence(SEQUENCE)
            .xchain_bridge(iou_bridge())
            .min_account_create_amount(XRPAmount::from("1000000"))
            .build();
        txn.validate().unwrap();
    }

    #[test]
    #[should_panic]
    fn test_invalid_signature_reward() {
        let txn = XChainModifyBridge::builder(Cow::Borrowed(ACCOUNT))
            .fee(XRPAmount::from(FEE))
            .sequence(SEQUENCE)
            .xchain_bridge(xrp_bridge())
            .min_account_create_amount(XRPAmount::from("1000000"))
            .signature_reward(Amount::from("hello"))
            .build();
        txn.validate().unwrap();
    }

    #[test]
    #[should_panic]
    fn test_invalid_min_account_create_amount() {
        let txn = XChainModifyBridge::builder(Cow::Borrowed(ACCOUNT))
            .fee(XRPAmount::from(FEE))
            .sequence(SEQUENCE)
            .xchain_bridge(xrp_bridge())
            .min_account_create_amount(Amount::from("hello"))
            .signature_reward(XRPAmount::from("200"))
            .build();
        txn.validate().unwrap();
    }
}
