use xrpl::asynch::clients::{AsyncJsonRpcClient, XRPLAsyncClient};
use xrpl::models::requests::account_info::AccountInfo;

#[tokio::main]
async fn main() {
    // connect to a XRP Ledger node
    let client = AsyncJsonRpcClient::connect("https://xrplcluster.com/".parse().unwrap());
    // request account info
    let account_info = AccountInfo::builder("r9cZA1mLK5R5Am25ArfXFmqgNwjZgnfk59").build();
    let response = client.request(account_info.into()).await.unwrap();
    println!("account info: {:?}", response);
}
