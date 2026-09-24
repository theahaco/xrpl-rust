use xrpl::clients::XRPLSyncClient;
use xrpl::models::requests::account_lines::AccountLines;

use crate::commands::global::{NetworkArgs, DEFAULT_PAGINATION_LIMIT};
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
    pub network: NetworkArgs,
}

impl Cmd {
    pub fn run(&self) -> Result<(), Error> {
        let client = client::json_rpc(&self.network.url_or_mainnet())?;
        let request = AccountLines::builder(self.subject.address()?)
            .limit(self.limit)
            .maybe_peer(self.peer.as_deref())
            .build();

        output::response(client.request(request.into()), "Account trust lines")
    }
}
