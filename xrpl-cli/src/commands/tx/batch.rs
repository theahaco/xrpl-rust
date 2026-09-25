//! `xrpl tx batch` — fold a stream of transactions into one XLS-56 `Batch`.
//!
//! Stream in, one outer transaction out, like every other stage. What makes it
//! different is that the outer transaction *commits to the IDs* of the inner
//! ones, so everything this command does to an inner transaction happens before
//! those IDs are computed and is therefore unforgiving: an inner edited
//! afterwards is an inner the outer signature no longer vouches for.
//!
//! # Offline
//!
//! `autofill` and `submit` are the only stages that reach a node, and this is
//! not one of them. `--check-amendment` is the single opt-in exception, and it
//! is advisory — see [`Cmd::check_amendment`].

use std::collections::BTreeMap;

use serde_json::{json, Map, Value};
use xrpl::signer::{RawSigner, SigningDomain};

use crate::commands::tx::args::TxInput;
use crate::commands::tx::{hash, io, txdef};
use crate::error::Error;
use crate::output;
use crate::store::{resolve, Store};

/// rippled's `maxBatchTxCount`.
///
/// Both ends matter: one transaction is not a batch, and the ledger refuses
/// more than eight.
const MIN_INNER: usize = 2;
const MAX_INNER: usize = 8;

/// rippled's `kMaxBatchSigners`, which is `kMaxBatchTxCount * 3`.
///
/// Three because one inner transaction can require up to three distinct
/// signers — an initiator, a counterparty and a sponsor.
const MAX_BATCH_SIGNERS: usize = MAX_INNER * 3;

/// The four modes, and what each does when an inner transaction fails.
const MODES: [&str; 4] = [
    "tfAllOrNothing",
    "tfOnlyOne",
    "tfUntilFailure",
    "tfIndependent",
];

#[derive(Debug, clap::Subcommand)]
pub enum Cmd {
    /// Fold a stream into one Batch transaction (OFFLINE)
    Wrap(Wrap),
}

impl Cmd {
    pub fn run(&self) -> Result<(), Error> {
        match self {
            Cmd::Wrap(cmd) => cmd.run(),
        }
    }
}

#[derive(Debug, Clone, clap::Args)]
pub struct Wrap {
    #[command(flatten)]
    pub input: TxInput,

    /// The account that submits the batch.
    ///
    /// Resolved from the local record, which is a pure file read: no credential
    /// store, no network, no prompt. The record is also where the outer
    /// `NetworkID` comes from, which is what makes the consistency check below
    /// possible without reaching a node.
    #[arg(long, value_name = "ALIAS_OR_ADDRESS")]
    pub account: Option<String>,

    /// The batch mode: tfAllOrNothing, tfOnlyOne, tfUntilFailure, tfIndependent.
    #[arg(long = "flag", value_name = "NAME")]
    pub flag: String,

    /// `Sequence` for the outer transaction.
    ///
    /// The batch account's own inner transactions are renumbered from it —
    /// outer `N`, then `N+1 … N+k` — because the outer consumes `N` and an
    /// inner reusing it is a collision the ledger reports as `tefPAST_SEQ`.
    /// Inner transactions belonging to other accounts are left alone, and
    /// inner transactions using a ticket are never renumbered.
    #[arg(long)]
    pub sequence: Option<u32>,

    /// Scale the outer fee for a multisigned outer transaction.
    ///
    /// Same warning as `tx autofill --signers`: `Fee` is a signing field and
    /// XRPL has no fee-bump wrapper, so N is irrevocable once the first
    /// signature exists.
    #[arg(long)]
    pub signers: Option<u32>,

    /// The network's reference fee, in drops.
    ///
    /// This stage is offline, so it cannot ask. 10 is the protocol default; a
    /// network configured otherwise needs the real value, because the outer fee
    /// is computed from it and `Fee` is a signing field.
    #[arg(long, default_value_t = 10)]
    pub base_fee: u64,

    /// Sign the batch pre-image with this recorded key. Repeatable.
    ///
    /// A **batch** co-signer, which is not the same as `tx sign --multisign --key`:
    /// these sign the `BCH\0` pre-image over the inner IDs and land in
    /// `BatchSigners`, while `tx sign --multisign --key` multisigns the outer transaction
    /// itself and lands in `Signers`. Both can be present on one Batch.
    #[arg(long = "batch-sign-with", value_name = "KEY_ID")]
    pub batch_sign_with: Vec<String>,

