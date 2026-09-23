//! `xrpl rpc <command> [--param k=v]…` — call any rippled RPC.
//!
//! The request-side twin of the transaction pipeline, making the same bet:
//! rippled's own definitions are the contract, not this crate's Rust structs.
//! The library already ships `GenericRequest` as the escape hatch for RPCs it
//! has not caught up with; this is ten lines over it.
//!
//! It exists because a standalone node never closes a ledger on its own, so
//! anything that waits for validation hangs until `LastLedgerSequence` passes
//! unless something can issue `ledger_accept`. Nothing could.
//!
//! It is deliberately outside the identity surface: it never signs, so a
//! `--param account=r…` is a raw RPC parameter with no alias resolution and no
//! seed rejection. Its output is JSON unconditionally and it must not acquire a
//! `--json` flag.

use serde_json::{Map, Value};
use xrpl::asynch::clients::AsyncJsonRpcClient;
use xrpl::asynch::clients::XRPLAsyncClient;
use xrpl::models::requests::generic_request::GenericRequest;

use crate::client;
use crate::commands::global::NetworkArgs;
use crate::error::Error;
use crate::output;

#[derive(Debug, Clone, clap::Args)]
pub struct Cmd {
    /// The rippled RPC command name, e.g. `server_info` or `ledger_accept`.
    pub command: String,

    /// A request parameter, as `key=value`. Repeatable.
    ///
    /// The value is parsed as JSON when it parses as JSON, and treated as a
    /// string otherwise — so `--param ledger_index=validated` and
    /// `--param mptoken='{"account":"r…"}'` both do the right thing.
    #[arg(long = "param", value_name = "KEY=VALUE")]
    pub params: Vec<String>,

    #[command(flatten)]
    pub network: NetworkArgs,
}

impl Cmd {
    pub fn run(&self) -> Result<(), Error> {
        let params = parse_params(&self.params)?;
        let url = self.network.url_or_mainnet();

        // Async client inside one runtime per invocation. The sync client builds
        // its own runtime per request and panics when called from inside one.
        let runtime = client::runtime()?;
        let response = runtime.block_on(async {
            let client = AsyncJsonRpcClient::connect(client::parse_url(&url)?);
            let request = GenericRequest::builder(self.command.clone())
                .params(params)
                .build();

            client.request(request.into()).await.map_err(Error::Client)
        })?;

        // The node's result, and nothing else, on stdout.
        output::artifact(&client::result_value(&response)?)
    }
}

/// Parse repeated `key=value` arguments into an RPC parameter map.
///
/// A value that parses as JSON is used as JSON; anything else is a string. That
/// keeps `ledger_index=validated` a string while letting a caller pass a nested
/// object or a number without quoting gymnastics.
fn parse_params(raw: &[String]) -> Result<Map<String, Value>, Error> {
    let mut params = Map::new();

    for entry in raw {
        let (key, value) = entry.split_once('=').ok_or_else(|| {
            Error::other(format!(
                "--param expects KEY=VALUE, got {entry:?} with no `=`"
            ))
        })?;

        if key.is_empty() {
            return Err(Error::other(format!("--param has an empty key: {entry:?}")));
        }

        // `command` and `id` are the two keys the request serializer emits
        // itself. Letting a caller set them would silently redirect the RPC.
        if key == "command" || key == "id" {
            return Err(Error::other(format!(
                "--param {key} is reserved; the command name is the positional argument"
            )));
        }

        let parsed = serde_json::from_str::<Value>(value)
            .unwrap_or_else(|_| Value::String(value.to_string()));
        params.insert(key.to_string(), parsed);
    }

    Ok(params)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn params(entries: &[&str]) -> Map<String, Value> {
        let owned: Vec<String> = entries.iter().map(|e| e.to_string()).collect();
        parse_params(&owned).expect("parses")
    }

    #[test]
    fn test_a_bare_word_stays_a_string() {
        assert_eq!(
            params(&["ledger_index=validated"])["ledger_index"],
            json!("validated")
        );
    }

    #[test]
    fn test_json_values_are_parsed_as_json() {
        let parsed = params(&[
            "binary=true",
            "limit=10",
            r#"mptoken={"account":"rAlice","mpt_issuance_id":"AB"}"#,
        ]);

        assert_eq!(parsed["binary"], json!(true));
        assert_eq!(parsed["limit"], json!(10));
        assert_eq!(
            parsed["mptoken"],
            json!({"account": "rAlice", "mpt_issuance_id": "AB"})
        );
    }

    #[test]
    fn test_a_value_containing_equals_is_kept_whole() {
        // Splitting on the first `=` only; a base64 or hex value with padding
        // must survive.
        assert_eq!(params(&["blob=aGVsbG8="])["blob"], json!("aGVsbG8="));
    }

    #[test]
    fn test_a_48_hex_issuance_id_stays_a_string() {
        // Hex that happens to be all digits must not become a number and lose
        // its leading zeros.
        let id = "00000123456789ABCDEF0123456789ABCDEF0123456789AB";
        assert_eq!(
            params(&[&format!("mpt_issuance={id}")])["mpt_issuance"],
            json!(id)
        );
    }

    #[test]
    fn test_a_param_without_equals_is_a_usage_error() {
        let owned = vec!["justakey".to_string()];
        assert!(parse_params(&owned).is_err());
    }

    #[test]
    fn test_reserved_keys_are_refused() {
        for reserved in ["command=stop", "id=attacker"] {
            let owned = vec![reserved.to_string()];
            assert!(
                parse_params(&owned).is_err(),
                "{reserved} should be refused"
            );
        }
    }

    #[test]
    fn test_an_empty_key_is_a_usage_error() {
        let owned = vec!["=value".to_string()];
        assert!(parse_params(&owned).is_err());
    }
}
