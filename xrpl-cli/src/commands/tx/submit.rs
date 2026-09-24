//! `xrpl tx submit` — encode and send. The other network stage.

use std::time::{Duration, Instant};

use serde_json::Value;
use xrpl::asynch::clients::{AsyncJsonRpcClient, XRPLAsyncClient};
use xrpl::core::binarycodec::encode;
use xrpl::models::requests::generic_request::GenericRequest;

use crate::client;
use crate::commands::tx::args::{RequiredNetworkArgs, TxInput};
use crate::commands::tx::io;
use crate::error::{exit, Error};
use crate::output;

/// How long `--wait` polls before giving up.
///
/// A wall-clock bound as well as the ledger's own `LastLedgerSequence` one: a
/// node that has stopped advancing would otherwise hang the pipeline forever,
/// and "the CLI is stuck" is a much worse diagnosis than "the node is not
/// closing ledgers".
const WAIT_TIMEOUT: Duration = Duration::from_secs(60);

#[derive(Debug, Clone, clap::Args)]
pub struct Cmd {
    #[command(flatten)]
    pub input: TxInput,

    #[command(flatten)]
    pub network: RequiredNetworkArgs,

    /// Poll until the transaction is in a validated ledger.
    #[arg(long)]
    pub wait: bool,

    /// Close a ledger between polls, for a standalone node.
    ///
    /// A standalone node never closes one on its own, so `--wait` without this
    /// waits for something that will not happen.
    #[arg(long, requires = "wait")]
    pub accept_ledger: bool,
}

impl Cmd {
    pub fn run(&self) -> Result<(), Error> {
        let url = self.network.url()?;
        let transactions = io::read_txs(&self.input.tx)?;

        let runtime = client::runtime()?;
        runtime.block_on(async {
            let node = AsyncJsonRpcClient::connect(client::parse_url(&url)?);

            for transaction in &transactions {
                self.submit_one(transaction, &node).await?;
            }

            Ok::<(), Error>(())
        })
    }

    async fn submit_one(
        &self,
        transaction: &Value,
        node: &AsyncJsonRpcClient,
    ) -> Result<(), Error> {
        require_submittable(transaction)?;
        self.warn_if_the_destination_cannot_hold_it(transaction, node)
            .await;

        let blob = encode(transaction)?;
        let mut params = serde_json::Map::new();
        params.insert("tx_blob".into(), Value::String(blob));

        let response = node
            .request(
                GenericRequest::builder("submit")
                    .params(params)
                    .build()
                    .into(),
            )
            .await
            .map_err(Error::Client)?;

        let result = client::result_value(&response)?;
        let engine_result = result["engine_result"]
            .as_str()
            .unwrap_or_default()
            .to_string();

        if !self.wait {
            output::artifact(&result)?;
            return classify(&engine_result);
        }

        let hash = result["tx_json"]["hash"]
            .as_str()
            .ok_or_else(|| {
                Error::other(format!(
                    "node accepted the submission but reported no hash: {}",
                    result
                ))
            })?
            .to_string();

        let validated = self.poll_until_validated(node, &hash).await?;
        let result_code = validated["meta"]["TransactionResult"]
            .as_str()
            .unwrap_or(&engine_result)
            .to_string();

        output::artifact(&validated)?;
        classify(&result_code)
    }

    /// Warn before spending a ceremony on `tecNO_AUTH`.
    ///
    /// An MPT `Payment` to an account with no `MPToken` object for that issuance
    /// fails at the ledger. That is exit 3 and a burnt fee for a single-signed
    /// transaction, and a re-collected quorum for a multisigned one — the
    /// TypeScript project this CLI replaces hit exactly this, because its
    /// `redistribute` never authorized its destination.
    ///
    /// A warning, never a refusal, and never a failure of its own: the node is
    /// the authority on whether a transaction applies, and a submit that
    /// refused to run because a read failed would be worse than the `tec` it
    /// was trying to prevent.
    async fn warn_if_the_destination_cannot_hold_it(
        &self,
        transaction: &Value,
        node: &AsyncJsonRpcClient,
    ) {
        if transaction["TransactionType"] != "Payment" {
            return;
        }

        let Some(issuance) = transaction["Amount"]["mpt_issuance_id"].as_str() else {
            return;
        };
        let Some(destination) = transaction["Destination"].as_str() else {
            return;
        };

        if holds_mpt(node, destination, issuance).await
            || is_the_issuer(node, destination, issuance).await
        {
            return;
        }

        output::warn(format!(
            "{destination} holds no MPToken for {issuance}. \
             This will fail with tecNO_AUTH unless it is authorized first: \
             `xrpl tx new MPTokenAuthorize --account {destination} \
             --mptoken-issuance-id {issuance}`, signed by {destination}."
        ));
    }

