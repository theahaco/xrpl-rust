use xrpl::clients::XRPLSyncClient;
use xrpl::models::requests::account_info::AccountInfo;

use crate::commands::global::{LedgerArgs, NetworkArgs, OutputArgs};
use crate::error::Error;
use crate::{client, output};

/// Fetch an account's root entry.
#[derive(Debug, Clone, clap::Args)]
pub struct Cmd {
    #[command(flatten)]
    pub subject: super::subject::AccountArg,

    /// Include the account's signer list, if it has one.
    ///
    /// The only RPC that answers "who can authorize this account now", which is
    /// what `account doctor` reconciles a local key record against.
    #[arg(long)]
    pub signer_lists: bool,

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
        let request = AccountInfo::builder(self.subject.address()?)
            .maybe_signer_lists(self.signer_lists.then_some(true))
            .maybe_ledger_hash(self.ledger.hash())
            .maybe_ledger_index(self.ledger.index())
            .build();

        let response = client.request(request.into()).map_err(Error::Client)?;
        output::response(
            &client::ok_result(&response)?,
            "Account info",
            self.output.json,
        )
    }
}