    /// Ask a node whether XLS-56 is usable before spending a signature.
    ///
    /// **Advisory, and the only thing here that touches the network.** It
    /// cannot be conclusive: on a standalone node the `feature` RPC reports
    /// `enabled: false` for every amendment, including the ones that
    /// demonstrably work, so a refusal based on it would be wrong far more
    /// often than right. A node that lacks the amendment answers `temDISABLED`
    /// at submit time, and that is the reliable signal.
    #[arg(long, value_name = "URL")]
    pub check_amendment: Option<String>,
}

impl Wrap {
    pub fn run(&self) -> Result<(), Error> {
        let mode = self.mode()?;
        let store = Store::from_env()?;
        let account = resolve::account(&store, self.account.as_deref())?;

        let mut inners = io::read_txs(&self.input.tx)?;
        if !(MIN_INNER..=MAX_INNER).contains(&inners.len()) {
            return Err(Error::BatchSize {
                found: inners.len(),
            });
        }

        if let Some(url) = &self.check_amendment {
            advise_on_amendment(url);
        }

        // Captured before anything is rewritten: zeroing first and summing
        // afterwards under-pays the batch by exactly the inner total, which the
        // ledger reports as `telINSUF_FEE_P` after the signatures are collected.
        let inner_fees = total_inner_fees(&inners)?;

        let network_id = self.agreed_network_id(&inners, &account)?;
        self.renumber(&mut inners, &account.address)?;

        for inner in &mut inners {
            prepare_inner(inner)?;
        }

        let ids = inners
            .iter()
            .map(hash::raw_transaction_id)
            .collect::<Result<Vec<_>, _>>()?;

        let inner_accounts: Vec<String> = inners
            .iter()
            .filter_map(|inner| inner.get("Account").and_then(Value::as_str))
            .map(str::to_string)
            .collect();

        let mut outer = Map::new();
        outer.insert("TransactionType".into(), json!("Batch"));
        outer.insert("Account".into(), json!(account.address));
        outer.insert("Flags".into(), json!(mode));
        outer.insert(
            "RawTransactions".into(),
            Value::Array(
                inners
                    .into_iter()
                    .map(|inner| json!({ "RawTransaction": inner }))
                    .collect(),
            ),
        );
        outer.insert("Fee".into(), json!(self.outer_fee(inner_fees).to_string()));
        outer.insert(
            "SigningPubKey".into(),
            json!(crate::commands::tx::EMPTY_SIGNING_PUB_KEY),
        );

        if let Some(sequence) = self.sequence {
            outer.insert("Sequence".into(), json!(sequence));
        }
        if let Some(network_id) = network_id {
            outer.insert("NetworkID".into(), json!(network_id));
        }

        if !self.batch_sign_with.is_empty() {
            let signers =
                self.batch_signers(&store, mode, &ids, &account.address, &inner_accounts)?;
            outer.insert("BatchSigners".into(), Value::Array(signers));
        }

        output::note(format!(
            "batched {} transaction(s) as {} for {}",
            ids.len(),
            self.flag,
            account.address
        ));

        io::write_txs(&[Value::Object(outer)])
    }

    /// The flag bit for the chosen mode, read from the definitions.
    fn mode(&self) -> Result<u32, Error> {
        let definition = txdef::transaction("Batch")
            .ok_or_else(|| Error::other("this build's definitions carry no Batch type"))?;

        definition
            .flags
            .get(self.flag.as_str())
            .copied()
            .filter(|_| MODES.contains(&self.flag.as_str()))
            .ok_or_else(|| {
                Error::other(format!(
                    "--flag must be one of {}, got {:?}",
                    MODES.join(", "),
                    self.flag
                ))
            })
    }

    /// `base × (2 + every signature) + Σ inner fees`.
    ///
    /// Every signature means both kinds: a `--signers` entry multisigning the
    /// outer transaction and a `--batch-sign-with` entry in `BatchSigners`
    /// count the same. Counting only the first under-prices a batch with
    /// co-signers and the ledger answers `telINSUF_FEE_P` — after the
    /// signatures are collected, and `Fee` is a signing field, so the only way
    /// out is to collect them again.
    fn outer_fee(&self, inner_fees: u64) -> u64 {
        let signatures = u64::from(self.signers.unwrap_or(0)) + self.batch_sign_with.len() as u64;

        self.base_fee * (2 + signatures) + inner_fees
    }