    async fn poll_until_validated(
        &self,
        node: &AsyncJsonRpcClient,
        hash: &str,
    ) -> Result<Value, Error> {
        let started = Instant::now();

        loop {
            if self.accept_ledger {
                let _ = node
                    .request(
                        GenericRequest::builder("ledger_accept")
                            .params(serde_json::Map::new())
                            .build()
                            .into(),
                    )
                    .await;
            }

            let mut params = serde_json::Map::new();
            params.insert("transaction".into(), Value::String(hash.to_string()));

            if let Ok(response) = node
                .request(GenericRequest::builder("tx").params(params).build().into())
                .await
            {
                let result = client::result_value(&response)?;
                if result["validated"].as_bool() == Some(true) {
                    return Ok(result);
                }
            }

            if started.elapsed() >= WAIT_TIMEOUT {
                return Err(Error::other(format!(
                    "gave up waiting for {hash} to validate after {WAIT_TIMEOUT:?}. \
                     On a standalone node, pass --accept-ledger."
                )));
            }

            tokio::time::sleep(Duration::from_millis(500)).await;
        }
    }
}

/// Whether an account already has an `MPToken` for this issuance.
///
/// Read against the **current** ledger rather than the validated one: a
/// destination authorized moments ago by an earlier stage of the same script is
/// not validated yet, and warning about it would be a false alarm people learn
/// to skip.
///
/// Every failure answers "yes" — an unreachable node, an unsupported method, a
/// malformed response. The cost of a missed warning is a `tec` the caller sees
/// anyway; the cost of a false one is a warning nobody reads.
async fn holds_mpt(node: &AsyncJsonRpcClient, account: &str, issuance: &str) -> bool {
    let mut params = serde_json::Map::new();
    params.insert("account".into(), Value::String(account.to_string()));
    params.insert("type".into(), Value::String("mptoken".into()));
    params.insert("ledger_index".into(), Value::String("current".into()));

    let Ok(response) = node
        .request(
            GenericRequest::builder("account_objects")
                .params(params)
                .build()
                .into(),
        )
        .await
    else {
        return true;
    };

    let Ok(result) = client::result_value(&response) else {
        return true;
    };

    match result["account_objects"].as_array() {
        Some(objects) => objects
            .iter()
            .any(|object| object["MPTokenIssuanceID"] == issuance),
        None => true,
    }
}

/// Whether this account issued the MPT.
///
/// An issuer holds no `MPToken` for its own issuance and never needs one, so
/// redeeming back to it is not the mistake this warning is about.
async fn is_the_issuer(node: &AsyncJsonRpcClient, account: &str, issuance: &str) -> bool {
    let mut params = serde_json::Map::new();
    params.insert("mpt_issuance".into(), Value::String(issuance.to_string()));
    params.insert("ledger_index".into(), Value::String("current".into()));

    let Ok(response) = node
        .request(
            GenericRequest::builder("ledger_entry")
                .params(params)
                .build()
                .into(),
        )
        .await
    else {
        return false;
    };

    client::result_value(&response)
        .map(|result| result["node"]["Issuer"] == account)
        .unwrap_or(false)
}

/// Refuse a transaction the node would only reject.
///
/// Catching this here rather than at the node also catches a hand-assembled
/// transaction that never passed through a signing stage.
fn require_submittable(transaction: &Value) -> Result<(), Error> {
    let mut missing = Vec::new();
    for field in ["Fee", "Sequence"] {
        if transaction.get(field).is_none() {
            missing.push(field);
        }
    }

    // A transaction using a ticket has no `Sequence`, and that is legitimate.
    if missing == ["Sequence"] && transaction.get("TicketSequence").is_some() {
        missing.clear();
    }

    if !missing.is_empty() {
        return Err(Error::other(format!(
            "missing {}: run `xrpl tx autofill` before signing",
            missing.join(" and ")
        )));
    }

    Ok(())
}

/// Map a ledger result code onto this command's exit status.
///
/// `tes` is success. Everything else reached a ledger and failed there, which is
/// exit 3 — a different thing from a bad flag and from an unreachable node, and
/// a retry wrapper needs to tell them apart.
fn classify(result_code: &str) -> Result<(), Error> {
    if result_code.starts_with("tes") {
        return Ok(());
    }

    let error = Error::Helper(
        xrpl::asynch::exceptions::XRPLHelperException::XRPLTransactionHelperError(
            xrpl::asynch::transaction::exceptions::XRPLTransactionHelperException::XRPLSubmitAndWaitError(
                xrpl::asynch::transaction::exceptions::XRPLSubmitAndWaitException::SubmissionFailed {
                    result_code: result_code.to_string(),
                    message: Some(format!("transaction failed with {result_code}")),
                },
            ),
        ),
    );

    debug_assert_eq!(
        format!("{:?}", error.exit_code()),
        format!("{:?}", std::process::ExitCode::from(exit::LEDGER)),
        "a ledger-level failure must exit 3"
    );

    Err(error)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_a_transaction_without_a_fee_is_refused_before_the_network() {
        let tx = json!({"TransactionType": "Payment", "Sequence": 1});
        let message = require_submittable(&tx).unwrap_err().to_string();

        assert!(message.contains("missing Fee"), "{message}");
        assert!(message.contains("tx autofill"), "{message}");
    }

    #[test]
    fn test_a_ticketed_transaction_needs_no_sequence() {
        let tx = json!({"TransactionType": "Payment", "Fee": "12", "TicketSequence": 7});
        assert!(require_submittable(&tx).is_ok());
    }

    #[test]
    fn test_tes_is_success_and_tec_is_not() {
        assert!(classify("tesSUCCESS").is_ok());
        assert!(classify("tecUNFUNDED_PAYMENT").is_err());
        assert!(classify("tefBAD_AUTH").is_err());
    }
}
