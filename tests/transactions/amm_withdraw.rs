// Scenarios:
//   - single_asset: withdraw 500 XRP drops from an XRP/USD pool (TfSingleAsset flag)
//

use crate::common::amm::setup_amm_pool;
use crate::common::{test_transaction, with_blockchain_lock};
use xrpl::models::transactions::amm_withdraw::{AMMWithdraw, AMMWithdrawFlag};
use xrpl::models::{Amount, Currency, IssuedCurrency, XRPAmount, XRP};

#[tokio::test]
async fn test_amm_withdraw_single_asset() {
    with_blockchain_lock(|| async {
        let pool = setup_amm_pool().await;

        // Withdraw 500 XRP drops from the XRP side of the pool (TfSingleAsset).
        let mut tx = AMMWithdraw::builder(pool.lp_wallet.classic_address.clone())
            .flags(vec![AMMWithdrawFlag::TfSingleAsset])
            .asset(Currency::XRP(XRP::new()))
            .asset2(Currency::IssuedCurrency(IssuedCurrency::new(
                "USD".into(),
                pool.issuer_wallet.classic_address.clone().into(),
            )))
            .amount(Amount::XRPAmount(XRPAmount::from("500")))
            .build();

        test_transaction(&mut tx, &pool.lp_wallet).await;
    })
    .await;
}
