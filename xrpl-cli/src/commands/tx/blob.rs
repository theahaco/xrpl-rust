//! `xrpl tx blob` — render a transaction as a hex blob. Offline.
//!
//! A terminal render, not a pipe format. Nothing downstream consumes hex except
//! `tx decode`, which exists to undo this.

use xrpl::core::binarycodec::encode;

use crate::commands::tx::args::TxInput;
use crate::commands::tx::io;
use crate::error::Error;

#[derive(Debug, Clone, clap::Args)]
pub struct Cmd {
    #[command(flatten)]
    pub input: TxInput,
}

impl Cmd {
    pub fn run(&self) -> Result<(), Error> {
        for transaction in io::read_txs(&self.input.tx)? {
            crate::output::artifact(&encode(&transaction)?)?;
        }

        Ok(())
    }
}
