// Scenarios:
//   - base: create a time-locked XRP escrow (FinishAfter = close_time + 2) and verify tesSUCCESS
//
// NOTE: FinishAfter is set slightly ahead of the current ledger close_time.
// On testnet ledgers close automatically every ~3-4 s, so EscrowFinish and EscrowCancel
// scenarios (which require waiting for time to advance) live in their own test files.

use crate::common::{
    generate_funded_wallet, get_ledger_close_time, test_transaction, with_blockchain_lock,
};
use xrpl::models::transactions::escrow_create::EscrowCreate;

#[tokio::test]
async fn test_escrow_create_base() {
    with_blockchain_lock(|| async {
        let wallet = generate_funded_wallet().await;
        let destination = generate_funded_wallet().await;

        let close_time = get_ledger_close_time().await;
        let finish_after = (close_time + 2) as u32;

        let mut tx = EscrowCreate::builder(wallet.classic_address.clone())
            .amount("10000")
            .destination(destination.classic_address.clone())
            .finish_after(finish_after)
            .build();

        test_transaction(&mut tx, &wallet).await;
    })
    .await;
}