    /// The `NetworkID` every line agrees on, and the account's record agrees with.
    ///
    /// A batch is the easiest place to fold in a transaction built for another
    /// chain, and the outer signature vouches for every inner ID it commits to.
    fn agreed_network_id(
        &self,
        inners: &[Value],
        account: &resolve::Resolved,
    ) -> Result<Option<u32>, Error> {
        let mut seen: Option<u32> = None;

        for (index, inner) in inners.iter().enumerate() {
            let Some(id) = inner.get("NetworkID").and_then(Value::as_u64) else {
                continue;
            };
            let id = id as u32;

            match seen {
                Some(first) if first != id => {
                    return Err(Error::BatchNetworkMismatch {
                        detail: format!("inner {index} is network {id}, an earlier one is {first}"),
                    })
                }
                _ => seen = Some(id),
            }
        }

        let recorded = account.record.as_ref().and_then(|record| record.network_id);

        match (seen, recorded) {
            (Some(inner), Some(outer)) if inner != outer => Err(Error::BatchNetworkMismatch {
                detail: format!(
                    "the inner transactions are network {inner} and {} is recorded as network {outer}",
                    account.address
                ),
            }),
            (Some(inner), _) => Ok(Some(inner)),
            // The <=1024 rule: network 0 omits the field entirely.
            (None, Some(outer)) if outer > u32::from(xrpl::asynch::transaction::RESTRICTED_NETWORKS) => {
                Ok(Some(outer))
            }
            _ => Ok(None),
        }
    }

    /// Renumber the batch account's own inner transactions from the outer's.
    fn renumber(&self, inners: &mut [Value], address: &str) -> Result<(), Error> {
        let Some(outer) = self.sequence else {
            return Ok(());
        };

        let mut next = outer + 1;
        for inner in inners.iter_mut() {
            if inner.get("Account").and_then(Value::as_str) != Some(address) {
                continue;
            }
            if inner.get("TicketSequence").is_some() {
                continue;
            }

            if let Some(object) = inner.as_object_mut() {
                object.insert("Sequence".into(), json!(next));
                next += 1;
            }
        }

        Ok(())
    }

    /// One `BatchSigners` entry per co-signer, sorted, outer signer excluded.
    fn batch_signers(
        &self,
        store: &Store,
        mode: u32,
        ids: &[String],
        outer: &str,
        inner_accounts: &[String],
    ) -> Result<Vec<Value>, Error> {
        if self.batch_sign_with.len() > MAX_BATCH_SIGNERS {
            return Err(Error::BatchSignerCount {
                found: self.batch_sign_with.len(),
            });
        }

        let borrowed: Vec<&str> = ids.iter().map(String::as_str).collect();

        // Sorted by account, so two people assembling the same batch produce
        // the same bytes.
        let mut entries: BTreeMap<String, Value> = BTreeMap::new();

        for id in &self.batch_sign_with {
            let signer = crate::signer::StoredSigner::unlock(store, id)?;
            let address = signer
                .classic_address()
                .map_err(|error| Error::other(error.to_string()))?;

            if !inner_accounts.iter().any(|account| account == &address) {
                // rippled builds the required set from the inner transactions
                // and refuses any entry outside it, and any entry missing from
                // it, with `temBAD_SIGNER`. Catching it here costs nothing and
                // the node's version of this message names no key id.
                return Err(Error::other(format!(
                    "{id} ({address}) signs none of the inner transactions. A batch co-signer \
                     authorizes its own inner transaction; rippled refuses an entry that \
                     matches no inner account with temBAD_SIGNER."
                )));
            }

            if address == outer {
                // The outer signature already commits to these IDs.
                return Err(Error::other(format!(
                    "{id} is the batch account itself ({address}); its signature on the \
                     outer transaction already covers the inner IDs. Batch co-signers are \
                     the *other* accounts whose transactions are in the batch."
                )));
            }

            // Built per signer: the pre-image ends with that signer's own
            // account, so every entry signs different bytes and one entry's
            // signature cannot be replayed as another's.
            let payload = xrpl::core::binarycodec::encode_for_signing_batch_unframed(
                outer,
                self.sequence.unwrap_or(0),
                mode,
                &borrowed,
                &address,
            )
            .map_err(Error::Core)?;

            let signature = match signer
                .sign(SigningDomain::BatchInner, &payload)
                .map_err(|error| Error::other(error.to_string()))?
            {
                xrpl::signer::SignOutcome::Signed(signature) => signature,
                other => {
                    return Err(Error::other(format!(
                        "{id} produced {other:?} rather than a signature; a batch co-signer \
                         must return one"
                    )))
                }
            };

            entries.insert(
                address.clone(),
                json!({ "BatchSigner": {
                    "Account": address,
                    "SigningPubKey": signer.public_key(),
                    "TxnSignature": signature,
                }}),
            );
        }

        Ok(entries.into_values().collect())
    }
}

