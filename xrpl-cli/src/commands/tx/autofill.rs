//! `xrpl tx autofill` — fill in what only a node can tell you.
//!
//! One of two stages that touch the network. It fills `Fee`, `Sequence`,
//! `LastLedgerSequence` and `NetworkID`, and **only where they are absent**: a
//! value the caller set is theirs.
//!
//! It refuses a transaction that already carries a signature. Every field it
//! fills is a signing field, and XRPL has no fee-bump wrapper, so there is no
//! legal edit after the first signature — "autofill then sign again" produces a
//! transaction whose earlier signatures verify against nothing.
//!
//! It needs no key material at all, so a watch-only account can be prepared on
//! one machine and signed on another.

use serde_json::{json, Value};
use xrpl::asynch::clients::{AsyncJsonRpcClient, XRPLAsyncClient};
use xrpl::models::requests::generic_request::GenericRequest;

use crate::client;
use crate::commands::tx::args::{RequiredNetworkArgs, TxInput};
use crate::commands::tx::io;
use crate::error::Error;

#[derive(Debug, Clone, clap::Args)]
pub struct Cmd {
    #[command(flatten)]
    pub input: TxInput,

    #[command(flatten)]
    pub network: RequiredNetworkArgs,

    /// Use this fee instead of asking the node.
    #[arg(long)]
    pub fee: Option<String>,

    /// Use this sequence instead of asking the node.
    #[arg(long)]
    pub sequence: Option<u32>,

    /// Leave `Sequence` absent, for a transaction using a ticket.
    #[arg(long, conflicts_with = "sequence")]
    pub no_sequence: bool,

    /// Scale the fee for a multisigned transaction with this many signers.
    ///
    /// A transaction carrying N `Signer` entries must pay `(N + 1) ×` the
    /// reference fee. This is committed *before* any signature exists, because
    /// `Fee` is a signing field and XRPL has no fee-bump wrapper — so getting N
    /// wrong means `telINSUF_FEE_P`, or an overpayment, and a full re-collection
    /// of the quorum either way.
    #[arg(long, conflicts_with = "fee")]
    pub signers: Option<u32>,

    /// Leave `LastLedgerSequence` absent.
    ///
    /// The transaction then never expires, which is what a multisig collection
    /// sitting between signers needs — and also means it can be replayed until
    /// its sequence is consumed. Consume the sequence deliberately.
    #[arg(long)]
    pub no_last_ledger_sequence: bool,
}

impl Cmd {
    pub fn run(&self) -> Result<(), Error> {
        let url = self.network.url()?;
        let mut transactions = io::read_txs(&self.input.tx)?;

        let runtime = client::runtime()?;
        runtime.block_on(async {
            let node = AsyncJsonRpcClient::connect(client::parse_url(&url)?);

            for transaction in &mut transactions {
                self.fill(transaction, &node).await?;
            }

            Ok::<(), Error>(())
        })?;

        io::write_txs(&transactions)
    }

    async fn fill(&self, transaction: &mut Value, node: &AsyncJsonRpcClient) -> Result<(), Error> {
        refuse_if_signed(transaction)?;

        let account = transaction
            .get("Account")
            .and_then(Value::as_str)
            .ok_or_else(|| Error::other("transaction has no Account to autofill for"))?
            .to_string();

        let object = transaction
            .as_object_mut()
            .ok_or_else(|| Error::other("expected a transaction object"))?;

        if !object.contains_key("Fee") {
            let fee = match (&self.fee, self.signers) {
                (Some(fee), _) => fee.clone(),
                (None, Some(signers)) => multisign_fee(node, signers).await?,
                (None, None) => open_ledger_fee(node).await?,
            };
            object.insert("Fee".into(), Value::String(fee));
        }

        if !object.contains_key("Sequence") && !self.no_sequence {
            let sequence = match self.sequence {
                Some(sequence) => sequence,
                None => account_sequence(node, &account).await?,
            };
            object.insert("Sequence".into(), json!(sequence));
        }

        if !object.contains_key("LastLedgerSequence") && !self.no_last_ledger_sequence {
            let current = validated_ledger_index(node).await?;
            object.insert(
                "LastLedgerSequence".into(),
                json!(current + u32::from(xrpl::asynch::transaction::LEDGER_OFFSET)),
            );
        }

        Ok(())
    }
}

