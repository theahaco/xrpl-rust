use serde_json::json;
use xrpl::asynch::clients::AsyncJsonRpcClient;
use xrpl::asynch::ledger::{get_fee, FeeType};

use crate::client;
use crate::commands::global::{NetworkArgs, OutputArgs};
use crate::error::Error;
use crate::output;

/// Report the node's current open-ledger fee.
///
/// Awaits the async helper directly. It used to call the **sync** one inside a
/// `block_on`, with a comment explaining that the sync helper needs a reactor
/// anyway — which is true, and is the problem: `src/ledger`'s wrappers drive
/// their futures with `embassy_futures::block_on`, a bare poll loop with no
/// reactor, so under `std` every one of them carries an undocumented
/// precondition that the caller already be inside a Tokio runtime. Building a
/// runtime purely to satisfy that, then running an `embassy` poll loop on one
/// of its worker threads, is a workaround for an interface that should not
/// need one. The `tx` group is async-only for the same reason.
#[derive(Debug, Clone, clap::Args)]
pub struct Cmd {
    #[command(flatten)]
    pub network: NetworkArgs,

    #[command(flatten)]
    pub output: OutputArgs,
}

impl Cmd {
    pub fn run(&self) -> Result<(), Error> {
        let url = self.network.url_or_mainnet();
        let runtime = client::runtime()?;

        let fee = runtime.block_on(async {
            let client = AsyncJsonRpcClient::connect(client::parse_url(&url)?);

            // Rendered inside the closure: `XRPAmount` borrows from the
            // client, which does not outlive this block.
            get_fee(&client, None, Some(FeeType::Open))
                .await
                .map(|fee| fee.to_string())
                .map_err(Error::Helper)
        })?;

        output::response(
            &json!({ "drops": fee.to_string() }),
            "open-ledger fee",
            self.output.json,
        )
    }
}
