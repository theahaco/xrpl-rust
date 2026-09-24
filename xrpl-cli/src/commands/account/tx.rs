use xrpl::clients::XRPLSyncClient;
use xrpl::models::requests::account_tx::AccountTx;

use crate::commands::global::{NetworkArgs, DEFAULT_PAGINATION_LIMIT};
use crate::error::Error;
use crate::{client, output};

/// List an account's transactions.
#[derive(Debug, Clone, clap::Args)]
pub struct Cmd {
    #[command(flatten)]
    pub subject: super::subject::AccountArg,

    /// Limit the number of transactions returned
    #[arg(long, default_value_t = DEFAULT_PAGINATION_LIMIT as u16)]
    pub limit: u16,

    #[command(flatten)]
    pub network: NetworkArgs,
}

impl Cmd {
    /// The request this command sends, split out so the flag-to-field mapping
    /// is testable without a node.
    fn request(&self, address: String) -> AccountTx<'static> {
        AccountTx::builder(address).limit(self.limit).build()
    }

    pub fn run(&self) -> Result<(), Error> {
        let client = client::json_rpc(&self.network.url_or_mainnet())?;

        output::response(
            client.request(self.request(self.subject.address()?).into()),
            "Account transactions",
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::global::NetworkArgs;

    #[test]
    fn limit_sets_limit_not_a_ledger_bound() {
        let cmd = Cmd {
            subject: super::super::subject::AccountArg {
                account: "rHb9CJAWyB4rj91VRWn96DkukG4bwdtyTh".into(),
            },
            limit: 25,
            network: NetworkArgs {
                url: None,
                network: None,
            },
        };

        let request = cmd.request("rHb9CJAWyB4rj91VRWn96DkukG4bwdtyTh".into());
        assert_eq!(request.limit, Some(25));
        assert_eq!(request.ledger_index_min, None);
    }
}
