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
pub struct XChainAddAccountCreateAttestation<'a> {
    #[serde(flatten)]
    pub common_fields: CommonFields<'a, NoFlags>,
    pub amount: Amount<'a>,
    pub attestation_reward_account: Cow<'a, str>,
    pub attestation_signer_account: Cow<'a, str>,
    pub destination: Cow<'a, str>,
    pub other_chain_source: Cow<'a, str>,
    pub public_key: Cow<'a, str>,
    pub signature: Cow<'a, str>,
    pub signature_reward: Amount<'a>,
    pub was_locking_chain_send: u8,
    #[serde(rename = "XChainAccountCreateCount")]
    pub xchain_account_create_count: Cow<'a, str>,
    #[serde(rename = "XChainBridge")]
    pub xchain_bridge: XChainBridge<'a>,
}

impl Model for XChainAddAccountCreateAttestation<'_> {
    fn get_errors(&self) -> crate::models::XRPLModelResult<()> {
        self.validate_currencies()
    }
}

impl<'a> Transaction<'a, NoFlags> for XChainAddAccountCreateAttestation<'a> {
    fn get_transaction_type(&self) -> &super::TransactionType {
        self.common_fields.get_transaction_type()
    }

    fn get_common_fields(&self) -> &super::CommonFields<'_, NoFlags> {
        &self.common_fields
    }

    fn get_mut_common_fields(&mut self) -> &mut super::CommonFields<'a, NoFlags> {
        &mut self.common_fields
    }
}

#[bon::bon]
impl<'a> XChainAddAccountCreateAttestation<'a> {
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
        #[builder(into)] attestation_reward_account: Cow<'a, str>,
        #[builder(into)] attestation_signer_account: Cow<'a, str>,
        #[builder(into)] destination: Cow<'a, str>,
        #[builder(into)] other_chain_source: Cow<'a, str>,
        #[builder(into)] public_key: Cow<'a, str>,
        #[builder(into)] signature: Cow<'a, str>,
        #[builder(into)] signature_reward: Amount<'a>,
        was_locking_chain_send: u8,
        #[builder(into)] xchain_account_create_count: Cow<'a, str>,
        #[builder(into)] xchain_bridge: XChainBridge<'a>,
    ) -> XChainAddAccountCreateAttestation<'a> {
        XChainAddAccountCreateAttestation {
            common_fields: CommonFields::builder(account)
                .transaction_type(TransactionType::XChainAddAccountCreateAttestation)
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
            attestation_reward_account,
            attestation_signer_account,
            destination,
            other_chain_source,
            public_key,
            signature,
            signature_reward,
            was_locking_chain_send,
            xchain_account_create_count,
            xchain_bridge,
        }
    }
}

#[cfg(test)]
mod test_serde {
    const EXAMPLE_JSON: &str = r#"{
        "Account": "rDr5okqGKmMpn44Bbhe5WAfDQx8e9XquEv",
        "Flags": 0,
        "TransactionType": "XChainAddAccountCreateAttestation",
        "OtherChainSource": "rUzB7yg1LcFa7m3q1hfrjr5w53vcWzNh3U",
        "Destination": "rJMfWNVbyjcCtds8kpoEjEbYQ41J5B6MUd",
        "Amount": "2000000000",
        "PublicKey": "EDF7C3F9C80C102AF6D241752B37356E91ED454F26A35C567CF6F8477960F66614",
        "Signature": "F95675BA8FDA21030DE1B687937A79E8491CE51832D6BEEBC071484FA5AF5B8A0E9AFF11A4AA46F09ECFFB04C6A8DAE8284AF3ED8128C7D0046D842448478500",
        "WasLockingChainSend": 1,
        "AttestationRewardAccount": "rpFp36UHW6FpEcZjZqq5jSJWY6UCj3k4Es",
        "AttestationSignerAccount": "rpWLegmW9WrFBzHUj7brhQNZzrxgLj9oxw",
        "XChainAccountCreateCount": "2",
        "SignatureReward": "204",
        "XChainBridge": {
            "LockingChainDoor": "r3nCVTbZGGYoWvZ58BcxDmiMUU7ChMa1eC",
            "LockingChainIssue": {
                "currency": "XRP"
            },
            "IssuingChainDoor": "rHb9CJAWyB4rj91VRWn96DkukG4bwdtyTh",
            "IssuingChainIssue": {
                "currency": "XRP"
            }
        },
        "Fee": "20"
    }"#;
    use serde_json::Value;

    use super::*;

    #[test]
    fn test_deserialize() {
        let json = EXAMPLE_JSON;
        let deserialized: Result<XChainAddAccountCreateAttestation, _> = serde_json::from_str(json);
        assert!(deserialized.is_ok());
    }

    #[test]
    fn test_serialize() {
        let attestation: XChainAddAccountCreateAttestation<'_> =
            serde_json::from_str(EXAMPLE_JSON).unwrap();
        let actual = serde_json::to_value(&attestation).unwrap();
        let expected: Value = serde_json::from_str(EXAMPLE_JSON).unwrap();

        assert_eq!(actual, expected);
    }
}

