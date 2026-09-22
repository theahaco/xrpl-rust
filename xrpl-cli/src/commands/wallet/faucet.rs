use xrpl::asynch::clients::AsyncJsonRpcClient;
use xrpl::asynch::wallet::generate_faucet_wallet;

use crate::client;
use crate::commands::global::{NetworkArgs, DEFAULT_TESTNET_URL};
use crate::error::Error;

/// Generate a wallet funded by the network's faucet.
///
/// Standalone nodes have no faucet; see the funding instructions in the README
/// for how to fund an account from the genesis account instead.
#[derive(Debug, Clone, clap::Args)]
pub struct Cmd {
    #[command(flatten)]
    pub network: NetworkArgs,
}

impl Cmd {
    pub fn run(&self) -> Result<(), Error> {
        let url = self.network.json_rpc_url(DEFAULT_TESTNET_URL);
        let runtime = client::runtime()?;

        let result = runtime.block_on(async {
            let client = AsyncJsonRpcClient::connect(url.parse()?);
            generate_faucet_wallet(&client, None, None, None, None).await
        });

        match result {
            Ok(wallet) => {
                println!("Generated faucet wallet: {wallet:#?}");
                Ok(())
            }
            Err(error) => Err(Error::other(format!(
                "Failed to generate faucet wallet: {error}"
            ))),
        }
    }
}
