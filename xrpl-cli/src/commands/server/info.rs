use xrpl::clients::XRPLSyncClient;
use xrpl::models::requests::server_info::ServerInfo;

use crate::commands::global::{NetworkArgs, OutputArgs};
use crate::error::Error;
use crate::{client, output};

/// Fetch `server_info` from a node.
#[derive(Debug, Clone, clap::Args)]
pub struct Cmd {
    #[command(flatten)]
    pub network: NetworkArgs,

    #[command(flatten)]
    pub output: OutputArgs,
}

impl Cmd {
    pub fn run(&self) -> Result<(), Error> {
        let client = client::json_rpc(&self.network.url_or_mainnet())?;
        let request = ServerInfo::builder().build();

        let response = client.request(request.into()).map_err(Error::Client)?;
        output::response(
            &client::result_value(&response)?,
            "Server info",
            self.output.json,
        )
    }
}
