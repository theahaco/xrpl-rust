use alloc::{borrow::Cow, vec::Vec};
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;

use crate::models::{Amount, FlagCollection, Model, NoFlags, ValidateCurrencies, XChainBridge};

use super::{CommonFields, Transaction, TransactionType};

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, xrpl_rust_macros::ValidateCurrencies)]
#[serde(rename_all = "PascalCase")]
pub struct XChainAddClaimAttestation<'a> {
    #[serde(flatten)]
    pub common_fields: CommonFields<'a, NoFlags>,
    pub amount: Amount<'a>,
    pub attestation_reward_account: Cow<'a, str>,
    pub attestation_signer_account: Cow<'a, str>,
    pub other_chain_source: Cow<'a, str>,
    pub public_key: Cow<'a, str>,
    pub signature: Cow<'a, str>,
    pub was_locking_chain_send: u8,
    #[serde(rename = "XChainBridge")]
    pub xchain_bridge: XChainBridge<'a>,
    #[serde(rename = "XChainClaimID")]
    pub xchain_claim_id: Cow<'a, str>,
    pub destination: Option<Cow<'a, str>>,
}

impl Model for XChainAddClaimAttestation<'_> {
    fn get_errors(&self) -> crate::models::XRPLModelResult<()> {
        self.validate_currencies()
    }
}

impl<'a> Transaction<'a, NoFlags> for XChainAddClaimAttestation<'a> {
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
impl<'a> XChainAddClaimAttestation<'a> {
    #[builder]
    pub fn new(
        #[builder(start_fn, into)] account: Cow<'a, str>,
        #[builder(into)] account_txn_id: Option<Cow<'a, str>>,
        #[builder(into)] fee: Option<crate::models::XRPAmount<'a>>,
        last_ledger_sequence: Option<u32>,
        #[builder(into)] memos: Option<Vec<super::Memo>>,
        sequence: Option<u32>,
        #[builder(into)] signers: Option<Vec<super::Signer>>,
        source_tag: Option<u32>,
        ticket_sequence: Option<u32>,
        #[builder(into)] amount: Amount<'a>,
        #[builder(into)] attestation_reward_account: Cow<'a, str>,
        #[builder(into)] attestation_signer_account: Cow<'a, str>,
        #[builder(into)] other_chain_source: Cow<'a, str>,
        #[builder(into)] public_key: Cow<'a, str>,
        #[builder(into)] signature: Cow<'a, str>,
        was_locking_chain_send: u8,
        #[builder(into)] xchain_bridge: XChainBridge<'a>,
        #[builder(into)] xchain_claim_id: Cow<'a, str>,
        #[builder(into)] destination: Option<Cow<'a, str>>,
    ) -> XChainAddClaimAttestation<'a> {
        XChainAddClaimAttestation {
            common_fields: CommonFields::builder(account)
                .transaction_type(TransactionType::XChainAddClaimAttestation)
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
            other_chain_source,
            public_key,
            signature,
            was_locking_chain_send,
            xchain_bridge,
            xchain_claim_id,
            destination,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::amount::XRPAmount;
    use crate::models::currency::XRP;

    fn xrp_bridge<'a>() -> XChainBridge<'a> {
        XChainBridge {
            locking_chain_door: "rMAXACCrp3Y8PpswXcg3bKggHX76V3F8M4".into(),
            locking_chain_issue: XRP::new().into(),
            issuing_chain_door: "rHb9CJAWyB4rj91VRWn96DkukG4bwdtyTh".into(),
            issuing_chain_issue: XRP::new().into(),
        }
    }

    #[test]
    fn test_serde_round_trip() {
        let txn = XChainAddClaimAttestation::builder("rPV4mZjsXfH2HvUSPLNmqz1J8d3Lpv7tpe")
            .fee(XRPAmount::from("10"))
            .sequence(1)
            .amount(Amount::XRPAmount(XRPAmount::from("10000")))
            .attestation_reward_account("rPV4mZjsXfH2HvUSPLNmqz1J8d3Lpv7tpe")
            .attestation_signer_account("rPV4mZjsXfH2HvUSPLNmqz1J8d3Lpv7tpe")
            .other_chain_source("rSrc111111111111111111111111111111")
            .public_key("ED1234567890ABCDEF1234567890ABCDEF1234567890ABCDEF1234567890ABCDEF")
            .signature("30440220ABCDEF")
            .was_locking_chain_send(1)
            .xchain_bridge(xrp_bridge())
            .xchain_claim_id("13f")
            .destination("rDest11111111111111111111111111111")
            .build();
        let serialized = serde_json::to_string(&txn).unwrap();
        let deserialized: XChainAddClaimAttestation = serde_json::from_str(&serialized).unwrap();
        let reserialized = serde_json::to_string(&deserialized).unwrap();
        assert_eq!(serialized, reserialized);
        assert!(serialized.contains("\"TransactionType\":\"XChainAddClaimAttestation\""));
        assert!(serialized.contains("\"XChainBridge\""));
        assert!(serialized.contains("\"XChainClaimID\":\"13f\""));
        assert!(serialized.contains("\"WasLockingChainSend\":1"));
    }

    #[test]
    fn test_get_transaction_type() {
        let txn = XChainAddClaimAttestation::builder("rPV4mZjsXfH2HvUSPLNmqz1J8d3Lpv7tpe")
            .amount(Amount::XRPAmount(XRPAmount::from("10000")))
            .attestation_reward_account("rPV4mZjsXfH2HvUSPLNmqz1J8d3Lpv7tpe")
            .attestation_signer_account("rPV4mZjsXfH2HvUSPLNmqz1J8d3Lpv7tpe")
            .other_chain_source("rSrc111111111111111111111111111111")
            .public_key("ED00")
            .signature("30")
            .was_locking_chain_send(0)
            .xchain_bridge(xrp_bridge())
            .xchain_claim_id("1")
            .build();
        assert_eq!(
            txn.get_transaction_type(),
            &TransactionType::XChainAddClaimAttestation
        );
    }
}
