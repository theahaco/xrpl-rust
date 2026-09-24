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

use std::collections::BTreeMap;

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

    /// Set `Sequence` to 0, for a transaction using a ticket.
    ///
    /// Zero, not absent. rippled answers `invalidTransaction: Field 'Sequence'
    /// is required but missing` to a ticketed transaction with no `Sequence`
    /// at all — the field is mandatory and zero is what says "a ticket
    /// authorizes this, not a sequence".
    #[arg(long, conflicts_with = "sequence")]
    pub no_sequence: bool,

    /// Number a stream consecutively from one `account_info` call per account.
    ///
    /// Without it, every line costs its own round trip and they all come back
    /// with the *same* sequence, so only the first can apply. The counter lives
    /// for this invocation and is never persisted — there is no reserved
    /// sequence anywhere, and concurrent invocations collide as `tefPAST_SEQ`.
    ///
    /// The account is each line's own `Account` field, never `--account` or the
    /// config default, and the stage groups by it: a mixed-account stream, which
    /// is the normal shape for a batch, is numbered correctly per account.
    #[arg(long, conflicts_with_all = ["sequence", "no_sequence", "tickets"])]
    pub sequence_from_auto: bool,

    /// Assign `TicketSequence` from a range, one per line: `--tickets 165:167`.
    ///
    /// Inclusive at both ends. `Sequence` is set to 0 on every line, because a
    /// ticket is what authorizes it. Use the range a `TicketCreate` produced:
    /// tickets need no ordering, so the lines can be submitted in any order and
    /// by different people at the same time.
    #[arg(long, value_name = "FIRST:LAST", conflicts_with_all = ["sequence", "no_sequence"])]
    pub tickets: Option<String>,

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

/// What one invocation looked up, so a stream pays for each answer once.
///
/// A stream used to cost three round trips *per line* — a `fee`, an
/// `account_info` and a `ledger` — and the sequences all came back identical,
/// so only the first line could ever apply.
#[derive(Default)]
struct Lookups {
    fee: Option<String>,
    last_ledger_sequence: Option<u32>,
    /// Next sequence per account, seeded from one `account_info` each.
    next_sequence: BTreeMap<String, u32>,
    /// Tickets left to hand out, in order.
    tickets: Vec<u32>,
    /// `Some(id)` when this network requires `NetworkID`, resolved once.
    network_id: Option<Option<u32>>,
}

impl Cmd {
    pub fn run(&self) -> Result<(), Error> {
        let url = self.network.url()?;
        let mut transactions = io::read_txs(&self.input.tx)?;

        let mut tickets = self.ticket_range()?;
        if let Some(range) = &tickets {
            if range.len() < transactions.len() {
                return Err(Error::other(format!(
                    "--tickets covers {} ticket(s) and the stream has {} transaction(s). \
                     A ticket is consumed by exactly one transaction.",
                    range.len(),
                    transactions.len()
                )));
            }
        }

        let runtime = client::runtime()?;
        runtime.block_on(async {
            let node = AsyncJsonRpcClient::connect(client::parse_url(&url)?);
            let mut lookups = Lookups {
                tickets: tickets.take().unwrap_or_default(),
                ..Lookups::default()
            };

            for transaction in &mut transactions {
                self.fill(transaction, &node, &mut lookups).await?;
            }

            Ok::<(), Error>(())
        })?;

        io::write_txs(&transactions)
    }

    /// The ticket range, expanded and validated.
    fn ticket_range(&self) -> Result<Option<Vec<u32>>, Error> {
        let Some(spec) = &self.tickets else {
            return Ok(None);
        };

        let (first, last) = spec.split_once(':').ok_or_else(|| {
            Error::other(format!(
                "--tickets wants FIRST:LAST, got {spec:?}. A single ticket is `N:N`."
            ))
        })?;

        let parse = |value: &str| -> Result<u32, Error> {
            value
                .trim()
                .parse()
                .map_err(|_| Error::other(format!("--tickets: {value:?} is not a ticket sequence")))
        };
        let (first, last) = (parse(first)?, parse(last)?);

        if last < first {
            return Err(Error::other(format!(
                "--tickets {first}:{last} runs backwards"
            )));
        }

        Ok(Some((first..=last).collect()))
    }

