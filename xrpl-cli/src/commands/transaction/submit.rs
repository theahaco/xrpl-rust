use xrpl::clients::XRPLSyncClient;
use xrpl::models::requests::submit::Submit;

use crate::commands::global::NetworkArgs;
use crate::error::Error;
use crate::{client, output};

/// Submit a signed transaction blob.
#[derive(Debug, Clone, clap::Args)]
pub struct Cmd {
    /// The signed transaction blob
    #[arg(short, long)]
    pub tx_blob: String,

    #[command(flatten)]
    pub network: NetworkArgs,
}

impl Cmd {
    pub fn run(&self) -> Result<(), Error> {
        let client = client::json_rpc(&self.network.url_or_mainnet())?;
        let request = Submit::builder(self.tx_blob.clone()).build();

        output::response(
            client.request(request.into()),
            "Transaction submission result",
        )
    }
}
