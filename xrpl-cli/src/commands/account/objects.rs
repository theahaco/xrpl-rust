use std::str::FromStr;

use xrpl::clients::XRPLSyncClient;
use xrpl::models::requests::account_objects::{AccountObjectType, AccountObjects};

use crate::commands::global::{NetworkArgs, DEFAULT_PAGINATION_LIMIT};
use crate::error::Error;
use crate::{client, output};

/// List an account's ledger objects.
#[derive(Debug, Clone, clap::Args)]
pub struct Cmd {
    #[command(flatten)]
    pub subject: super::subject::AccountArg,

    /// Type of objects to return (all, offer, state, etc.)
    #[arg(long)]
    pub type_filter: Option<String>,

    /// Limit the number of objects returned
    #[arg(long, default_value_t = DEFAULT_PAGINATION_LIMIT as u16)]
    pub limit: u16,

    #[command(flatten)]
    pub network: NetworkArgs,
}

impl Cmd {
    pub fn run(&self) -> Result<(), Error> {
        let object_type = match self.type_filter.as_deref() {
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
            .build();

        output::response(client.request(request.into()), "Account objects")
    }
}
