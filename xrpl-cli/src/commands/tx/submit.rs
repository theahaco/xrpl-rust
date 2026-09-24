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

    /// Abandon the rest of the stream at the first failure.
    ///
    /// Without it every line is submitted and reported, and the command exits
    /// with the worst outcome it saw — which is what a funding stream wants,
    /// since a later line rarely depends on an earlier one. With it, a stream
    /// whose lines *do* depend on each other stops rather than piling failures
    /// on top of the one that mattered.
    ///
    /// Either way the lines already submitted stay submitted. Nothing here
    /// rolls back; that is what `Batch` is for.
    #[arg(long)]
    pub stop_on_error: bool,
}

impl Cmd {
    pub fn run(&self) -> Result<(), Error> {
        let url = self.network.url()?;
        let transactions = io::read_txs(&self.input.tx)?;

        let runtime = client::runtime()?;
        runtime.block_on(async {
            let node = AsyncJsonRpcClient::connect(client::parse_url(&url)?);

            // The worst outcome seen, so a stream reports every line and still
            // exits with something a retry wrapper can read.
            let mut worst: Option<Error> = None;

            for (index, transaction) in transactions.iter().enumerate() {
                match self.submit_one(transaction, &node).await {
                    Ok(()) => {}
                    Err(error) => {
                        if self.stop_on_error || transactions.len() == 1 {
                            return Err(error);
                        }

                        output::warn(format!("line {}: {error}", index + 1));
                        // A ledger-level failure outranks a usage one here: it
                        // means something landed, which is the outcome a caller
                        // most needs to know about.
                        if worst.is_none() || matches!(error, Error::Helper(_)) {
                            worst = Some(error);
                        }
                    }
                }
            }

            match worst {
                Some(error) => Err(error),
                None => Ok(()),
            }
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
            // An inner transaction's result does not exist until the batch is
            // in a validated ledger, so there is nothing to report yet — and
            // the outer's `tesSUCCESS` says nothing about whether the inners
            // applied. Silence here would read as "all of them worked".
            if transaction["TransactionType"] == "Batch" {
                output::warn(
                    "this is a Batch and --wait was not given, so its inner transactions are \
                     not reported. The outer result is tesSUCCESS whether or not they applied.",
                );
            }

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

        // A `Batch` reports `tesSUCCESS` whether or not its inner transactions
        // applied, so the outer result alone is not an answer.
        let inner = self.report_inner_results(transaction, node).await?;

        classify(&result_code)?;
        inner
    }

    /// Report each inner transaction of a `Batch`, and say whether any failed.
    ///
    /// **This is the one place `tx submit` writes something other than the
    /// transaction it was handed to stdout**: one NDJSON line per inner,
    /// `{"index":…,"hash":…,"result":…}`. It has to go to stdout, because it is
    /// the only machine-readable record that the batch landed and one leg
    /// failed — and that is a different outcome from "nothing happened", which
    /// is the distinction a retry wrapper exists to make.
    ///
    /// Inner transactions apply as separate transactions in the same ledger,
    /// each with its own metadata, so each is looked up by the ID the outer
    /// committed to.
    async fn report_inner_results(
        &self,
        transaction: &Value,
        node: &AsyncJsonRpcClient,
    ) -> Result<Result<(), Error>, Error> {
        if transaction["TransactionType"] != "Batch" {
            return Ok(Ok(()));
        }

        let Some(raw) = transaction["RawTransactions"].as_array() else {
            return Ok(Ok(()));
        };

        let mut failures = Vec::new();

        for (index, entry) in raw.iter().enumerate() {
            let inner = &entry["RawTransaction"];
            let hash = crate::commands::tx::hash::raw_transaction_id(inner)?;

            let result = self
                .inner_result(node, &hash)
                .await
                .unwrap_or_else(|| "unknown".to_string());

            output::artifact(&serde_json::json!({
                "index": index,
                "hash": hash,
                "result": result,
            }))?;

            if !result.starts_with("tes") {
                failures.push((index, result));
            }
        }

        if failures.is_empty() {
            return Ok(Ok(()));
        }

        output::warn(format!(
            "the batch applied and {} inner transaction(s) failed ({}). \
             Exit 3 here does NOT mean nothing happened, so this is not safe to resubmit.",
            failures.len(),
            failures
                .iter()
                .map(|(index, result)| format!("inner {index}: {result}"))
                .collect::<Vec<_>>()
                .join(", ")
        ));

        // The first failure decides the message. Every one of them is exit 3,
        // but naming one keeps the error specific.
        Ok(classify(&failures[0].1))
    }

    /// One inner transaction's result, or `None` if the node cannot say.
    async fn inner_result(&self, node: &AsyncJsonRpcClient, hash: &str) -> Option<String> {
        let mut params = serde_json::Map::new();
        params.insert("transaction".into(), Value::String(hash.to_string()));

        let response = node
            .request(GenericRequest::builder("tx").params(params).build().into())
            .await
            .ok()?;

        client::result_value(&response).ok()?["meta"]["TransactionResult"]
            .as_str()
            .map(str::to_string)
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
    // Checked before the fields, because the diagnosis is otherwise the node's
    // and it is a bad one: rippled decides single- versus multi-signing purely
    // from `SigningPubKey` being empty, so an unsigned transaction takes the
    // multi-sign path, finds no `Signers`, and comes back `temBAD_SIGNATURE:
    // Malformed: Bad signature.` — which reads like the signature is wrong
    // rather than absent.
    let signed = transaction.get("TxnSignature").is_some()
        || transaction
            .get("Signers")
            .and_then(Value::as_array)
            .is_some_and(|signers| !signers.is_empty());

    if !signed {
        return Err(Error::other(
            "unsigned: run `xrpl tx sign` before submitting. Submitting as-is makes the node \
             read the empty SigningPubKey as a multisign attempt and answer temBAD_SIGNATURE, \
             which says the signature is bad rather than missing.",
        ));
    }

    let mut missing = Vec::new();
    for field in ["Fee", "Sequence"] {
        if transaction.get(field).is_none() {
            missing.push(field);
        }
    }

    // A ticketed transaction still carries `Sequence`, set to 0 — the field is
    // mandatory and zero is what says "a ticket authorizes this". rippled
    // answers `invalidTransaction: Field 'Sequence' is required but missing`
    // otherwise, so tolerating its absence here only moved the rejection to
    // the node.

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

    /// A signed transaction, so the tests below reach the field checks.
    fn signed(mut tx: Value) -> Value {
        tx["TxnSignature"] = json!("3045…");
        tx
    }

    #[test]
    fn test_a_transaction_without_a_fee_is_refused_before_the_network() {
        let tx = signed(json!({"TransactionType": "Payment", "Sequence": 1}));
        let message = require_submittable(&tx).unwrap_err().to_string();

        assert!(message.contains("missing Fee"), "{message}");
        assert!(message.contains("tx autofill"), "{message}");
    }

    #[test]
    fn test_an_unsigned_transaction_is_refused_here_rather_than_by_the_node() {
        // rippled reads the empty SigningPubKey as a multisign attempt, finds
        // no Signers, and answers temBAD_SIGNATURE — which says the signature
        // is wrong rather than missing.
        let tx = json!({"TransactionType": "Payment", "Fee": "12", "Sequence": 1});
        let message = require_submittable(&tx).unwrap_err().to_string();

        assert!(message.contains("unsigned"), "{message}");
        assert!(message.contains("tx sign"), "{message}");
    }

    #[test]
    fn test_a_multisigned_transaction_counts_as_signed() {
        let tx = json!({
            "TransactionType": "Payment", "Fee": "12", "Sequence": 1,
            "Signers": [{"Signer": {"Account": "rA"}}],
        });

        assert!(require_submittable(&tx).is_ok());
    }

    #[test]
    fn test_a_ticketed_transaction_still_carries_sequence_zero() {
        // The field is mandatory whatever authorizes the transaction. Omitting
        // it is `invalidTransaction: Field 'Sequence' is required but missing`,
        // so this refuses locally rather than letting the node say it.
        let absent = signed(json!({
            "TransactionType": "Payment", "Fee": "12", "TicketSequence": 7,
        }));
        assert!(require_submittable(&absent).is_err());

        let present = signed(json!({
            "TransactionType": "Payment", "Fee": "12", "TicketSequence": 7, "Sequence": 0,
        }));
        assert!(require_submittable(&present).is_ok());
    }

    #[test]
    fn test_tes_is_success_and_tec_is_not() {
        assert!(classify("tesSUCCESS").is_ok());
        assert!(classify("tecUNFUNDED_PAYMENT").is_err());
        assert!(classify("tefBAD_AUTH").is_err());
    }
}
