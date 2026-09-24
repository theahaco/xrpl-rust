use xrpl::clients::XRPLSyncClient;
use xrpl::models::requests::account_tx::AccountTx;

use crate::commands::global::{LedgerArgs, NetworkArgs, OutputArgs, DEFAULT_PAGINATION_LIMIT};
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

    /// Only transactions at or after this ledger index.
    #[arg(long, value_name = "INDEX")]
    pub ledger_index_min: Option<u32>,

    /// Only transactions at or before this ledger index.
    #[arg(long, value_name = "INDEX")]
    pub ledger_index_max: Option<u32>,

    #[command(flatten)]
    pub ledger: LedgerArgs,

    #[command(flatten)]
    pub network: NetworkArgs,

    #[command(flatten)]
    pub output: OutputArgs,
}

impl Cmd {
    /// The request this command sends, split out so the flag-to-field mapping
    /// is testable without a node.
    fn request(&self, address: String) -> AccountTx<'_> {
        AccountTx::builder(address)
            .limit(self.limit)
            .maybe_ledger_index_min(self.ledger_index_min)
            .maybe_ledger_index_max(self.ledger_index_max)
            .maybe_ledger_hash(self.ledger.hash())
            .maybe_ledger_index(self.ledger.index())
            .build()
    }

    pub fn run(&self) -> Result<(), Error> {
        let client = client::json_rpc(&self.network.url_or_mainnet())?;

        let response = client
            .request(self.request(self.subject.address()?).into())
            .map_err(Error::Client)?;

        output::response(
            &client::ok_result(&response)?,
            "Account transactions",
            self.output.json,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::global::{LedgerArgs, NetworkArgs, OutputArgs};

    const ADDRESS: &str = "rHb9CJAWyB4rj91VRWn96DkukG4bwdtyTh";

    fn cmd() -> Cmd {
        Cmd {
            subject: super::super::subject::AccountArg {
                account: ADDRESS.into(),
            },
            limit: 25,
            ledger_index_min: None,
            ledger_index_max: None,
            ledger: LedgerArgs::default(),
            network: NetworkArgs {
                url: None,
                network: None,
            },
            output: OutputArgs { json: false },
        }
    }

    #[test]
    fn limit_sets_limit_not_a_ledger_bound() {
        let cmd = cmd();
        let request = cmd.request(ADDRESS.into());

        assert_eq!(request.limit, Some(25));
        assert_eq!(request.ledger_index_min, None);
    }

    #[test]
    fn the_range_flags_reach_the_request() {
        // They did not before: the request has carried these fields all along
        // and no flag set them, so this test asserted `None` and could assert
        // nothing else.
        let mut cmd = cmd();
        cmd.ledger_index_min = Some(100);
        cmd.ledger_index_max = Some(200);

        let request = cmd.request(ADDRESS.into());
        assert_eq!(request.ledger_index_min, Some(100));
        assert_eq!(request.ledger_index_max, Some(200));
    }

    #[test]
    fn a_ledger_shortcut_stays_a_string_and_a_number_becomes_one() {
        use xrpl::models::requests::LedgerIndex;

        let mut cmd = cmd();

        cmd.ledger.ledger_index = Some("validated".into());
        let request = cmd.request(ADDRESS.into());
        assert!(matches!(
            request
                .ledger_lookup
                .as_ref()
                .and_then(|l| l.ledger_index.as_ref()),
            Some(LedgerIndex::Str(_))
        ));
        drop(request);

        // rippled accepts both, and a caller who typed a number means a
        // sequence rather than a ledger named "600".
        cmd.ledger.ledger_index = Some("600".into());
        let request = cmd.request(ADDRESS.into());
        assert!(matches!(
            request
                .ledger_lookup
                .as_ref()
                .and_then(|l| l.ledger_index.as_ref()),
            Some(LedgerIndex::Int(600))
        ));
    }
}
