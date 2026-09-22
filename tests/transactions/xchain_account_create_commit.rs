// Scenarios:
//   - base: committer funds creation of a new account on the issuing chain by locking
//           10_000_000 drops + signature_reward on the locking chain door
//
// The `destination` address does not need to be funded (it will be created on the issuing chain).

use crate::common::xchain::setup_bridge;
use crate::common::{generate_funded_wallet, test_transaction, with_blockchain_lock};
use xrpl::models::transactions::xchain_account_create_commit::XChainAccountCreateCommit;
use xrpl::models::{Amount, XRPAmount};
use xrpl::wallet::Wallet;

#[tokio::test]
async fn test_xchain_account_create_commit_base() {
    with_blockchain_lock(|| async {
        let bridge = setup_bridge().await;

        // Committer — a funded wallet on the locking chain.
        let committer = generate_funded_wallet().await;

        // Destination — an unfunded address that will be created on the issuing chain.
        let dest_seed = xrpl::core::keypairs::generate_seed(None, None).expect("seed");
        let dest_wallet = Wallet::new(&dest_seed, 0).expect("wallet");

        let mut tx = XChainAccountCreateCommit::builder(committer.classic_address.clone())
            .amount(Amount::XRPAmount(XRPAmount::from("10000000")))
            .destination(dest_wallet.classic_address.clone())
            .xchain_bridge(bridge.bridge())
            .signature_reward(Amount::XRPAmount(XRPAmount::from(
                bridge.signature_reward.as_str(),
            )))
            .build();

        test_transaction(&mut tx, &committer).await;
    })
    .await;
}
