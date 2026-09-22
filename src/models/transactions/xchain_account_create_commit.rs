use core::fmt::Debug;

use alloc::{borrow::Cow, vec::Vec};
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;

use crate::models::{
    Amount, FlagCollection, Model, NoFlags, ValidateCurrencies, XChainBridge, XRPAmount,
};

use super::{CommonFields, Memo, Signer, Transaction, TransactionType};

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, xrpl_rust_macros::ValidateCurrencies)]
#[serde(rename_all = "PascalCase")]
pub struct XChainAccountCreateCommit<'a> {
    #[serde(flatten)]
    pub common_fields: CommonFields<'a, NoFlags>,
    pub amount: Amount<'a>,
    pub destination: Cow<'a, str>,
    #[serde(rename = "XChainBridge")]
    pub xchain_bridge: XChainBridge<'a>,
    pub signature_reward: Option<Amount<'a>>,
}

impl Model for XChainAccountCreateCommit<'_> {
    fn get_errors(&self) -> crate::models::XRPLModelResult<()> {
        self.validate_currencies()?;

        Ok(())
    }
}

impl<'a> Transaction<'a, NoFlags> for XChainAccountCreateCommit<'a> {
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
impl<'a> XChainAccountCreateCommit<'a> {
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
        #[builder(into)] amount: Amount<'a>,
        #[builder(into)] destination: Cow<'a, str>,
        #[builder(into)] xchain_bridge: XChainBridge<'a>,
        #[builder(into)] signature_reward: Option<Amount<'a>>,
    ) -> XChainAccountCreateCommit<'a> {
        XChainAccountCreateCommit {
            common_fields: CommonFields::builder(account)
                .transaction_type(TransactionType::XChainAccountCreateCommit)
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
            amount,
            destination,
            xchain_bridge,
            signature_reward,
        }
    }
}

#[cfg(test)]
mod test {
    use super::XChainAccountCreateCommit;
    use crate::models::{IssuedCurrency, XChainBridge, XRPAmount, XRP};
    use alloc::borrow::Cow;

    use super::*;

    const ACCOUNT: &str = "r9LqNeG6qHxjeUocjvVki2XR35weJ9mZgQ";
    const ACCOUNT2: &str = "rpZc4mVfWUif9CRoHRKKcmhu1nx2xktxBo";
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
    fn test_deserialize() {
        let json = r#"{
                "Account": "rwEqJ2UaQHe7jihxGqmx6J4xdbGiiyMaGa",
                "Destination": "rD323VyRjgzzhY4bFpo44rmyh2neB5d8Mo",
                "TransactionType": "XChainAccountCreateCommit",
                "Amount": "20000000",
                "SignatureReward": "100",
                "XChainBridge": {
                    "LockingChainDoor": "rMAXACCrp3Y8PpswXcg3bKggHX76V3F8M4",
                    "LockingChainIssue": {
                        "currency": "XRP"
                    },
                    "IssuingChainDoor": "rHb9CJAWyB4rj91VRWn96DkukG4bwdtyTh",
                    "IssuingChainIssue": {
                        "currency": "XRP"
                    }
                }
            }"#;
        let txn: XChainAccountCreateCommit<'_> = serde_json::from_str(json).unwrap();
        assert_eq!(txn.amount, "20000000".into());
    }

    #[test]
    fn test_successful() {
        let txn = XChainAccountCreateCommit::builder(Cow::Borrowed(ACCOUNT))
            .amount(XRPAmount::from("1000000"))
            .destination(Cow::Borrowed(ACCOUNT2))
            .xchain_bridge(xrp_bridge())
            .signature_reward(XRPAmount::from("200"))
            .build();
        assert_eq!(txn.amount, XRPAmount::from("1000000").into());
        assert_eq!(txn.signature_reward, Some(XRPAmount::from("200").into()));
    }

    #[test]
    #[should_panic]
    fn test_bad_signature_reward() {
        // Simulate a bad signature_reward by using a non-numeric string if your Amount type panics or errors on parse
        let tx = XChainAccountCreateCommit::builder(Cow::Borrowed(ACCOUNT))
            .amount(XRPAmount::from("1000000"))
            .destination(Cow::Borrowed(ACCOUNT2))
            .xchain_bridge(xrp_bridge())
            .signature_reward(XRPAmount::from("hello"))
            .build();

        tx.validate().unwrap();
    }

    #[test]
    #[should_panic]
    fn test_bad_amount() {
        // Simulate a bad amount by using a non-numeric string if your Amount type panics or errors on parse
        let tx = XChainAccountCreateCommit::builder(Cow::Borrowed(ACCOUNT))
            .amount(XRPAmount::from("hello"))
            .destination(Cow::Borrowed(ACCOUNT2))
            .xchain_bridge(xrp_bridge())
            .signature_reward(XRPAmount::from("200"))
            .build();

        tx.validate().unwrap();
    }
}
