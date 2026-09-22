use xrpl::clients::XRPLSyncClient;
use xrpl::models::requests::account_currencies::AccountCurrencies;

use crate::commands::global::NetworkArgs;
use crate::error::Error;
use crate::{client, output};

/// List the currencies an account can send or receive.
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
        let request = AccountCurrencies::builder(self.address.clone()).build();

        output::response(client.request(request.into()), "Account currencies")
    }
}
