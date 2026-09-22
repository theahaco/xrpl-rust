// Scenarios:
//   - base: create a channel, then add 100 more drops via PaymentChannelFund
//
// NOTE: xrpl-rust has no hashPaymentChannel utility, so we read the channel ID from
// account_objects after the PaymentChannelCreate is validated.

use crate::common::{
    generate_funded_wallet, get_client, ledger_accept, test_transaction, with_blockchain_lock,
};
use xrpl::asynch::{clients::XRPLAsyncClient, transaction::sign_and_submit};
use xrpl::models::requests::account_objects::{AccountObjectType, AccountObjects};
use xrpl::models::results;
use xrpl::models::transactions::payment_channel_create::PaymentChannelCreate;
use xrpl::models::transactions::payment_channel_fund::PaymentChannelFund;
use xrpl::models::XRPAmount;

#[tokio::test]
async fn test_payment_channel_fund_base() {
    with_blockchain_lock(|| async {
        let client = get_client().await;
        let wallet = generate_funded_wallet().await;
        let destination = generate_funded_wallet().await;

        // Step 1: create the payment channel
        let mut create_tx = PaymentChannelCreate::builder(wallet.classic_address.clone())
            .amount(XRPAmount::from("100"))
            .destination(destination.classic_address.clone())
            .public_key(wallet.public_key.clone())
            .settle_delay(86400)
            .build();

        sign_and_submit(&mut create_tx, client, &wallet, true, true)
            .await
            .expect("Failed to submit PaymentChannelCreate");

        ledger_accept().await;

        // Step 2: get the channel ID from account_objects
        let ao_response = client
            .request(
                AccountObjects::builder(wallet.classic_address.clone())
                    .r#type(AccountObjectType::PaymentChannel)
                    .build()
                    .into(),
            )
            .await
            .expect("Failed to query account_objects");
        let ao_result: results::account_objects::AccountObjects<'_> = ao_response
            .try_into()
            .expect("Failed to parse account_objects");

        assert_eq!(ao_result.account_objects.len(), 1, "Expected one channel");

        let channel_id = ao_result.account_objects[0]["index"]
            .as_str()
            .expect("Expected index field on channel object")
            .to_string();

        // Step 3: fund the channel with an additional 100 drops
        let mut fund_tx = PaymentChannelFund::builder(wallet.classic_address.clone())
            .amount(XRPAmount::from("100"))
            .channel(channel_id)
            .build();

        test_transaction(&mut fund_tx, &wallet).await;
    })
    .await;
}