/// Refuse to touch a transaction that already carries a signature.
fn refuse_if_signed(transaction: &Value) -> Result<(), Error> {
    let has_signature = transaction.get("TxnSignature").is_some();
    let has_signers = transaction
        .get("Signers")
        .and_then(Value::as_array)
        .is_some_and(|signers| !signers.is_empty());

    if has_signature || has_signers {
        return Err(Error::other(
            "already signed: every field autofill sets is a signing field, and XRPL has no \
             fee-bump wrapper, so editing now invalidates the signatures already attached",
        ));
    }

    Ok(())
}

async fn open_ledger_fee(node: &AsyncJsonRpcClient) -> Result<String, Error> {
    let response = node
        .request(
            GenericRequest::builder("fee")
                .params(serde_json::Map::new())
                .build()
                .into(),
        )
        .await
        .map_err(Error::Client)?;

    client::result_value(&response)?["drops"]["open_ledger_fee"]
        .as_str()
        .map(str::to_string)
        .ok_or_else(|| Error::other("node did not report an open ledger fee"))
}

/// The fee a multisigned transaction with `signers` entries must pay.
///
/// Built on the *reference* fee rather than the open-ledger fee. A proposal
/// that has to sit while a quorum is collected should queue if the network is
/// busy, not be priced for a ledger that closed long before the last signature
/// arrived.
async fn multisign_fee(node: &AsyncJsonRpcClient, signers: u32) -> Result<String, Error> {
    let response = node
        .request(
            GenericRequest::builder("fee")
                .params(serde_json::Map::new())
                .build()
                .into(),
        )
        .await
        .map_err(Error::Client)?;

    let reference: u64 = client::result_value(&response)?["drops"]["base_fee"]
        .as_str()
        .ok_or_else(|| Error::other("node did not report a reference fee"))?
        .parse()
        .map_err(|_| Error::other("node reported a reference fee that is not a number"))?;

    Ok((reference * (u64::from(signers) + 1)).to_string())
}

async fn account_sequence(node: &AsyncJsonRpcClient, account: &str) -> Result<u32, Error> {
    let mut params = serde_json::Map::new();
    params.insert("account".into(), Value::String(account.to_string()));
    params.insert("ledger_index".into(), Value::String("validated".into()));

    let response = node
        .request(
            GenericRequest::builder("account_info")
                .params(params)
                .build()
                .into(),
        )
        .await
        .map_err(Error::Client)?;

    let result = client::result_value(&response)?;
    result["account_data"]["Sequence"]
        .as_u64()
        .map(|sequence| sequence as u32)
        .ok_or_else(|| {
            Error::other(format!(
                "node did not report a sequence for {account}: {}",
                result
            ))
        })
}

async fn validated_ledger_index(node: &AsyncJsonRpcClient) -> Result<u32, Error> {
    let response = node
        .request(
            GenericRequest::builder("ledger")
                .params({
                    let mut params = serde_json::Map::new();
                    params.insert("ledger_index".into(), Value::String("validated".into()));
                    params
                })
                .build()
                .into(),
        )
        .await
        .map_err(Error::Client)?;

    client::result_value(&response)?["ledger_index"]
        .as_u64()
        .map(|index| index as u32)
        .ok_or_else(|| Error::other("node did not report a validated ledger index"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_a_signed_transaction_is_refused() {
        let tx = json!({"TransactionType": "Payment", "TxnSignature": "DEAD"});
        let message = refuse_if_signed(&tx).unwrap_err().to_string();

        assert!(message.contains("already signed"), "{message}");
    }

    #[test]
    fn test_a_multisigned_transaction_is_refused() {
        let tx = json!({"TransactionType": "Payment", "Signers": [{"Signer": {}}]});
        assert!(refuse_if_signed(&tx).is_err());
    }

    #[test]
    fn test_an_empty_signers_array_is_not_a_signature() {
        let tx = json!({"TransactionType": "Payment", "Signers": []});
        assert!(refuse_if_signed(&tx).is_ok());
    }
}
