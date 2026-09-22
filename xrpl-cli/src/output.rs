//! Printing. Commands produce values; this module decides how they reach the
//! terminal, so the formatting lives in one place rather than in every arm.

use serde::Serialize;
use xrpl::asynch::clients::exceptions::XRPLClientException;

use crate::error::Error;

/// Print a node response under a human-readable label.
pub fn response<T: core::fmt::Debug>(
    result: Result<T, XRPLClientException>,
    label: &str,
) -> Result<(), Error> {
    match result {
        Ok(response) => {
            println!("{label}: {response:#?}");
            Ok(())
        }
        Err(error) => Err(Error::Client(error)),
    }
}

/// Serialize a signed transaction to its binary blob, print it, and hand it
/// back so the caller can tell the user how to submit it.
pub fn signed_tx_blob<T: Serialize>(tx: &T) -> Result<String, Error> {
    let tx_blob = xrpl::core::binarycodec::encode(tx)?;
    println!("Signed transaction blob: {tx_blob}");
    Ok(tx_blob)
}

/// The follow-up line printed after a command signs but does not submit.
pub fn submit_hint(tx_blob: &str, url: &str) {
    println!("To submit, use: xrpl transaction submit --tx-blob {tx_blob} --url {url}");
}
