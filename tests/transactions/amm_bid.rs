// Scenarios:
//   - base: LP holder bids for the AMM's auction slot (no BidMin/BidMax/AuthAccounts)

use crate::common::amm::setup_amm_pool;
use crate::common::{test_transaction, with_blockchain_lock};
use xrpl::models::transactions::amm_bid::AMMBid;
use xrpl::models::{Currency, IssuedCurrency, XRP};

#[tokio::test]
async fn test_amm_bid_base() {
    with_blockchain_lock(|| async {
        let pool = setup_amm_pool().await;

        let mut tx = AMMBid::builder(pool.lp_wallet.classic_address.clone())
            .asset(Currency::XRP(XRP::new()))
            .asset2(Currency::IssuedCurrency(IssuedCurrency::new(
                "USD".into(),
                pool.issuer_wallet.classic_address.clone().into(),
            )))
            .build();

        test_transaction(&mut tx, &pool.lp_wallet).await;
    })
    .await;
}
