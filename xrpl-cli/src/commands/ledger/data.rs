use xrpl::clients::XRPLSyncClient;
use xrpl::models::requests::ledger_data::LedgerData;

use crate::commands::global::{NetworkArgs, DEFAULT_PAGINATION_LIMIT};
use crate::error::Error;
use crate::{client, output};

/// Fetch the contents of a ledger.
#[derive(Debug, Clone, clap::Args)]
pub struct Cmd {
    /// Ledger index (empty for latest)
    #[arg(long)]
    pub ledger_index: Option<String>,

    /// Ledger hash (empty for latest)
    #[arg(long)]
    pub ledger_hash: Option<String>,

    /// Limit the number of objects returned
    #[arg(long, default_value_t = DEFAULT_PAGINATION_LIMIT as u16)]
    pub limit: u16,

    #[command(flatten)]
    pub network: NetworkArgs,
}

impl Cmd {
    /// The request this command sends, split out so the flag-to-field mapping
    /// is testable without a node.
    fn request(&self) -> LedgerData<'_> {
        LedgerData::builder()
            .maybe_ledger_hash(self.ledger_hash.as_deref())
            .maybe_ledger_index(self.ledger_index.as_deref())
            .limit(self.limit)
            .build()
    }

    pub fn run(&self) -> Result<(), Error> {
        let client = client::json_rpc(&self.network.url_or_mainnet())?;

        output::response(client.request(self.request().into()), "Ledger data")
    }
}

#[cfg(test)]
mod tests {
    use xrpl::models::requests::LedgerIndex;

    use super::*;
    use crate::commands::global::NetworkArgs;

    #[test]
    fn index_and_hash_land_in_their_own_fields() {
        let cmd = Cmd {
            ledger_index: Some("validated".into()),
            ledger_hash: Some("FEED".into()),
            limit: 5,
            network: NetworkArgs {
                url: None,
                network: None,
            },
        };

        let lookup = cmd.request().ledger_lookup.expect("lookup set");
        assert_eq!(lookup.ledger_hash.as_deref(), Some("FEED"));
        assert_eq!(
            lookup.ledger_index,
            Some(LedgerIndex::Str("validated".into()))
        );
    }
}
