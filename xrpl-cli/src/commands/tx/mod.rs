//! `xrpl tx` — a pipeable transaction builder.
//!
//! Every stage reads one machine artifact on stdin and writes one on stdout,
//! and puts everything a human reads on stderr:
//!
//! ```text
//! xrpl tx new Payment --field Account=r… --field Amount=10000000 \
//!   | xrpl tx autofill --url … \
//!   | xrpl tx sign --seed-file ~/keys/alice \
//!   | xrpl tx submit --wait --url …
//! ```
//!
//! Which stages touch the network is a property of the stage, not a flag:
//! `new`, `edit`, `batch wrap`, `sign`, `hash`, `digest`, `blob`, `decode` and
//! `mpt-issuance-id` are offline and need no URL at all. `autofill` and `submit` are the only two that
//! reach a node.
//!
//! # A stream is the general case
//!
//! Every stage reads and writes `Vec<Value>`; one transaction is the degenerate
//! case. `tx autofill --sequence-from-auto` numbers a stream consecutively from
//! one `account_info` call per distinct account, and `tx sign` unlocks its key
//! once and signs every line with it — a stream must never mean a passphrase
//! prompt per transaction.
//!
//! The pipe format is not configurable. There is no `--output`, no `--format`
//! and no `--json`, because a stage that can be told what to emit is a stage
//! every downstream consumer has to sniff.

pub mod args;
pub mod autofill;
pub mod batch;
pub mod blob;
pub mod decode;
pub mod digest;
pub mod edit;
pub mod fields;
pub mod hash;
pub mod io;
pub mod merge;
pub mod mpt_issuance_id;
pub mod new;
pub mod sign;
pub mod signers;
pub mod submit;
pub mod txdef;
pub mod value;

use crate::error::Error;

#[derive(Debug, clap::Subcommand)]
pub enum Cmd {
    /// Build an unsigned transaction (OFFLINE)
    #[command(subcommand)]
    New(new::Cmd),
    /// Fill in Fee, Sequence, LastLedgerSequence and NetworkID
    Autofill(autofill::Cmd),
    /// Fold a stream into one XLS-56 Batch (OFFLINE)
    #[command(subcommand)]
    Batch(batch::Cmd),
    /// Open a transaction in $EDITOR (OFFLINE)
    Edit(edit::Cmd),
    /// Sign a transaction (OFFLINE)
    Sign(sign::Cmd),
    /// Combine independently signed copies of one transaction (OFFLINE)
    Merge(merge::Cmd),
    /// Encode and submit a transaction
    Submit(submit::Cmd),
    /// Print the transaction ID (OFFLINE, signed only)
    Hash(hash::Cmd),
    /// Print what a signer actually signs (OFFLINE)
    Digest(digest::Cmd),
    /// Render a transaction as a hex blob (OFFLINE)
    Blob(blob::Cmd),
    /// Parse a hex blob back into JSON (OFFLINE)
    Decode(decode::Cmd),
    /// Derive an MPT issuance ID (OFFLINE)
    MptIssuanceId(mpt_issuance_id::Cmd),
    /// List the fields and flags a transaction type accepts (OFFLINE)
    Fields(fields::Cmd),
}

impl Cmd {
    pub fn run(&self) -> Result<(), Error> {
        match self {
            Cmd::New(cmd) => cmd.run(),
            Cmd::Autofill(cmd) => cmd.run(),
            Cmd::Batch(cmd) => cmd.run(),
            Cmd::Edit(cmd) => cmd.run(),
            Cmd::Sign(cmd) => cmd.run(),
            Cmd::Merge(cmd) => cmd.run(),
            Cmd::Submit(cmd) => cmd.run(),
            Cmd::Hash(cmd) => cmd.run(),
            Cmd::Digest(cmd) => cmd.run(),
            Cmd::Blob(cmd) => cmd.run(),
            Cmd::Decode(cmd) => cmd.run(),
            Cmd::MptIssuanceId(cmd) => cmd.run(),
            Cmd::Fields(cmd) => cmd.run(),
        }
    }
}

/// `SigningPubKey` must be present and exactly empty on an unsigned
/// transaction.
///
/// Absent and `""` are different bytes in the STObject — `7300` versus nothing —
/// so two signers that disagree about it sign two different digests. Nothing
/// catches that locally; it surfaces as `tefBAD_SIGNATURE` on a ceremony that
/// cannot be redone.
pub const EMPTY_SIGNING_PUB_KEY: &str = "";
