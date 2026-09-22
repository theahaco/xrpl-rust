// Scenarios:
//   - base: create a Check for 50 drops and verify one check object exists on the ledger

use crate::common::{generate_funded_wallet, get_client, test_transaction, with_blockchain_lock};
use xrpl::asynch::clients::XRPLAsyncClient;
use xrpl::models::requests::account_objects::{AccountObjectType, AccountObjects};
use xrpl::models::results;
use xrpl::models::transactions::check_create::CheckCreate;
use xrpl::models::{Amount, XRPAmount};

#[tokio::test]
async fn test_check_create_base() {
    with_blockchain_lock(|| async {
        let client = get_client().await;
        let wallet = generate_funded_wallet().await;
        let destination = generate_funded_wallet().await;

        let mut tx = CheckCreate::builder(wallet.classic_address.clone())
            .destination(destination.classic_address.clone())
            .send_max(Amount::XRPAmount(XRPAmount::from("50")))
            .build();

        test_transaction(&mut tx, &wallet).await;

        // Verify the check ledger object was created
        let ao_response = client
            .request(
                AccountObjects::builder(wallet.classic_address.clone())
                    .r#type(AccountObjectType::Check)
                    .build()
                    .into(),
            )
            .await
            .expect("Failed to query account_objects");
        let ao_result: results::account_objects::AccountObjects<'_> = ao_response
            .try_into()
            .expect("Failed to parse account_objects");

        assert_eq!(
            ao_result.account_objects.len(),
            1,
            "Should be exactly one check on the ledger"
        );
    })
    .await;
}
