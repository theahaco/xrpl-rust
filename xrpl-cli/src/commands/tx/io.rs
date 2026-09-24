//! Reading transactions in and writing them out.
//!
//! The pipe carries **one line of compact transaction JSON per transaction**.
//! Not hex: XRPL has no unsigned-envelope encoding, so a hex blob between stages
//! would be a format this project invented, opaque to `jq` and unable to say
//! whether a ceremony has one signature or three. Hex is a terminal render,
//! produced only by `tx blob` and consumed only by `tx decode`.
//!
//! Every stage reads and writes `Vec<Value>`. One transaction is the degenerate
//! case of a stream; costing nothing now is what makes batching later a matter
//! of iterating rather than redesigning.

use std::ffi::OsString;
use std::io::{IsTerminal, Read};
use std::path::Path;

use serde_json::Value;

use crate::error::Error;

/// Read transactions from a positional argument, a file, or stdin.
///
/// In order: a literal that parses as JSON, then a path that exists, then stdin
/// (for `-`, or for nothing at all).
///
/// Literal-first matters for the error message. If a caller mistypes a path,
/// "no such file" is a better answer than a JSON parse error about the path
/// they typed.
pub fn read_txs(source: &Option<OsString>) -> Result<Vec<Value>, Error> {
    let raw = match source {
        Some(value) if value != "-" => {
            let text = value.to_string_lossy().into_owned();

            // Anything with a brace is being offered as JSON; if it does not
            // parse, say so rather than reporting it as a missing file.
            if text.trim_start().starts_with(['{', '[']) {
                text
            } else if Path::new(value).exists() {
                std::fs::read_to_string(value).map_err(Error::Io)?
            } else {
                return Err(Error::other(format!(
                    "no such file: {}. Pass transaction JSON, a path, or `-` for stdin.",
                    text
                )));
            }
        }
        _ => read_stdin()?,
    };

    parse_stream(&raw)
}

fn read_stdin() -> Result<String, Error> {
    if std::io::stdin().is_terminal() {
        // Nothing is piping anything in and nobody typed a path. Blocking here
        // looks like a hang; saying so is the whole difference.
        return Err(Error::other(
            "no transaction provided: pipe one in, pass a path, or give JSON as an argument",
        ));
    }

    let mut buffer = String::new();
    std::io::stdin()
        .read_to_string(&mut buffer)
        .map_err(Error::Io)?;

    Ok(buffer)
}

/// Parse NDJSON: one transaction per non-blank line.
///
/// Tolerates a single pretty-printed object too, because a human pasting one in
/// by hand is a normal thing to do and failing on it teaches nothing.
pub fn parse_stream(raw: &str) -> Result<Vec<Value>, Error> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(Error::other("no transaction provided: input was empty"));
    }

    let lines: Vec<&str> = trimmed
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect();

    // A pretty-printed object spans lines but is one value.
    if lines.len() > 1 {
        if let Ok(value) = serde_json::from_str::<Value>(trimmed) {
            return Ok(as_stream(value));
        }
    }

    let mut transactions = Vec::with_capacity(lines.len());
    for (index, line) in lines.iter().enumerate() {
        let value: Value = serde_json::from_str(line)
            .map_err(|error| Error::other(format!("line {}: {error}", index + 1)))?;
        transactions.extend(as_stream(value));
    }

    Ok(transactions)
}

/// A JSON array of transactions is a stream; anything else is one transaction.
fn as_stream(value: Value) -> Vec<Value> {
    match value {
        Value::Array(items) => items,
        other => vec![other],
    }
}

/// Write transactions to stdout, one compact line each.
pub fn write_txs(transactions: &[Value]) -> Result<(), Error> {
    for transaction in transactions {
        crate::output::artifact(transaction)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_one_compact_line_is_one_transaction() {
        let parsed = parse_stream(r#"{"TransactionType":"Payment"}"#).expect("parses");

        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0]["TransactionType"], json!("Payment"));
    }

    #[test]
    fn test_ndjson_is_a_stream() {
        let raw = "{\"Account\":\"rA\"}\n{\"Account\":\"rB\"}\n{\"Account\":\"rC\"}\n";
        let parsed = parse_stream(raw).expect("parses");

        assert_eq!(parsed.len(), 3);
        assert_eq!(parsed[2]["Account"], json!("rC"));
    }

    #[test]
    fn test_blank_lines_are_ignored() {
        let raw = "\n{\"Account\":\"rA\"}\n\n  \n{\"Account\":\"rB\"}\n\n";
        assert_eq!(parse_stream(raw).expect("parses").len(), 2);
    }

    #[test]
    fn test_a_pretty_printed_object_is_still_one_transaction() {
        // A human pasting a formatted transaction in is normal; failing on it
        // teaches nothing about the format.
        let raw = "{\n  \"TransactionType\": \"Payment\",\n  \"Account\": \"rA\"\n}\n";
        let parsed = parse_stream(raw).expect("parses");

        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0]["Account"], json!("rA"));
    }

    #[test]
    fn test_a_json_array_is_a_stream() {
        let parsed = parse_stream(r#"[{"Account":"rA"},{"Account":"rB"}]"#).expect("parses");
        assert_eq!(parsed.len(), 2);
    }

    #[test]
    fn test_empty_input_names_the_problem() {
        let message = parse_stream("   \n  ").unwrap_err().to_string();
        assert!(message.contains("no transaction provided"), "{message}");
    }

    #[test]
    fn test_a_malformed_line_names_its_line_number() {
        let raw = "{\"Account\":\"rA\"}\nnot json\n";
        let message = parse_stream(raw).unwrap_err().to_string();

        assert!(message.contains("line 2"), "{message}");
    }

    #[test]
    fn test_a_literal_argument_is_read_as_json() {
        let source = Some(OsString::from(r#"{"TransactionType":"Payment"}"#));
        let parsed = read_txs(&source).expect("parses");

        assert_eq!(parsed[0]["TransactionType"], json!("Payment"));
    }

    #[test]
    fn test_a_file_argument_is_read_from_disk() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("tx.json");
        std::fs::write(&path, r#"{"Account":"rFromFile"}"#).expect("write");

        let parsed = read_txs(&Some(path.into_os_string())).expect("parses");
        assert_eq!(parsed[0]["Account"], json!("rFromFile"));
    }

    #[test]
    fn test_a_missing_path_says_so_rather_than_failing_to_parse() {
        let source = Some(OsString::from("./definitely-not-here.json"));
        let message = read_txs(&source).unwrap_err().to_string();

        assert!(message.contains("no such file"), "{message}");
    }
}
