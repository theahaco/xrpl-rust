// Scenarios:
//   - base: door account creates an XRP/XRP bridge with genesis as the issuing chain door
//
// The `account` must be either the locking_chain_door or the issuing_chain_door.
// locking_chain_door == account, issuing_chain_door == GENESIS_ACCOUNT

use crate::common::constants::GENESIS_ACCOUNT;
use crate::common::{generate_funded_wallet, test_transaction, with_blockchain_lock};
use xrpl::models::transactions::xchain_create_bridge::XChainCreateBridge;
use xrpl::models::{Amount, Currency, XChainBridge, XRPAmount, XRP};

#[tokio::test]
async fn test_xchain_create_bridge_base() {
    with_blockchain_lock(|| async {
        let door_wallet = generate_funded_wallet().await;

        let mut tx = XChainCreateBridge::builder(door_wallet.classic_address.clone())
            .signature_reward(Amount::XRPAmount(XRPAmount::from("200")))
            .xchain_bridge(XChainBridge {
                issuing_chain_door: GENESIS_ACCOUNT.into(),
                issuing_chain_issue: Currency::XRP(XRP::new()),
                locking_chain_door: door_wallet.classic_address.clone().into(),
                locking_chain_issue: Currency::XRP(XRP::new()),
            })
            .min_account_create_amount(XRPAmount::from("10000000"))
            .build();

        test_transaction(&mut tx, &door_wallet).await;
    })
    .await;
}
