// Scenarios:
//   - base: create a payment channel from sender to destination with 100 drops and 86400s settle delay

use crate::common::{generate_funded_wallet, test_transaction, with_blockchain_lock};
use xrpl::models::transactions::payment_channel_create::PaymentChannelCreate;
use xrpl::models::XRPAmount;

#[tokio::test]
async fn test_payment_channel_create_base() {
    with_blockchain_lock(|| async {
        let wallet = generate_funded_wallet().await;
        let destination = generate_funded_wallet().await;

        let mut tx = PaymentChannelCreate::builder(wallet.classic_address.clone())
            .amount(XRPAmount::from("100"))
            .destination(destination.classic_address.clone())
            .public_key(wallet.public_key.clone())
            .settle_delay(86400)
            .build();

        test_transaction(&mut tx, &wallet).await;
    })
    .await;
}
