// Scenarios:
//   - base: sign a transaction, encode it, submit via the submit request,
//     and verify tesSUCCESS

use crate::common::with_blockchain_lock;
use xrpl::asynch::clients::XRPLAsyncClient;
use xrpl::core::binarycodec::encode;
use xrpl::models::requests::submit::Submit as SubmitRequest;
use xrpl::models::results::submit::Submit as SubmitResult;
use xrpl::models::transactions::payment::Payment;
use xrpl::models::{Amount, XRPAmount};

#[tokio::test]
async fn test_submit_base() {
    with_blockchain_lock(|| async {
        let client = crate::common::get_client().await;
        let wallet = crate::common::generate_funded_wallet().await;
        let destination = crate::common::generate_funded_wallet().await;

        // Build and sign a payment
        let mut payment = Payment::builder(wallet.classic_address.clone())
            .amount(Amount::XRPAmount(XRPAmount::from("1000000")))
            .destination(destination.classic_address.clone())
            .build();

        // Autofill sequence, fee, etc.
        xrpl::asynch::transaction::autofill_and_sign(&mut payment, client, &wallet, true)
            .await
            .expect("autofill_and_sign failed");

        // Encode to blob and submit
        let tx_blob = encode(&payment).expect("encode failed");
        let request = SubmitRequest::builder(tx_blob).build();

        let response = client
            .request(request.into())
            .await
            .expect("submit request failed");

        let result: SubmitResult = response.try_into().expect("failed to parse submit result");

        assert_eq!(result.engine_result.as_ref(), "tesSUCCESS");
        assert!(!result.tx_blob.is_empty());
        assert!(result.tx_json.is_object());
    })
    .await;
}
