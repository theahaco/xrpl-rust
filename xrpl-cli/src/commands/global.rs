//! Arguments shared by every command that talks to a node.

/// Default JSON-RPC endpoint when neither `--url` nor `--network` is given.
pub const DEFAULT_MAINNET_URL: &str = "https://xrplcluster.com/";
/// Default JSON-RPC endpoint for `wallet faucet`.
pub const DEFAULT_TESTNET_URL: &str = "https://s.altnet.rippletest.net:51234";
/// Default WebSocket endpoint for streaming commands.
pub const DEFAULT_WEBSOCKET_URL: &str = "wss://xrplcluster.com/";
/// Default page size for paginated queries.
pub const DEFAULT_PAGINATION_LIMIT: u32 = 10;

/// A named network, so callers don't have to remember endpoints and ports.
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum Network {
    /// Public mainnet.
    Mainnet,
    /// Public testnet.
    Testnet,
    /// Public devnet.
    Devnet,
    /// A standalone node on this machine (rippled's admin ports).
    Local,
}

impl Network {
    /// The HTTP JSON-RPC endpoint for this network.
    pub fn json_rpc_url(self) -> &'static str {
        match self {
            Network::Mainnet => DEFAULT_MAINNET_URL,
            Network::Testnet => DEFAULT_TESTNET_URL,
            Network::Devnet => "https://s.devnet.rippletest.net:51234",
            Network::Local => "http://127.0.0.1:5005",
        }
    }

    /// The WebSocket endpoint for this network.
    pub fn websocket_url(self) -> &'static str {
        match self {
            Network::Mainnet => DEFAULT_WEBSOCKET_URL,
            Network::Testnet => "wss://s.altnet.rippletest.net:51233",
            Network::Devnet => "wss://s.devnet.rippletest.net:51233",
            Network::Local => "ws://127.0.0.1:6006",
        }
    }
}

/// `--url` / `--network`, flattened into every command that reaches a node.
///
/// `--url` wins when both are given; a command that names no endpoint falls
/// back to the default it passes to [`NetworkArgs::json_rpc_url`], which keeps
/// per-command defaults (mainnet for queries, testnet for the faucet) intact.
#[derive(Debug, Clone, clap::Args)]
pub struct NetworkArgs {
    /// The XRPL node URL. Takes precedence over --network.
    #[arg(short = 'u', long)]
    pub url: Option<String>,

    /// A named network to use instead of spelling out --url.
    #[arg(long, value_enum)]
    pub network: Option<Network>,
}

impl NetworkArgs {
    /// Resolve the JSON-RPC endpoint, falling back to `default` when the user
    /// named neither a URL nor a network.
    pub fn json_rpc_url(&self, default: &str) -> String {
        match (&self.url, self.network) {
            (Some(url), _) => url.clone(),
            (None, Some(network)) => network.json_rpc_url().to_string(),
            (None, None) => default.to_string(),
        }
    }

    /// Resolve the WebSocket endpoint under the same rules.
    pub fn websocket_url(&self, default: &str) -> String {
        match (&self.url, self.network) {
            (Some(url), _) => url.clone(),
            (None, Some(network)) => network.websocket_url().to_string(),
            (None, None) => default.to_string(),
        }
    }

    /// The mainnet-defaulting JSON-RPC endpoint most query commands want.
    pub fn url_or_mainnet(&self) -> String {
        self.json_rpc_url(DEFAULT_MAINNET_URL)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(url: Option<&str>, network: Option<Network>) -> NetworkArgs {
        NetworkArgs {
            url: url.map(str::to_string),
            network,
        }
    }

    #[test]
    fn url_wins_over_network() {
        let args = args(Some("http://127.0.0.1:5005"), Some(Network::Mainnet));
        assert_eq!(args.url_or_mainnet(), "http://127.0.0.1:5005");
    }

    #[test]
    fn network_resolves_per_protocol() {
        let args = args(None, Some(Network::Local));
        assert_eq!(args.url_or_mainnet(), "http://127.0.0.1:5005");
        assert_eq!(
            args.websocket_url(DEFAULT_WEBSOCKET_URL),
            "ws://127.0.0.1:6006"
        );
    }

    #[test]
    fn falls_back_to_the_command_default() {
        let args = args(None, None);
        assert_eq!(args.url_or_mainnet(), DEFAULT_MAINNET_URL);
        assert_eq!(args.json_rpc_url(DEFAULT_TESTNET_URL), DEFAULT_TESTNET_URL);
    }
}
