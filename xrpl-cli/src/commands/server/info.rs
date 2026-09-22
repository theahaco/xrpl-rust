use xrpl::clients::XRPLSyncClient;
use xrpl::models::requests::server_info::ServerInfo;

use crate::commands::global::NetworkArgs;
use crate::error::Error;
use crate::{client, output};

/// Fetch `server_info` from a node.
#[derive(Debug, Clone, clap::Args)]
pub struct Cmd {
    #[command(flatten)]
    pub network: NetworkArgs,
}

impl Cmd {
    pub fn run(&self) -> Result<(), Error> {
        let client = client::json_rpc(&self.network.url_or_mainnet())?;
        let request = ServerInfo::builder().build();

        output::response(client.request(request.into()), "Server info")
    }
}
