use xrpl::clients::XRPLSyncClient;
use xrpl::models::requests::account_info::AccountInfo;

use crate::commands::global::NetworkArgs;
use crate::error::Error;
use crate::{client, output};

/// Fetch an account's root entry.
#[derive(Debug, Clone, clap::Args)]
pub struct Cmd {
    /// The account address
    #[arg(long)]
    pub address: String,

    #[command(flatten)]
    pub network: NetworkArgs,
}

impl Cmd {
    pub fn run(&self) -> Result<(), Error> {
        let client = client::json_rpc(&self.network.url_or_mainnet())?;
        let request = AccountInfo::builder(self.address.clone()).build();

        output::response(client.request(request.into()), "Account info")
    }
}
