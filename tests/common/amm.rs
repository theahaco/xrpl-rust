// Shared AMM pool setup helper used by all AMM integration tests.
//
// Creates a minimal XRP/USD AMM pool:
//   1. issuerWallet:  AccountSet — enable DefaultRipple so the IOU can flow
//   2. lpWallet:      TrustSet  — trust issuer for up to 1000 USD (tfClearNoRipple)
//   3. issuerWallet:  Payment   — send 500 USD to lpWallet
//   4. lpWallet:      AMMCreate — 250 XRP drops + 250 USD, trading_fee = 12
//   5. lpWallet:      AMMDeposit — 1000 XRP drops (TfSingleAsset) so
//                     pool has enough XRP for a 500-drop single-asset withdraw.
//
// The returned `AmmPool` carries both wallets so individual tests can build
// `Currency` / `IssuedCurrencyAmount` values from `issuer_wallet.classic_address`.

use super::{generate_funded_wallet, test_transaction};
use xrpl::models::transactions::account_set::{AccountSet, AccountSetFlag};
use xrpl::models::transactions::amm_create::AMMCreate;
use xrpl::models::transactions::amm_deposit::{AMMDeposit, AMMDepositFlag};
use xrpl::models::transactions::payment::Payment;
use xrpl::models::transactions::trust_set::{TrustSet, TrustSetFlag};
use xrpl::models::{Amount, Currency, IssuedCurrencyAmount, XRPAmount, XRP};
use xrpl::wallet::Wallet;

pub struct AmmPool {
    pub lp_wallet: Wallet,
    pub issuer_wallet: Wallet,
}

#[cfg(feature = "std")]
pub async fn setup_amm_pool() -> AmmPool {
    let issuer_wallet = generate_funded_wallet().await;
    let lp_wallet = generate_funded_wallet().await;

    // Step 1: enable DefaultRipple on issuer so the USD IOU can flow through AMM
    let mut set_tx = AccountSet::builder(issuer_wallet.classic_address.clone())
        .set_flag(AccountSetFlag::AsfDefaultRipple)
        .build();
    test_transaction(&mut set_tx, &issuer_wallet).await;

    // Step 2: lp_wallet sets trust line to issuer for 1000 USD
    let mut trust_tx = TrustSet::builder(lp_wallet.classic_address.clone())
        .flags(vec![TrustSetFlag::TfClearNoRipple])
        .limit_amount(IssuedCurrencyAmount::new(
            "USD".into(),
            issuer_wallet.classic_address.clone().into(),
            "1000".into(),
        ))
        .build();
    test_transaction(&mut trust_tx, &lp_wallet).await;

    // Step 3: issuer sends 500 USD to lp_wallet
    let mut pay_tx = Payment::builder(issuer_wallet.classic_address.clone())
        .amount(Amount::IssuedCurrencyAmount(IssuedCurrencyAmount::new(
            "USD".into(),
            issuer_wallet.classic_address.clone().into(),
            "500".into(),
        )))
        .destination(lp_wallet.classic_address.clone())
        .build();
    test_transaction(&mut pay_tx, &issuer_wallet).await;

    // Step 4: lp_wallet creates the AMM with 250 XRP drops + 250 USD, fee = 12
    // AMMCreate has no flags; uses standard 9 common fields.
    // AMMCreate requires a fee equal to the owner reserve (inc_reserve = 5 XRP = 5_000_000 drops
    // in the standalone Docker image). The SDK's autofill handles this via
    // calculate_fee_per_transaction_type which returns get_owner_reserve for AMMCreate.
    let mut amm_tx = AMMCreate::builder(lp_wallet.classic_address.clone())
        .amount(Amount::XRPAmount(XRPAmount::from("250")))
        .amount2(Amount::IssuedCurrencyAmount(IssuedCurrencyAmount::new(
            "USD".into(),
            issuer_wallet.classic_address.clone().into(),
            "250".into(),
        )))
        .trading_fee(12)
        .build();
    test_transaction(&mut amm_tx, &lp_wallet).await;

    // Step 5: lp_wallet deposits 1000 XRP drops (TfSingleAsset) so pool has
    // enough XRP for a 500-drop single-asset withdraw in amm_withdraw tests.
    // Adds liquidity so pool has enough XRP for single-asset withdraw tests.
    let mut deposit_tx = AMMDeposit::builder(lp_wallet.classic_address.clone())
        .flags(vec![AMMDepositFlag::TfSingleAsset])
        .asset(Currency::XRP(XRP::new()))
        .asset2(Currency::IssuedCurrency(xrpl::models::IssuedCurrency::new(
            "USD".into(),
            issuer_wallet.classic_address.clone().into(),
        )))
        .amount(Amount::XRPAmount(XRPAmount::from("1000")))
        .build();
    test_transaction(&mut deposit_tx, &lp_wallet).await;

    AmmPool {
        lp_wallet,
        issuer_wallet,
    }
}