    async fn fill(
        &self,
        transaction: &mut Value,
        node: &AsyncJsonRpcClient,
        lookups: &mut Lookups,
    ) -> Result<(), Error> {
        refuse_if_signed(transaction)?;

        let account = transaction
            .get("Account")
            .and_then(Value::as_str)
            .ok_or_else(|| Error::other("transaction has no Account to autofill for"))?
            .to_string();

        // Resolved before the mutable borrow, because both need the node.
        let fee = if transaction.get("Fee").is_none() {
            Some(self.fee_for(node, lookups).await?)
        } else {
            None
        };

        let sequence = self
            .sequence_for(node, lookups, &account, transaction)
            .await?;

        let last_ledger =
            if transaction.get("LastLedgerSequence").is_none() && !self.no_last_ledger_sequence {
                Some(self.last_ledger_for(node, lookups).await?)
            } else {
                None
            };

        let network_id = if transaction.get("NetworkID").is_none() {
            network_id_for(node, lookups).await?
        } else {
            None
        };

        let object = transaction
            .as_object_mut()
            .ok_or_else(|| Error::other("expected a transaction object"))?;

        if let Some(fee) = fee {
            object.insert("Fee".into(), Value::String(fee));
        }
        if let Some((key, value)) = sequence {
            object.insert(key.into(), json!(value));
            // A ticket authorizes the transaction, so `Sequence` is 0 — and
            // present, because rippled requires the field either way.
            if key == "TicketSequence" {
                object.insert("Sequence".into(), json!(0));
            }
        }
        if let Some(last_ledger) = last_ledger {
            object.insert("LastLedgerSequence".into(), json!(last_ledger));
        }
        if let Some(network_id) = network_id {
            object.insert("NetworkID".into(), json!(network_id));
        }

        Ok(())
    }

    async fn fee_for(
        &self,
        node: &AsyncJsonRpcClient,
        lookups: &mut Lookups,
    ) -> Result<String, Error> {
        if let Some(fee) = &lookups.fee {
            return Ok(fee.clone());
        }

        let fee = match (&self.fee, self.signers) {
            (Some(fee), _) => fee.clone(),
            (None, Some(signers)) => multisign_fee(node, signers).await?,
            (None, None) => open_ledger_fee(node).await?,
        };

        lookups.fee = Some(fee.clone());
        Ok(fee)
    }

    async fn last_ledger_for(
        &self,
        node: &AsyncJsonRpcClient,
        lookups: &mut Lookups,
    ) -> Result<u32, Error> {
        if let Some(last) = lookups.last_ledger_sequence {
            return Ok(last);
        }

        let current = validated_ledger_index(node).await?;
        let last = current + u32::from(xrpl::asynch::transaction::LEDGER_OFFSET);

        lookups.last_ledger_sequence = Some(last);
        Ok(last)
    }

    /// Which sequencing field this line gets, if any.
    async fn sequence_for(
        &self,
        node: &AsyncJsonRpcClient,
        lookups: &mut Lookups,
        account: &str,
        transaction: &Value,
    ) -> Result<Option<(&'static str, u32)>, Error> {
        if self.tickets.is_some() {
            if transaction.get("TicketSequence").is_some() {
                return Ok(None);
            }

            let ticket = lookups.tickets.first().copied().ok_or_else(|| {
                Error::other("--tickets ran out: the range is shorter than the stream")
            })?;
            lookups.tickets.remove(0);

            return Ok(Some(("TicketSequence", ticket)));
        }

        if transaction.get("Sequence").is_some() || self.no_sequence {
            // `--no-sequence` still has to write the field; the caller means
            // "a ticket authorizes this", and rippled rejects it absent.
            return Ok(
                if self.no_sequence && transaction.get("Sequence").is_none() {
                    Some(("Sequence", 0))
                } else {
                    None
                },
            );
        }

        if let Some(sequence) = self.sequence {
            return Ok(Some(("Sequence", sequence)));
        }

        if self.sequence_from_auto {
            // One `account_info` per distinct account, then count up locally.
            let next = match lookups.next_sequence.get(account) {
                Some(next) => *next,
                None => account_sequence(node, account).await?,
            };
            lookups.next_sequence.insert(account.to_string(), next + 1);

            return Ok(Some(("Sequence", next)));
        }

        Ok(Some(("Sequence", account_sequence(node, account).await?)))
    }
}

/// The `NetworkID` this network requires, if it requires one.
///
/// This module's documentation has always claimed autofill fills `NetworkID`.
/// It did not: there was no branch for it anywhere in the CLI, and the test
/// asserting the field was absent passed only because the standalone node runs
/// network 0, where absent is correct. On a network with id >= 1025 the
/// pipeline emitted transactions with no replay protection at all.
///
/// The rule is the library's, not a second copy of it: `txn_needs_network_id`
/// wants the id **and** a `build_version` of at least 1.11.0, because older
/// nodes reject the field. Resolved once per invocation.
async fn network_id_for(
    node: &AsyncJsonRpcClient,
    lookups: &mut Lookups,
) -> Result<Option<u32>, Error> {
    if let Some(cached) = lookups.network_id {
        return Ok(cached);
    }

    let common = node.get_common_fields().await.map_err(Error::Client)?;
    let needed =
        xrpl::asynch::transaction::txn_needs_network_id(common.clone()).map_err(Error::Helper)?;

    let resolved = needed.then_some(common.network_id).flatten();
    lookups.network_id = Some(resolved);

    Ok(resolved)
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
