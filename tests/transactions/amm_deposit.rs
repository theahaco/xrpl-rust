// Scenarios:
//   - single_asset: deposit 1000 XRP drops into an XRP/USD pool (TfSingleAsset flag)

use crate::common::amm::setup_amm_pool;
use crate::common::{test_transaction, with_blockchain_lock};
use xrpl::models::transactions::amm_deposit::{AMMDeposit, AMMDepositFlag};
use xrpl::models::{Amount, Currency, IssuedCurrency, XRPAmount, XRP};

#[tokio::test]
async fn test_amm_deposit_single_asset() {
    with_blockchain_lock(|| async {
        let pool = setup_amm_pool().await;

        // Deposit 1000 XRP drops into the XRP side of the pool (TfSingleAsset).
        let mut tx = AMMDeposit::builder(pool.lp_wallet.classic_address.clone())
            .flags(vec![AMMDepositFlag::TfSingleAsset])
            .asset(Currency::XRP(XRP::new()))
            .asset2(Currency::IssuedCurrency(IssuedCurrency::new(
                "USD".into(),
                pool.issuer_wallet.classic_address.clone().into(),
            )))
            .amount(Amount::XRPAmount(XRPAmount::from("1000")))
            .build();

        test_transaction(&mut tx, &pool.lp_wallet).await;
    })
    .await;
}
