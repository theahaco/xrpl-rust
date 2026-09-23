//! `xrpl tx digest` — the bytes a signer actually signs. Offline.
//!
//! This is what a ceremony quotes to its co-signers, **for comparison only**.
//! No stage anywhere accepts a digest as input, and `tx sign` always re-derives
//! it from the transaction JSON. That is what keeps it a terminal render rather
//! than a signing oracle: the rule that a signer must frame its own bytes
//! constrains what a *signer consumes*, not what a *renderer prints*.

use xrpl::core::binarycodec::{encode_for_multisigning, encode_for_signing};

use crate::commands::tx::args::TxInput;
use crate::commands::tx::io;
use crate::error::Error;

#[derive(Debug, Clone, clap::Args)]
pub struct Cmd {
    #[command(flatten)]
    pub input: TxInput,

    /// Render the multisign pre-image for this signer's address.
    ///
    /// A multisign digest is signer-specific — the signer's own AccountID is
    /// part of the bytes — so there is no single digest a ceremony can share.
    /// Takes a literal r-address: this names which signer's pre-image to
    /// render, and is not an account or key selector.
    #[arg(long, value_name = "R_ADDRESS")]
    pub as_signer: Option<String>,
}

impl Cmd {
    pub fn run(&self) -> Result<(), Error> {
        for transaction in io::read_txs(&self.input.tx)? {
            let digest = match &self.as_signer {
                Some(address) => encode_for_multisigning(&transaction, address.into())?,
                None => encode_for_signing(&transaction)?,
            };

            crate::output::artifact(&digest)?;
        }

        Ok(())
    }
}
