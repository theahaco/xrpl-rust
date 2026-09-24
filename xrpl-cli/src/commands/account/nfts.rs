use xrpl::clients::XRPLSyncClient;
use xrpl::models::requests::account_nfts::AccountNfts;

use crate::commands::global::{NetworkArgs, OutputArgs};
use crate::error::Error;
use crate::{client, output};

/// List the NFTs an account holds.
#[derive(Debug, Clone, clap::Args)]
pub struct Cmd {
    #[command(flatten)]
    pub subject: super::subject::AccountArg,

    #[command(flatten)]
    pub network: NetworkArgs,

    #[command(flatten)]
    pub output: OutputArgs,
}

impl Cmd {
    pub fn run(&self) -> Result<(), Error> {
        let client = client::json_rpc(&self.network.url_or_mainnet())?;
        let request = AccountNfts::builder(self.subject.address()?).build();

        let response = client.request(request.into()).map_err(Error::Client)?;
        output::response(
            &client::ok_result(&response)?,
            "Account NFTs",
            self.output.json,
        )
    }
}
