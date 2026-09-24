use xrpl::clients::XRPLSyncClient;
use xrpl::models::requests::account_info::AccountInfo;

use crate::commands::global::{NetworkArgs, OutputArgs};
use crate::error::Error;
use crate::{client, output};

/// Fetch an account's root entry.
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
        let request = AccountInfo::builder(self.subject.address()?).build();

        let response = client.request(request.into()).map_err(Error::Client)?;
        output::response(
            &client::result_value(&response)?,
            "Account info",
            self.output.json,
        )
    }
}