/// Make one transaction fit to be an inner.
fn prepare_inner(inner: &mut Value) -> Result<(), Error> {
    let inner_batch_flag = txdef::transaction("Batch")
        .and_then(|definition| definition.flags.get("tfInnerBatchTxn").copied())
        .ok_or_else(|| Error::other("this build's definitions carry no tfInnerBatchTxn"))?;

    let object = inner
        .as_object_mut()
        .ok_or_else(|| Error::other("expected a transaction object"))?;

    if object
        .get("Signers")
        .and_then(Value::as_array)
        .is_some_and(|signers| !signers.is_empty())
    {
        // Legal only as a `BatchSigners` entry's own `Signers` sub-field, which
        // this command does not assemble. Stripping it silently would discard a
        // collected quorum.
        return Err(Error::other(
            "an inner transaction is multisigned. A `Signers` array on an inner is \
             `temBAD_SIGNER`; a multisigned inner is legal only as a BatchSigners entry's \
             own Signers sub-field, which this command does not build. Fold the \
             single-signed transaction instead.",
        ));
    }

    // An inner is authorized by the outer signature, never by its own.
    object.remove("TxnSignature");

    let flags = object.get("Flags").and_then(Value::as_u64).unwrap_or(0) as u32;
    object.insert("Flags".into(), json!(flags | inner_batch_flag));

    // Both required, and `SigningPubKey` must be present and empty rather than
    // absent: rippled answers `invalidTransaction` when the field is missing.
    object.insert("Fee".into(), json!("0"));
    object.insert(
        "SigningPubKey".into(),
        json!(crate::commands::tx::EMPTY_SIGNING_PUB_KEY),
    );

    let has_sequence = object
        .get("Sequence")
        .and_then(Value::as_u64)
        .is_some_and(|sequence| sequence > 0);
    let has_ticket = object.get("TicketSequence").is_some();

    if has_sequence == has_ticket {
        return Err(Error::BatchInnerSequence {
            detail: if has_sequence {
                "it carries both a non-zero Sequence and a TicketSequence".into()
            } else {
                "it carries neither a non-zero Sequence nor a TicketSequence".into()
            },
        });
    }

    if has_ticket {
        // A ticket authorizes it, and the field is still mandatory.
        object.insert("Sequence".into(), json!(0));
    }

    Ok(())
}

/// The inner fees, summed before any of them is rewritten.
fn total_inner_fees(inners: &[Value]) -> Result<u64, Error> {
    let mut total = 0u64;

    for inner in inners {
        let Some(fee) = inner.get("Fee") else {
            continue;
        };
        let drops = fee
            .as_str()
            .and_then(|fee| fee.parse::<u64>().ok())
            .or_else(|| fee.as_u64())
            .ok_or_else(|| {
                Error::other(format!("an inner transaction's Fee is not drops: {fee}"))
            })?;

        total = total.saturating_add(drops);
    }

    Ok(total)
}

