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

/// `--json`, flattened into every query command.
///
/// Not on the `tx` stages or `xrpl rpc`: those have exactly one output shape
/// and no flag at all, because a configurable pipe format is how chaining
/// breaks — every downstream consumer then has to sniff what it is reading.
/// A query command has a person at the other end often enough to be worth the
/// two shapes.
#[derive(Debug, Clone, Copy, clap::Args)]
pub struct OutputArgs {
    /// Print one line of compact JSON instead of an indented document.
    #[arg(long)]
    pub json: bool,
}

/// `--ledger-index` / `--ledger-hash`, flattened into every query that can name
/// a ledger.
///
/// Every one of these requests supports them and none of them exposed one, so
/// a query could only ever read whatever the node felt was current. That is
/// wrong for the reconciliation `account doctor` does and wrong for anyone
/// reproducing a result: "the balance at ledger 96,000,000" was unaskable.
#[derive(Debug, Clone, Default, clap::Args)]
pub struct LedgerArgs {
    /// The ledger to read: a sequence number, or validated|closed|current.
    #[arg(long, value_name = "INDEX_OR_SHORTCUT")]
    pub ledger_index: Option<String>,

    /// The ledger to read, by its hash.
    #[arg(long, value_name = "HASH", conflicts_with = "ledger_index")]
    pub ledger_hash: Option<String>,
}

impl LedgerArgs {
    /// The ledger index in the shape the request models want.
    ///
    /// An integer when it parses as one, the string otherwise — rippled accepts
    /// both, and `validated` is the one most callers actually want.
    pub fn index(&self) -> Option<xrpl::models::requests::LedgerIndex<'_>> {
        use xrpl::models::requests::LedgerIndex;

        self.ledger_index
            .as_deref()
            .map(|value| match value.parse() {
                Ok(number) => LedgerIndex::Int(number),
                Err(_) => LedgerIndex::Str(value.into()),
            })
    }

    /// The ledger hash, if one was given.
    pub fn hash(&self) -> Option<&str> {
        self.ledger_hash.as_deref()
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
    #[arg(long, value_enum, env = "XRPL_NETWORK")]
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
