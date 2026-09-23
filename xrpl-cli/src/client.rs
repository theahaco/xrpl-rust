//! Client construction shared by every command that talks to a node.

use serde_json::Value;
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

/// The node's `result` object, exactly as it arrived.
///
/// `XRPLResponse::result` is a typed, untagged enum: useful when the library
/// models the RPC, lossy when it does not, and shaped differently per variant.
/// The pipeline and `xrpl rpc` want the node's own JSON, so they read
/// `raw_result` — which the deserializer preserves for exactly this reason —
/// and fall back to re-serializing the typed value only if it is missing.
pub fn result_value(response: &xrpl::models::results::XRPLResponse<'_>) -> Result<Value, Error> {
    if let Some(raw) = &response.raw_result {
        return Ok(raw.clone());
    }

    match &response.result {
        Some(result) => Ok(serde_json::to_value(result)?),
        None => Ok(Value::Null),
    }
}
