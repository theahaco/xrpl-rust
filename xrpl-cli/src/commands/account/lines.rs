use xrpl::clients::XRPLSyncClient;
use xrpl::models::requests::account_lines::AccountLines;

use crate::commands::global::{LedgerArgs, NetworkArgs, OutputArgs, DEFAULT_PAGINATION_LIMIT};
use crate::error::Error;
use crate::{client, output};

/// List an account's trust lines.
#[derive(Debug, Clone, clap::Args)]
pub struct Cmd {
    #[command(flatten)]
    pub subject: super::subject::AccountArg,

    /// Peer account to filter trust lines
    #[arg(long)]
    pub peer: Option<String>,

    /// Limit the number of trust lines returned
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
        let request = AccountLines::builder(self.subject.address()?)
            .limit(self.limit)
            .maybe_peer(self.peer.as_deref())
            .maybe_ledger_hash(self.ledger.hash())
            .maybe_ledger_index(self.ledger.index())
            .build();

        let response = client.request(request.into()).map_err(Error::Client)?;
        output::response(
            &client::ok_result(&response)?,
            "Account trust lines",
            self.output.json,
        )
    }
}
