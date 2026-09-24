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
/// The node's result, or its error.
///
/// A node that answers `lgrNotFound` has answered — the transport worked — so
/// nothing below the response layer fails, and [`result_value`] happily returns
/// the error body as though it were data. Every query command printed that and
/// exited 0, which told a script the opposite of what happened.
///
/// The pipeline stages and `xrpl rpc` keep using [`result_value`] directly:
/// showing exactly what the node said, error included, is what they are for.
pub fn ok_result(response: &xrpl::models::results::XRPLResponse<'_>) -> Result<Value, Error> {
    let value = result_value(response)?;

    // Both shapes, because rippled uses both. Over WebSocket the error is a
    // top-level field of the response; over JSON-RPC it is inside `result`,
    // which is where every command here finds it.
    let (code, message) = match &response.error {
        Some(code) => (
            code.to_string(),
            response.error_message.as_deref().unwrap_or("").to_string(),
        ),
        None => match value.get("error").and_then(Value::as_str) {
            Some(code) => (
                code.to_string(),
                value
                    .get("error_message")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string(),
            ),
            None => return Ok(value),
        },
    };

    Err(Error::Node {
        code,
        message: if message.is_empty() {
            "no message".to_string()
        } else {
            message
        },
    })
}

pub fn result_value(response: &xrpl::models::results::XRPLResponse<'_>) -> Result<Value, Error> {
    if let Some(raw) = &response.raw_result {
        return Ok(raw.clone());
    }

    match &response.result {
        Some(result) => Ok(serde_json::to_value(result)?),
        None => Ok(Value::Null),
    }
}
