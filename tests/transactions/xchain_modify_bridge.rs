// Scenarios:
//   - base: create a bridge then modify the signature_reward from 200 to 300 drops
//

use crate::common::xchain::setup_bridge;
use crate::common::{test_transaction, with_blockchain_lock};
use xrpl::models::transactions::xchain_modify_bridge::XChainModifyBridge;
use xrpl::models::{Amount, XRPAmount};

#[tokio::test]
async fn test_xchain_modify_bridge_base() {
    with_blockchain_lock(|| async {
        let bridge = setup_bridge().await;

        // Modify the signature_reward from 200 → 300 drops.
        let mut tx = XChainModifyBridge::builder(bridge.door_wallet.classic_address.clone())
            .xchain_bridge(bridge.bridge())
            .signature_reward(Amount::XRPAmount(XRPAmount::from("300")))
            .build();

        test_transaction(&mut tx, &bridge.door_wallet).await;
    })
    .await;
}
