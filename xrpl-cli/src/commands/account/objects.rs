use std::str::FromStr;

use xrpl::clients::XRPLSyncClient;
use xrpl::models::requests::account_objects::{AccountObjectType, AccountObjects};

use crate::commands::global::{LedgerArgs, NetworkArgs, OutputArgs, DEFAULT_PAGINATION_LIMIT};
use crate::error::Error;
use crate::{client, output};

/// List an account's ledger objects.
#[derive(Debug, Clone, clap::Args)]
pub struct Cmd {
    #[command(flatten)]
    pub subject: super::subject::AccountArg,

    /// Type of objects to return (all, offer, state, etc.)
    ///
    /// Named for the field it sets. `--type-filter` still works — it is what
    /// this flag was called — but it is hidden, because two spellings in the
    /// help text is how a reader learns to distrust both.
    #[arg(long = "type", alias = "type-filter", value_name = "TYPE")]
    pub object_type: Option<String>,

    /// Limit the number of objects returned
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
        let object_type = match self.object_type.as_deref() {
            Some(filter) => Some(
                AccountObjectType::from_str(filter)
                    .map_err(|_| Error::other(format!("Invalid object type: {filter}")))?,
            ),
            None => None,
        };

        let client = client::json_rpc(&self.network.url_or_mainnet())?;
        let request = AccountObjects::builder(self.subject.address()?)
            .maybe_type(object_type)
            .limit(self.limit)
            .maybe_ledger_hash(self.ledger.hash())
            .maybe_ledger_index(self.ledger.index())
            .build();

        let response = client.request(request.into()).map_err(Error::Client)?;
        output::response(
            &client::ok_result(&response)?,
            "Account objects",
            self.output.json,
        )
    }
}
