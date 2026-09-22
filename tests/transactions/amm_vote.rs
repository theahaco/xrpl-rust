// Scenarios:
//   - base: LP holder votes to change trading_fee to 150 (per 100_000)

use crate::common::amm::setup_amm_pool;
use crate::common::{test_transaction, with_blockchain_lock};
use xrpl::models::transactions::amm_vote::AMMVote;
use xrpl::models::{Currency, IssuedCurrency, XRP};

#[tokio::test]
async fn test_amm_vote_base() {
    with_blockchain_lock(|| async {
        let pool = setup_amm_pool().await;

        let mut tx = AMMVote::builder(pool.lp_wallet.classic_address.clone())
            .asset(Currency::XRP(XRP::new()))
            .asset2(Currency::IssuedCurrency(IssuedCurrency::new(
                "USD".into(),
                pool.issuer_wallet.classic_address.clone().into(),
            )))
            .trading_fee(150)
            .build();

        test_transaction(&mut tx, &pool.lp_wallet).await;
    })
    .await;
}