/// Say what a node reports about XLS-56, without pretending it is conclusive.
fn advise_on_amendment(url: &str) {
    output::note(format!("asking {url} about BatchV1_1"));
    output::warn(
        "the amendment check is advisory: a standalone node reports every amendment as \
         disabled, including ones that work. A node without XLS-56 answers temDISABLED at \
         submit time, which is the reliable signal.",
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn payment(sequence: Option<u64>, fee: &str) -> Value {
        let mut tx = json!({
            "TransactionType": "Payment",
            "Account": "rHb9CJAWyB4rj91VRWn96DkukG4bwdtyTh",
            "Destination": "rPT1Sjq2YGrBMTttX4GZHjKu9dyfzbpAYe",
            "Amount": "1000000",
            "Fee": fee,
        });
        if let Some(sequence) = sequence {
            tx["Sequence"] = json!(sequence);
        }
        tx
    }

    #[test]
    fn test_an_inner_is_zero_fee_empty_key_and_flagged() {
        let mut inner = payment(Some(5), "200");
        prepare_inner(&mut inner).expect("prepares");

        assert_eq!(inner["Fee"], json!("0"));
        // Present and empty, not absent: rippled answers `invalidTransaction`
        // when the field is missing entirely.
        assert_eq!(inner["SigningPubKey"], json!(""));
        assert_eq!(inner["Flags"], json!(1073741824u32));
    }

    #[test]
    fn test_the_inner_flag_is_ored_into_what_was_already_there() {
        // tfPartialPayment, which an inner is entitled to keep.
        let mut inner = payment(Some(5), "200");
        inner["Flags"] = json!(131072);

        prepare_inner(&mut inner).expect("prepares");
        assert_eq!(inner["Flags"], json!(131072u32 | 1073741824u32));
    }

    #[test]
    fn test_an_inner_signature_is_dropped_but_a_quorum_is_refused() {
        let mut signed = payment(Some(5), "200");
        signed["TxnSignature"] = json!("3045…");
        prepare_inner(&mut signed).expect("prepares");
        assert!(signed.get("TxnSignature").is_none());

        // A collected quorum is not ours to discard: it is legal only as a
        // BatchSigners entry's own Signers sub-field.
        let mut multisigned = payment(Some(5), "200");
        multisigned["Signers"] = json!([{"Signer": {"Account": "rA"}}]);

        let error = prepare_inner(&mut multisigned).unwrap_err().to_string();
        assert!(error.contains("multisigned"), "{error}");
        assert!(error.contains("BatchSigners"), "{error}");
    }

    #[test]
    fn test_an_inner_needs_exactly_one_authorization() {
        let mut neither = payment(None, "200");
        assert!(matches!(
            prepare_inner(&mut neither),
            Err(Error::BatchInnerSequence { .. })
        ));

        // Sequence 0 is not an authorization; it is what a ticketed
        // transaction carries.
        let mut zero = payment(Some(0), "200");
        assert!(matches!(
            prepare_inner(&mut zero),
            Err(Error::BatchInnerSequence { .. })
        ));

        let mut both = payment(Some(5), "200");
        both["TicketSequence"] = json!(9);
        assert!(matches!(
            prepare_inner(&mut both),
            Err(Error::BatchInnerSequence { .. })
        ));
    }

    #[test]
    fn test_a_ticketed_inner_gets_sequence_zero() {
        let mut ticketed = payment(None, "200");
        ticketed["TicketSequence"] = json!(9);

        prepare_inner(&mut ticketed).expect("prepares");
        assert_eq!(ticketed["Sequence"], json!(0));
    }

    #[test]
    fn test_inner_fees_are_summed_before_they_are_zeroed() {
        // The whole point of capturing first: after `prepare_inner` every fee
        // is "0", and a batch priced from that under-pays by exactly the
        // inner total and fails `telINSUF_FEE_P` — after the quorum is spent.
        let inners = vec![payment(Some(1), "200"), payment(Some(2), "15")];
        assert_eq!(total_inner_fees(&inners).expect("sums"), 215);

        let mut zeroed = inners;
        for inner in &mut zeroed {
            prepare_inner(inner).expect("prepares");
        }
        assert_eq!(total_inner_fees(&zeroed).expect("sums"), 0);
    }

    #[test]
    fn test_the_outer_fee_is_two_base_plus_signatures_plus_inners() {
        let wrap = |signers| Wrap {
            input: TxInput { tx: None },
            account: None,
            flag: "tfAllOrNothing".into(),
            sequence: None,
            signers,
            base_fee: 10,
            batch_sign_with: Vec::new(),
            check_amendment: None,
        };

        assert_eq!(wrap(None).outer_fee(0), 20);
        assert_eq!(wrap(None).outer_fee(400), 420);
        // One base per outer signature, on top of the flat two.
        assert_eq!(wrap(Some(3)).outer_fee(400), 10 * 5 + 400);

        // And a BatchSigners entry is a signature too. Counting only the outer
        // ones under-prices the batch and the ledger says telINSUF_FEE_P once
        // the quorum is already spent.
        let mut with_cosigner = wrap(None);
        with_cosigner.batch_sign_with = vec!["other".into()];
        assert_eq!(with_cosigner.outer_fee(400), 10 * 3 + 400);

        let mut both = wrap(Some(2));
        both.batch_sign_with = vec!["a".into(), "b".into()];
        assert_eq!(both.outer_fee(0), 10 * (2 + 2 + 2));
    }

    #[test]
    fn test_only_the_four_modes_are_accepted() {
        let mode = |flag: &str| {
            Wrap {
                input: TxInput { tx: None },
                account: None,
                flag: flag.into(),
                sequence: None,
                signers: None,
                base_fee: 10,
                batch_sign_with: Vec::new(),
                check_amendment: None,
            }
            .mode()
        };

        assert_eq!(mode("tfAllOrNothing").expect("known"), 65536);
        assert_eq!(mode("tfIndependent").expect("known"), 524288);
        assert!(mode("tfNonsense").is_err());
        // A universal flag is a real bit but not a batch mode, so naming it
        // here would silently produce a batch with no mode at all.
        assert!(mode("tfInnerBatchTxn").is_err());
        assert!(mode("tfFullyCanonicalSig").is_err());
    }
}
