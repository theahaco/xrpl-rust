//! Client construction shared by every command that talks to a node.

use url::Url;
use xrpl::clients::json_rpc::JsonRpcClient;

use crate::error::Error;

/// Parse a URL, mapping the parse failure onto [`Error`].
pub fn parse_url(url: &str) -> Result<Url, Error> {
    url.parse().map_err(Error::UrlParse)
}

/// Connect a synchronous JSON-RPC client to `url`.
pub fn json_rpc(url: &str) -> Result<JsonRpcClient, Error> {
    Ok(JsonRpcClient::connect(parse_url(url)?))
}

/// A multi-threaded Tokio runtime for the commands that drive async helpers.
pub fn runtime() -> Result<tokio::runtime::Runtime, Error> {
    tokio::runtime::Runtime::new().map_err(Error::Io)
}