#[cfg(test)]
mod test_xchain_claim {
    use crate::models::{
        transactions::xchain_claim::XChainClaim, Amount, IssuedCurrency, IssuedCurrencyAmount,
        Model, XChainBridge, XRPAmount, XRP,
    };
    use alloc::{borrow::Cow, string::ToString};

    const ACCOUNT: &str = "r9LqNeG6qHxjeUocjvVki2XR35weJ9mZgQ";
    const ACCOUNT2: &str = "rpZc4mVfWUif9CRoHRKKcmhu1nx2xktxBo";
    const FEE: &str = "0.00001";
    const SEQUENCE: u32 = 19048;
    const ISSUER: &str = "rGWrZyQqhTp9Xu7G5Pkayo7bXjH4k4QYpf";
    const GENESIS: &str = "rHb9CJAWyB4rj91VRWn96DkukG4bwdtyTh";
    const DESTINATION: &str = "rJrRMgiRgrU6hDF4pgu5DXQdWyPbY35ErN";
    const CLAIM_ID: u64 = 3;
    const XRP_AMOUNT: &str = "123456789";

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

    fn iou_amount<'a>() -> Amount<'a> {
        IssuedCurrencyAmount {
            currency: Cow::Borrowed("USD"),
            issuer: Cow::Borrowed(ISSUER),
            value: Cow::Borrowed("123"),
        }
        .into()
    }

    #[test]
    fn test_successful_claim_xrp() {
        let claim = XChainClaim::builder(Cow::Borrowed(ACCOUNT))
            .fee(XRPAmount::from(FEE))
            .sequence(SEQUENCE)
            .amount(XRPAmount::from(XRP_AMOUNT))
            .destination(Cow::Borrowed(DESTINATION))
            .xchain_bridge(xrp_bridge())
            .xchain_claim_id(CLAIM_ID.to_string())
            .build();
        assert!(claim.validate().is_ok());
    }

    #[test]
    fn test_successful_claim_iou() {
        let claim = XChainClaim::builder(Cow::Borrowed(ACCOUNT))
            .fee(XRPAmount::from(FEE))
            .sequence(SEQUENCE)
            .amount(iou_amount())
            .destination(Cow::Borrowed(DESTINATION))
            .xchain_bridge(iou_bridge())
            .xchain_claim_id(CLAIM_ID.to_string())
            .build();
        assert!(claim.validate().is_ok());
    }

    #[test]
    fn test_successful_claim_destination_tag() {
        let claim = XChainClaim::builder(Cow::Borrowed(ACCOUNT))
            .fee(XRPAmount::from(FEE))
            .sequence(SEQUENCE)
            .source_tag(12345)
            .amount(XRPAmount::from(XRP_AMOUNT))
            .destination(Cow::Borrowed(DESTINATION))
            .xchain_bridge(xrp_bridge())
            .xchain_claim_id(CLAIM_ID.to_string())
            .build();
        assert!(claim.validate().is_ok());
    }

    #[test]
    fn test_successful_claim_str_claim_id() {
        let claim_id_str = CLAIM_ID.to_string();
        let claim = XChainClaim::builder(Cow::Borrowed(ACCOUNT))
            .fee(XRPAmount::from(FEE))
            .sequence(SEQUENCE)
            .amount(XRPAmount::from(XRP_AMOUNT))
            .destination(Cow::Borrowed(DESTINATION))
            .xchain_bridge(xrp_bridge())
            .xchain_claim_id(claim_id_str.as_str())
            .build();
        assert!(claim.validate().is_ok());
    }

    #[test]
    #[should_panic]
    fn test_xrp_bridge_iou_amount() {
        let claim = XChainClaim::builder(Cow::Borrowed(ACCOUNT))
            .fee(XRPAmount::from(FEE))
            .sequence(SEQUENCE)
            .amount(iou_amount())
            .destination(Cow::Borrowed(DESTINATION))
            .xchain_bridge(xrp_bridge())
            .xchain_claim_id(CLAIM_ID.to_string())
            .build();
        claim.validate().unwrap();
    }

    #[test]
    #[should_panic]
    fn test_iou_bridge_xrp_amount() {
        let claim = XChainClaim::builder(Cow::Borrowed(ACCOUNT))
            .fee(XRPAmount::from(FEE))
            .sequence(SEQUENCE)
            .amount(XRPAmount::from(XRP_AMOUNT))
            .destination(Cow::Borrowed(DESTINATION))
            .xchain_bridge(iou_bridge())
            .xchain_claim_id(CLAIM_ID.to_string())
            .build();
        claim.validate().unwrap();
    }
}
