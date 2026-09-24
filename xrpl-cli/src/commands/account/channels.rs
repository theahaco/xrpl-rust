use xrpl::clients::XRPLSyncClient;
use xrpl::models::requests::account_channels::AccountChannels;

use crate::commands::global::{LedgerArgs, NetworkArgs, OutputArgs, DEFAULT_PAGINATION_LIMIT};
use crate::error::Error;
use crate::{client, output};

/// List an account's payment channels.
#[derive(Debug, Clone, clap::Args)]
pub struct Cmd {
    #[command(flatten)]
    pub subject: super::subject::AccountArg,

    /// Destination account to filter channels
    #[arg(long)]
    pub destination_account: Option<String>,

    /// Limit the number of channels returned
    #[arg(long, default_value_t = DEFAULT_PAGINATION_LIMIT as u16)]
    pub limit: u16,

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
        let request = AccountChannels::builder(self.subject.address()?)
            .maybe_destination_account(self.destination_account.as_deref())
            .limit(self.limit)
            .maybe_ledger_hash(self.ledger.hash())
            .maybe_ledger_index(self.ledger.index())
            .build();

        let response = client.request(request.into()).map_err(Error::Client)?;
        output::response(
            &client::ok_result(&response)?,
            "Account channels",
            self.output.json,
        )
    }
}
