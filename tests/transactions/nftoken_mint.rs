// Scenarios:
//   - base: mint an NFT with a URI

use crate::common::{generate_funded_wallet, test_transaction, with_blockchain_lock};
use xrpl::models::transactions::nftoken_mint::NFTokenMint;

const TEST_NFT_URL: &str = "https://example.com/nft.json";

#[tokio::test]
async fn test_nftoken_mint_base() {
    with_blockchain_lock(|| async {
        // Fresh wallet: NFTokenMint modifies the account's NFToken page objects.
        let wallet = generate_funded_wallet().await;

        let mut tx = NFTokenMint::builder(wallet.classic_address.clone())
            .nftoken_taxon(0)
            .uri(hex::encode(TEST_NFT_URL))
            .build();

        test_transaction(&mut tx, &wallet).await;
    })
    .await;
}
