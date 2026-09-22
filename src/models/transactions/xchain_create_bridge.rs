use alloc::{borrow::Cow, vec::Vec};
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;

use crate::models::{
    transactions::exceptions::XRPLXChainCreateBridgeException, Amount, FlagCollection, Model,
    NoFlags, ValidateCurrencies, XChainBridge, XRPAmount, XRPLModelResult, XRP,
};

use super::{CommonFields, Memo, Signer, Transaction, TransactionType};

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, xrpl_rust_macros::ValidateCurrencies)]
#[serde(rename_all = "PascalCase")]
pub struct XChainCreateBridge<'a> {
    #[serde(flatten)]
    pub common_fields: CommonFields<'a, NoFlags>,
    pub signature_reward: Amount<'a>,
    #[serde(rename = "XChainBridge")]
    pub xchain_bridge: XChainBridge<'a>,
    pub min_account_create_amount: Option<XRPAmount<'a>>,
}

impl Model for XChainCreateBridge<'_> {
    fn get_errors(&self) -> XRPLModelResult<()> {
        self.validate_currencies()?;
        self.get_same_door_error()?;
        self.get_account_door_mismatch_error()?;
        self.get_cross_currency_bridge_not_allowed_error()?;
        self.get_min_account_create_amount_for_iou_error()
    }
}

impl<'a> Transaction<'a, NoFlags> for XChainCreateBridge<'a> {
    fn get_transaction_type(&self) -> &super::TransactionType {
        self.common_fields.get_transaction_type()
    }

    fn get_common_fields(&self) -> &CommonFields<'_, NoFlags> {
        &self.common_fields
    }

    fn get_mut_common_fields(&mut self) -> &mut CommonFields<'a, NoFlags> {
        &mut self.common_fields
    }
}

