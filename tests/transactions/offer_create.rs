// Scenarios:
//   - base: place an XRP/USD offer on the DEX
//
// NOTE: Bitstamp (rvYAfWj5gh67oV6fW32ZzP3Aw4Eubs59B) does not exist in standalone Docker mode.
// A fresh issuer wallet is funded from genesis to act as the IOU issuer.
// rippled requires the issuer account to exist (tecNO_ISSUER otherwise).

use crate::common::{generate_funded_wallet, test_transaction, with_blockchain_lock};
use xrpl::models::{
    transactions::offer_create::OfferCreate, Amount, IssuedCurrencyAmount, XRPAmount,
};

#[tokio::test]
async fn test_offer_create_base() {
    with_blockchain_lock(|| async {
        let wallet = generate_funded_wallet().await;
        let issuer = generate_funded_wallet().await;

        let mut tx = OfferCreate::builder(wallet.classic_address.clone())
            .taker_gets(Amount::XRPAmount(XRPAmount::from("100")))
            .taker_pays(Amount::IssuedCurrencyAmount(IssuedCurrencyAmount::new(
                "USD".into(),
                issuer.classic_address.clone().into(),
                "10".into(),
            )))
            .build();

        test_transaction(&mut tx, &wallet).await;
    })
    .await;
}
