//! Request test XRP for an existing address, without accessing its keys.

use std::time::Duration;

use serde_json::{json, Value};
use url::Url;
use xrpl::asynch::clients::{AsyncJsonRpcClient, XRPLAsyncClient, XRPLFaucet};
use xrpl::models::requests::{generic_request::GenericRequest, FundFaucet};

use crate::commands::global::{Network, OutputArgs};
use crate::commands::tx::args::RequiredNetworkArgs;
use crate::error::Error;
use crate::store::{resolve, Store};
use crate::{client, output};

#[derive(Debug, Clone, clap::Args)]
pub struct Cmd {
    /// The existing account alias or classic r-address to fund.
    #[arg(value_name = "ALIAS_OR_ADDRESS")]
    pub account: String,

    #[command(flatten)]
    pub network: RequiredNetworkArgs,

    /// Override the faucet's HTTP endpoint (the full funding URL).
    #[arg(long, value_name = "URL")]
    pub faucet_url: Option<String>,

    /// Maximum seconds for the request and validated balance increase (1–3600).
    ///
    /// A timeout does not prove funding failed. Check the account before retrying.
    #[arg(long, default_value_t = 60, value_parser = clap::value_parser!(u64).range(1..=3600))]
    pub timeout: u64,

    #[command(flatten)]
    pub output: OutputArgs,
}

impl Cmd {
    pub fn run(&self) -> Result<(), Error> {
        let node_url = http_url(&self.network.url()?)?;
        if self.network.url.is_none() && matches!(self.network.network, Some(Network::Mainnet)) {
            return Err(Error::other(
                "Mainnet has no test-XRP faucet; use --network testnet or --network devnet",
            ));
        }

        let store = Store::from_env()?;
        let address = resolve::account(&store, Some(&self.account))?.address;

        let node = AsyncJsonRpcClient::connect(node_url);
        let faucet = match &self.faucet_url {
            Some(url) => http_url(url)?,
            None => node.get_faucet_url(None).map_err(|_| {
                Error::other(
                    "no faucet is known for this node; use --network testnet/devnet or \
                     supply --faucet-url with the full funding URL. Standalone nodes have \
                     no built-in faucet; fund them with a Payment from a funded account",
                )
            })?,
        };

        output::note(format!("requesting test XRP for {address}"));
        let result = client::runtime()?.block_on(async {
            tokio::time::timeout(
                Duration::from_secs(self.timeout),
                fund(&node, faucet, &address),
            )
            .await
            .map_err(|_| Error::FundingTimeout {
                address: address.clone(),
                seconds: self.timeout,
            })?
        })?;

        output::response(
            &result,
            "Account funded (validated balance increased)",
            self.output.json,
        )
    }
}

fn http_url(value: &str) -> Result<Url, Error> {
    let url = client::parse_url(value)?;
    if !matches!(url.scheme(), "http" | "https") || url.host_str().is_none() {
        return Err(Error::other("node and faucet URLs must use HTTP or HTTPS"));
    }
    Ok(url)
}

async fn fund(node: &AsyncJsonRpcClient, faucet: Url, address: &str) -> Result<Value, Error> {
    let before = balance(node, address).await?;
    let previous_balance = before.as_ref().map_or(0, |snapshot| snapshot.drops);

    // The recipient authorizes nothing. The library's faucet transport accepts
    // an address directly, so funding a watch-only account needs no Wallet.
    // Send exactly once: a lost HTTP response is not permission to fund twice.
    node.request_funding(
        Some(faucet),
        FundFaucet {
            destination: address.into(),
            usage_context: None,
            user_agent: Some("xrpl-cli".into()),
        },
    )
    .await
    .map_err(Error::Client)?;

    loop {
        if let Some(after) = balance(node, address).await? {
            let newer = before
                .as_ref()
                .is_none_or(|before| after.ledger_index > before.ledger_index);
            if newer && after.drops > previous_balance {
                return Ok(json!({
                    "account": address,
                    "previous_balance": previous_balance.to_string(),
                    "balance": after.drops.to_string(),
                    "ledger_index": after.ledger_index,
                    "validated": true,
                    "funded": true,
                }));
            }
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
}

struct Balance {
    drops: u64,
    ledger_index: u64,
}

async fn balance(node: &AsyncJsonRpcClient, address: &str) -> Result<Option<Balance>, Error> {
    let response = node
        .request(
            GenericRequest::builder("account_info")
                .params(
                    json!({"account": address, "ledger_index": "validated"})
                        .as_object()
                        .expect("object literal")
                        .clone(),
                )
                .build()
                .into(),
        )
        .await
        .map_err(Error::Client)?;
    let result = match client::ok_result(&response) {
        Err(Error::Node { code, .. }) if code == "actNotFound" => return Ok(None),
        other => other?,
    };
    let invalid = || Error::FundingResponse {
        address: address.to_string(),
    };
    if result["validated"] != true || result["account_data"]["Account"] != address {
        return Err(invalid());
    }
    let drops = result["account_data"]["Balance"]
        .as_str()
        .and_then(|balance| balance.parse::<u64>().ok())
        .ok_or_else(invalid)?;
    let ledger_index = result["ledger_index"].as_u64().ok_or_else(invalid)?;
    Ok(Some(Balance {
        drops,
        ledger_index,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn endpoints_require_http() {
        assert!(http_url("https://faucet.example/accounts").is_ok());
        assert!(http_url("http://127.0.0.1:5005").is_ok());
        assert!(http_url("ws://127.0.0.1:6006").is_err());
        assert!(http_url("file:///tmp/faucet").is_err());
    }
}
