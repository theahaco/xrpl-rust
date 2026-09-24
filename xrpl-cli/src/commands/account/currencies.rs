use xrpl::clients::XRPLSyncClient;
use xrpl::models::requests::account_currencies::AccountCurrencies;

use crate::commands::global::{LedgerArgs, NetworkArgs, OutputArgs};
use crate::error::Error;
use crate::{client, output};

/// List the currencies an account can send or receive.
#[derive(Debug, Clone, clap::Args)]
pub struct Cmd {
    #[command(flatten)]
    pub subject: super::subject::AccountArg,

    #[command(flatten)]
    pub ledger: LedgerArgs,

    #[command(flatten)]
    pub network: NetworkArgs,

    #[command(flatten)]
    pub output: OutputArgs,
}

impl Cmd {
    pub fn run(&self) -> Result<(), Error> {
        let client = client::json_rpc(&self.network.url_or_mainnet())?;
        let request = AccountCurrencies::builder(self.subject.address()?)
            .maybe_ledger_hash(self.ledger.hash())
            .maybe_ledger_index(self.ledger.index())
            .build();

        let response = client.request(request.into()).map_err(Error::Client)?;
        output::response(
            &client::ok_result(&response)?,
            "Account currencies",
            self.output.json,
        )
    }
}
