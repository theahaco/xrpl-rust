use serde_json::json;
use xrpl::asynch::clients::AsyncJsonRpcClient;
use xrpl::asynch::wallet::generate_faucet_wallet;

use crate::client;
use crate::commands::global::{NetworkArgs, DEFAULT_TESTNET_URL};
use crate::error::Error;
use crate::output;

/// Generate a wallet funded by the network's faucet.
///
/// Standalone nodes have no faucet; see the funding instructions in the README
/// for how to fund an account from the genesis account instead.
///
/// # The seed, and why this is not the recommended path
///
/// This prints a real funded account, and its 16-byte family seed is the only
/// backup that exists — there is no mnemonic convention in this crate. It used
/// to print `Wallet`'s `Debug`, which redacts the seed, so the account was
/// unrecoverable the moment the process exited: the redaction protected
/// nothing and cost everything.
///
/// So it prints JSON, and the seed only under `--show-secret`, exactly as
/// `key generate` does. The supported path is `key generate`, `account add`,
/// then `account fund`, which enrolls the key encrypted at rest before funding;
/// the whole `wallet` group is deprecated in favour of it.
#[derive(Debug, Clone, clap::Args)]
pub struct Cmd {
    /// Print the seed on stdout.
    ///
    /// Without it the seed is gone when this process exits, and the funded
    /// account with it.
    #[arg(long)]
    pub show_secret: bool,

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

        let wallet = result
            .map_err(|error| Error::other(format!("Failed to generate faucet wallet: {error}")))?;

        let mut value = json!({
            "classic_address": wallet.classic_address,
            "public_key": wallet.public_key,
        });

        if self.show_secret {
            value["seed"] = json!(wallet.seed.as_str());
        } else {
            output::warn(
                "the seed was not printed, so this funded account is now unrecoverable. \
                 Re-run with --show-secret, or use `xrpl key generate` and \
                 `xrpl account add`, then `xrpl account fund` to enroll the key before funding.",
            );
        }

        output::artifact(&value)
    }
}
