//! Arguments shared by the pipeline stages.

use std::ffi::OsString;

/// The `[TX]` positional, written once so every stage documents it identically.
#[derive(Debug, Clone, clap::Args)]
pub struct TxInput {
    /// Transaction JSON, a file containing it, or `-`/empty for stdin.
    #[arg(value_name = "TX")]
    pub tx: Option<OsString>,
}

/// `--url` / `--network` for the two stages that touch the network.
///
/// Separate from the query commands' [`NetworkArgs`](crate::commands::global::NetworkArgs)
/// in one respect: **there is no mainnet default here.** `url_or_mainnet()`
/// falls back to `https://xrplcluster.com/`, so a bare `xrpl tx submit` would
/// otherwise reach mainnet with whatever it was handed. A pipeline stage that
/// silently picks a network is a pipeline stage that can spend real XRP.
#[derive(Debug, Clone, clap::Args)]
pub struct RequiredNetworkArgs {
    /// The XRPL node URL. Takes precedence over --network.
    #[arg(short = 'u', long)]
    pub url: Option<String>,

    /// A named network to use instead of spelling out --url.
    #[arg(long, value_enum)]
    pub network: Option<crate::commands::global::Network>,
}

impl RequiredNetworkArgs {
    /// The endpoint, or an error naming what is missing.
    pub fn url(&self) -> Result<String, crate::error::Error> {
        match (&self.url, self.network) {
            (Some(url), _) => Ok(url.clone()),
            (None, Some(network)) => Ok(network.json_rpc_url().to_string()),
            (None, None) => Err(crate::error::Error::other(
                "no network: pass --url or --network. \
                 This stage has no default, deliberately — a pipeline that picks \
                 a network on your behalf can pick mainnet.",
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::global::Network;

    #[test]
    fn test_no_network_is_an_error_not_a_mainnet_default() {
        let args = RequiredNetworkArgs {
            url: None,
            network: None,
        };

        let message = args.url().unwrap_err().to_string();
        assert!(message.contains("no network"), "{message}");
        // The point of the error: it must not quietly resolve to mainnet.
        assert!(!message.contains("xrplcluster"), "{message}");
    }

    #[test]
    fn test_url_wins_over_network() {
        let args = RequiredNetworkArgs {
            url: Some("http://127.0.0.1:5005".into()),
            network: Some(Network::Mainnet),
        };

        assert_eq!(args.url().expect("resolves"), "http://127.0.0.1:5005");
    }

    #[test]
    fn test_a_named_network_resolves() {
        let args = RequiredNetworkArgs {
            url: None,
            network: Some(Network::Local),
        };

        assert_eq!(args.url().expect("resolves"), "http://127.0.0.1:5005");
    }
}
