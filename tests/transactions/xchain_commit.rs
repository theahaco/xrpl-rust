// Scenarios:
//   - base: committer locks 10_000_000 drops onto the locking chain door (XChainClaimID = 1)
//
// xchain_claim_id is Cow<str> even though it is semantically a number.

use crate::common::xchain::setup_bridge;
use crate::common::{generate_funded_wallet, test_transaction, with_blockchain_lock};
use xrpl::models::transactions::xchain_commit::XChainCommit;
use xrpl::models::{Amount, XRPAmount};

#[tokio::test]
async fn test_xchain_commit_base() {
    with_blockchain_lock(|| async {
        let bridge = setup_bridge().await;

        // Committer — a separate funded wallet on the locking chain.
        let committer = generate_funded_wallet().await;

        let mut tx = XChainCommit::builder(committer.classic_address.clone())
            .amount(Amount::XRPAmount(XRPAmount::from("10000000")))
            .xchain_bridge(bridge.bridge())
            .xchain_claim_id("1")
            .build();

        test_transaction(&mut tx, &committer).await;
    })
    .await;
}