#[bon::bon]
impl<'a> XChainCreateBridge<'a> {
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
        #[builder(into)] signature_reward: Amount<'a>,
        #[builder(into)] xchain_bridge: XChainBridge<'a>,
        #[builder(into)] min_account_create_amount: Option<XRPAmount<'a>>,
    ) -> XChainCreateBridge<'a> {
        XChainCreateBridge {
            common_fields: CommonFields::builder(account)
                .transaction_type(TransactionType::XChainCreateBridge)
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
            signature_reward,
            xchain_bridge,
            min_account_create_amount,
        }
    }

    fn get_same_door_error(&self) -> XRPLModelResult<()> {
        let bridge = &self.xchain_bridge;
        if bridge.issuing_chain_door == bridge.locking_chain_door {
            Err(XRPLXChainCreateBridgeException::SameDoorAccounts.into())
        } else {
            Ok(())
        }
    }

    fn get_account_door_mismatch_error(&self) -> XRPLModelResult<()> {
        let bridge = &self.xchain_bridge;
        if ![&bridge.issuing_chain_door, &bridge.locking_chain_door]
            .contains(&&self.common_fields.account)
        {
            Err(XRPLXChainCreateBridgeException::AccountDoorMismatch.into())
        } else {
            Ok(())
        }
    }

    fn get_cross_currency_bridge_not_allowed_error(&self) -> XRPLModelResult<()> {
        let bridge = &self.xchain_bridge;
        if (bridge.locking_chain_issue == XRP::new().into())
            != (bridge.issuing_chain_issue == XRP::new().into())
        {
            Err(XRPLXChainCreateBridgeException::CrossCurrencyBridgeNotAllowed.into())
        } else {
            Ok(())
        }
    }

    fn get_min_account_create_amount_for_iou_error(&self) -> XRPLModelResult<()> {
        let bridge = &self.xchain_bridge;
        if self.min_account_create_amount.is_some()
            && bridge.locking_chain_issue != XRP::new().into()
        {
            Err(XRPLXChainCreateBridgeException::MinAccountCreateAmountForIOU.into())
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod test_xchain_create_bridge {
    use super::XChainCreateBridge;
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
    fn test_successful_xrp_xrp_bridge() {
        let bridge = xrp_bridge();
        let txn = XChainCreateBridge::builder(Cow::Borrowed(ACCOUNT))
            .fee(XRPAmount::from(FEE))
            .sequence(SEQUENCE)
            .signature_reward(XRPAmount::from("200"))
            .xchain_bridge(bridge)
            .min_account_create_amount(XRPAmount::from("1000000"))
            .build();
        assert!(txn.validate().is_ok());
    }

    #[test]
    fn test_successful_iou_iou_bridge() {
        let bridge = iou_bridge();
        let txn = XChainCreateBridge::builder(Cow::Borrowed(ACCOUNT))
            .fee(XRPAmount::from(FEE))
            .sequence(SEQUENCE)
            .signature_reward(XRPAmount::from("200"))
            .xchain_bridge(bridge)
            .build();
        assert!(txn.validate().is_ok());
    }

    #[test]
    #[should_panic]
    fn test_same_door_accounts() {
        let bridge = XChainBridge {
            locking_chain_door: Cow::Borrowed(ACCOUNT),
            locking_chain_issue: IssuedCurrency {
                currency: Cow::Borrowed("USD"),
                issuer: Cow::Borrowed(ISSUER),
            }
            .into(),
            issuing_chain_door: Cow::Borrowed(ACCOUNT),
            issuing_chain_issue: IssuedCurrency {
                currency: Cow::Borrowed("USD"),
                issuer: Cow::Borrowed(ACCOUNT),
            }
            .into(),
        };
        let txn = XChainCreateBridge::builder(Cow::Borrowed(ACCOUNT))
            .fee(XRPAmount::from(FEE))
            .sequence(SEQUENCE)
            .signature_reward(XRPAmount::from("200"))
            .xchain_bridge(bridge)
            .build();
        txn.validate().unwrap();
    }

    #[test]
    #[should_panic]
    fn test_xrp_iou_bridge() {
        let bridge = XChainBridge {
            locking_chain_door: Cow::Borrowed(ACCOUNT),
            locking_chain_issue: XRP::new().into(),
            issuing_chain_door: Cow::Borrowed(ACCOUNT),
            issuing_chain_issue: IssuedCurrency {
                currency: Cow::Borrowed("USD"),
                issuer: Cow::Borrowed(ACCOUNT),
            }
            .into(),
        };
        let txn = XChainCreateBridge::builder(Cow::Borrowed(ACCOUNT))
            .fee(XRPAmount::from(FEE))
            .sequence(SEQUENCE)
            .signature_reward(XRPAmount::from("200"))
            .xchain_bridge(bridge)
            .build();
        txn.validate().unwrap();
    }

    #[test]
    #[should_panic]
    fn test_iou_xrp_bridge() {
        let bridge = XChainBridge {
            locking_chain_door: Cow::Borrowed(ACCOUNT),
            locking_chain_issue: IssuedCurrency {
                currency: Cow::Borrowed("USD"),
                issuer: Cow::Borrowed(ISSUER),
            }
            .into(),
            issuing_chain_door: Cow::Borrowed(ACCOUNT),
            issuing_chain_issue: XRP::new().into(),
        };
        let txn = XChainCreateBridge::builder(Cow::Borrowed(ACCOUNT))
            .fee(XRPAmount::from(FEE))
            .sequence(SEQUENCE)
            .signature_reward(XRPAmount::from("200"))
            .xchain_bridge(bridge)
            .build();
        txn.validate().unwrap();
    }

    #[test]
    #[should_panic]
    fn test_account_not_in_bridge() {
        let bridge = XChainBridge {
            locking_chain_door: Cow::Borrowed(ACCOUNT),
            locking_chain_issue: XRP::new().into(),
            issuing_chain_door: Cow::Borrowed(ACCOUNT2),
            issuing_chain_issue: XRP::new().into(),
        };
        let txn = XChainCreateBridge::builder(Cow::Borrowed(GENESIS))
            .fee(XRPAmount::from(FEE))
            .sequence(SEQUENCE)
            .signature_reward(XRPAmount::from("200"))
            .xchain_bridge(bridge)
            .build();
        txn.validate().unwrap();
    }

    #[test]
    #[should_panic]
    fn test_iou_iou_min_account_create_amount() {
        let bridge = iou_bridge();
        let txn = XChainCreateBridge::builder(Cow::Borrowed(ACCOUNT))
            .fee(XRPAmount::from(FEE))
            .sequence(SEQUENCE)
            .signature_reward(XRPAmount::from("200"))
            .xchain_bridge(bridge)
            .min_account_create_amount(XRPAmount::from("1000000"))
            .build();
        txn.validate().unwrap();
    }

    #[test]
    #[should_panic]
    fn test_invalid_signature_reward() {
        let bridge = xrp_bridge();
        let txn = XChainCreateBridge::builder(Cow::Borrowed(ACCOUNT))
            .fee(XRPAmount::from(FEE))
            .sequence(SEQUENCE)
            .signature_reward(Amount::from("hello"))
            .xchain_bridge(bridge)
            .min_account_create_amount(XRPAmount::from("1000000"))
            .build();
        txn.validate().unwrap();
    }

    #[test]
    #[should_panic]
    fn test_invalid_min_account_create_amount() {
        let bridge = xrp_bridge();
        let txn = XChainCreateBridge::builder(Cow::Borrowed(ACCOUNT))
            .fee(XRPAmount::from(FEE))
            .sequence(SEQUENCE)
            .signature_reward(Amount::from("-200"))
            .xchain_bridge(bridge)
            .min_account_create_amount(XRPAmount::from("hello"))
            .build();
        txn.validate().unwrap();
    }
}
