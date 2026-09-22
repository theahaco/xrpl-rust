// Scenarios:
//   - base: submit a payment transaction, then query it by hash using the tx request

use crate::common::with_blockchain_lock;
use xrpl::asynch::clients::XRPLAsyncClient;
use xrpl::asynch::transaction::sign_and_submit;
use xrpl::models::requests::tx::Tx as TxRequest;
use xrpl::models::results::tx::TxVersionMap;
use xrpl::models::transactions::payment::Payment;
use xrpl::models::{Amount, XRPAmount};

#[tokio::test]
async fn test_tx_base() {
    with_blockchain_lock(|| async {
        let client = crate::common::get_client().await;
        let wallet = crate::common::generate_funded_wallet().await;
        let destination = crate::common::generate_funded_wallet().await;

        // Submit a payment so we have a transaction to look up
        let mut payment = Payment::builder(wallet.classic_address.clone())
            .amount(Amount::XRPAmount(XRPAmount::from("1000000")))
            .destination(destination.classic_address.clone())
            .build();

        let submit_result = sign_and_submit(&mut payment, client, &wallet, true, true)
            .await
            .expect("sign_and_submit failed");
        assert_eq!(submit_result.engine_result.as_ref(), "tesSUCCESS");

        // Extract the hash from the submit result
        let tx_hash = submit_result
            .tx_json
            .get("hash")
            .expect("submit result should contain hash")
            .as_str()
            .expect("hash should be a string");

        // Advance the ledger so the transaction is validated
        crate::common::ledger_accept().await;

        // Query the transaction by hash
        let request = TxRequest::builder()
            .transaction(tx_hash.to_string())
            .build();

        let response = client
            .request(request.into())
            .await
            .expect("tx request failed");

        let result: TxVersionMap = response.try_into().expect("failed to parse tx result");

        // Verify the hash matches what we submitted
        match &result {
            TxVersionMap::Default(tx) => {
                assert_eq!(tx.base.hash.as_ref(), tx_hash);
                assert!(tx.base.validated.unwrap_or(false));
            }
            TxVersionMap::V1(tx) => {
                assert_eq!(tx.base.hash.as_ref(), tx_hash);
            }
        }
    })
    .await;
}
