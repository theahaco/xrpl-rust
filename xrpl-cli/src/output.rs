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
//! [`response`] is the query commands' channel. Its output is **always JSON on
//! stdout** — indented by default because a person is usually reading it,
//! compact under `--json` because a script usually is not. Both parse, so `jq`
//! works either way and the flag changes only how many newlines there are. The
//! label that used to be prefixed onto stdout is a [`note`] on stderr.

use core::sync::atomic::{AtomicBool, Ordering};

use serde::Serialize;
use serde_json::Value;

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
    write_line(&line)
}

/// Write one line to stdout, treating a closed pipe as a normal ending.
///
/// `println!` panics when the write fails, so `xrpl tx new … | head -1` ended
/// in a Rust panic message rather than quietly — `head` closes the pipe as soon
/// as it has what it asked for, which is the reader doing its job, not an error.
/// Rust ignores `SIGPIPE`, so the failure arrives here as an ordinary write
/// error and this is where it belongs.
fn write_line(line: &str) -> Result<(), Error> {
    use std::io::Write;

    match writeln!(std::io::stdout(), "{line}") {
        Ok(()) => Ok(()),
        // Nothing is listening any more, and everything asked for was
        // delivered. Exiting is the whole remaining task.
        Err(error) if error.kind() == std::io::ErrorKind::BrokenPipe => std::process::exit(0),
        Err(error) => Err(Error::Io(error)),
    }
}

/// Write one already-rendered line to stdout.
///
/// For `server subscribe`, which passes the node's own frame through rather
/// than re-serializing it.
pub fn raw_line(line: &str) -> Result<(), Error> {
    write_line(line)
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

/// Print a node's `result` as JSON on stdout, and its label on stderr.
///
/// Takes a value rather than a `Result`: a printer that accepts an error
/// inverts control — the caller hands its failure to the formatter instead of
/// the formatter receiving something to format — and it meant every call site
/// could only report an error the one way this function chose.
///
/// `compact` is `--json`. The default is indented, which is still one JSON
/// document: the flag chooses a shape a script prefers, never a *format* a
/// consumer has to sniff. That is the same reasoning that keeps `--json` off
/// the `tx` pipeline stages entirely — there, one shape is the contract.
pub fn response(value: &Value, label: &str, compact: bool) -> Result<(), Error> {
    note(label);

    let rendered = if compact {
        serde_json::to_string(value)
    } else {
        serde_json::to_string_pretty(value)
    }
    .map_err(Error::Json)?;

    write_line(&rendered)
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
