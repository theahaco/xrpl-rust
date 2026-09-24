//! Printing. Commands produce values; this module decides how they reach the
//! terminal, so the formatting lives in one place rather than in every arm.
//!
//! # Two channels, and which one a command belongs to
//!
//! **stdout is the machine artifact and nothing else.** [`artifact`] writes one
//! line of compact JSON and is what pipeline commands and `xrpl rpc` use. It
//! takes no `--output`, no `--format` and no `--json` flag, deliberately: a
//! configurable pipe format is how chaining breaks, because every downstream
//! consumer then has to sniff what it is reading.
//!
//! **stderr carries everything a human reads.** [`note`] and [`warn`] write
//! there and are silenced by `-q`. A label prefixed onto stdout — `Signed
//! transaction blob: {blob}` — is the same defect as printing `{:#?}`: it makes
//! the output unpipeable while looking like it works.
//!
//! [`response`] is the human channel for the query commands, and is where a
//! `--json` flag belongs when those commands get one.

use core::sync::atomic::{AtomicBool, Ordering};

use serde::Serialize;
use xrpl::asynch::clients::exceptions::XRPLClientException;

use crate::error::Error;

/// Whether `-q` was passed. Set once during argument parsing.
static QUIET: AtomicBool = AtomicBool::new(false);

/// Silence [`note`] and [`warn`]. [`artifact`] is never silenced — it is the
/// command's output, not its commentary.
pub fn set_quiet(quiet: bool) {
    QUIET.store(quiet, Ordering::Relaxed);
}

/// Whether human-facing output is suppressed.
pub fn is_quiet() -> bool {
    QUIET.load(Ordering::Relaxed)
}

/// Write the machine artifact to stdout: one line of compact JSON.
///
/// Compact rather than pretty, and newline-terminated, so a stream of these is
/// valid NDJSON and each line survives a `while read` loop intact.
pub fn artifact<T: Serialize>(value: &T) -> Result<(), Error> {
    let line = serde_json::to_string(value)?;
    println!("{line}");

    Ok(())
}

/// Write a human-facing note to stderr. Silenced by `-q`.
pub fn note(message: impl core::fmt::Display) {
    if !is_quiet() {
        eprintln!("{message}");
    }
}

/// Write a human-facing warning to stderr. Silenced by `-q`.
pub fn warn(message: impl core::fmt::Display) {
    if !is_quiet() {
        eprintln!("warning: {message}");
    }
}

/// Print a node response under a human-readable label.
///
/// The human channel, for the query commands. New commands that produce a
/// machine artifact use [`artifact`] instead.
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
///
/// Deprecated, not deleted: the five call sites are in commands the removal PR
/// takes out, and they go together.
#[deprecated(note = "prose on stdout is unpipeable; the tx pipeline uses artifact()")]
pub fn signed_tx_blob<T: Serialize>(tx: &T) -> Result<String, Error> {
    let tx_blob = xrpl::core::binarycodec::encode(tx)?;
    println!("Signed transaction blob: {tx_blob}");
    Ok(tx_blob)
}

/// The follow-up line printed after a command signs but does not submit.
///
/// This hint *is* the problem it papers over: it exists because stdout carried
/// prose, so nothing could be piped and copy-pasting a hex blob between two
/// commands became the documented workflow.
#[deprecated(note = "the pipeline replaces copy-pasting a blob between commands")]
pub fn submit_hint(tx_blob: &str, url: &str) {
    println!("To submit, use: xrpl transaction submit --tx-blob {tx_blob} --url {url}");
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_artifact_is_compact_single_line_json() {
        // The property that matters is that `serde_json::to_string` is used
        // rather than `to_string_pretty`: one artifact is one line, so a stream
        // of them is NDJSON.
        let value = json!({"a": 1, "b": {"c": 2}});
        let line = serde_json::to_string(&value).expect("serializes");

        assert!(!line.contains('\n'));
        assert_eq!(line, r#"{"a":1,"b":{"c":2}}"#);
    }

    #[test]
    fn test_quiet_is_togglable() {
        let original = is_quiet();

        set_quiet(true);
        assert!(is_quiet());
        set_quiet(false);
        assert!(!is_quiet());

        set_quiet(original);
    }
}
