//! `xrpl tx edit` — open the transaction in an editor, mid-pipe. (OFFLINE)
//!
//! The escape hatch for everything the builder does not model. `tx new` covers
//! the generated field surface, but a transaction is JSON and occasionally the
//! quickest correct move is to look at it and change one thing.
//!
//! It sits in the pipe like every other stage:
//!
//! ```text
//! xrpl tx new Payment --account r… | xrpl tx edit | xrpl tx autofill --url …
//! ```
//!
//! which is only possible because the editor's descriptors are bound to
//! `/dev/tty` rather than inherited — stdin carries the transaction and stdout
//! carries the result, so neither is free. That rebinding lives in
//! [`crate::tty`], shared with the passphrase prompt, because two
//! implementations of it would diverge.

use serde_json::Value;

use crate::commands::tx::args::TxInput;
use crate::commands::tx::io;
use crate::error::Error;
use crate::output;
use crate::tty;

#[derive(Debug, Clone, clap::Args)]
pub struct Cmd {
    #[command(flatten)]
    pub input: TxInput,
}

impl Cmd {
    pub fn run(&self) -> Result<(), Error> {
        let before = io::read_txs(&self.input.tx)?;

        // Said before the editor opens, not after: afterwards the edit has
        // already been made and the warning is a postmortem.
        if let Some(what) = signature_fields(&before) {
            output::warn(format!(
                "this transaction carries {what}. Editing it invalidates \
                 {}, and the failure surfaces as tefBAD_SIGNATURE at submit time.",
                if before.len() == 1 { "them" } else { "those" }
            ));
        }

        let edited = tty::edit(&render(&before)?, ".json")?;
        let after = io::parse_stream(&edited)?;

        if after == before {
            output::note("unchanged");
        }

        io::write_txs(&after)
    }
}

/// Render the stream for a human to edit.
///
/// Pretty-printed, unlike everything else this CLI writes: it is going into an
/// editor rather than into a pipe. A stream becomes a JSON array so the shape
/// survives the round trip — `parse_stream` reads an array back as a stream.
fn render(transactions: &[Value]) -> Result<String, Error> {
    let value = match transactions {
        [one] => one.clone(),
        many => Value::Array(many.to_vec()),
    };

    serde_json::to_string_pretty(&value)
        .map(|rendered| format!("{rendered}\n"))
        .map_err(Error::Json)
}

/// Name the signature fields present, for the warning.
fn signature_fields(transactions: &[Value]) -> Option<&'static str> {
    let signed = transactions
        .iter()
        .any(|tx| tx.get("TxnSignature").is_some_and(|v| !v.is_null()));
    let multisigned = transactions.iter().any(|tx| {
        tx.get("Signers")
            .and_then(Value::as_array)
            .is_some_and(|signers| !signers.is_empty())
    });

    match (signed, multisigned) {
        (true, true) => Some("a signature and collected signers"),
        (true, false) => Some("a signature"),
        (false, true) => Some("collected signers"),
        (false, false) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_one_transaction_renders_as_an_object() {
        let rendered = render(&[json!({"TransactionType": "Payment"})]).expect("renders");

        assert!(rendered.starts_with('{'), "{rendered}");
        // Pretty-printed, because it is going into an editor rather than a pipe.
        assert!(rendered.contains("\n  \"TransactionType\""), "{rendered}");
    }

    #[test]
    fn test_a_stream_renders_as_an_array_and_reads_back_as_a_stream() {
        let before = vec![json!({"Account": "rA"}), json!({"Account": "rB"})];

        let rendered = render(&before).expect("renders");
        assert!(rendered.starts_with('['), "{rendered}");

        // The round trip is the point: what the editor saves has to parse back
        // into the same stream the next stage expects.
        assert_eq!(io::parse_stream(&rendered).expect("parses"), before);
    }

    #[test]
    fn test_a_single_transaction_round_trips_too() {
        let before = vec![json!({"TransactionType": "Payment", "Account": "rA"})];
        let rendered = render(&before).expect("renders");

        assert_eq!(io::parse_stream(&rendered).expect("parses"), before);
    }

    #[test]
    fn test_an_unsigned_transaction_needs_no_warning() {
        assert_eq!(
            signature_fields(&[json!({"TransactionType": "Payment"})]),
            None
        );

        // An unsigned multisign candidate has the field, empty. Warning about
        // it would train people to ignore the warning.
        assert_eq!(signature_fields(&[json!({"Signers": []})]), None);
    }

    #[test]
    fn test_a_signed_transaction_is_named_as_signed() {
        assert_eq!(
            signature_fields(&[json!({"TxnSignature": "3045…"})]),
            Some("a signature")
        );
    }

    #[test]
    fn test_collected_signers_are_named_as_signers() {
        let tx = json!({"Signers": [{"Signer": {"Account": "rA"}}]});
        assert_eq!(signature_fields(&[tx]), Some("collected signers"));
    }

    #[test]
    fn test_both_are_named_together() {
        let tx = json!({"TxnSignature": "30…", "Signers": [{"Signer": {"Account": "rA"}}]});
        assert_eq!(
            signature_fields(&[tx]),
            Some("a signature and collected signers")
        );
    }

    #[test]
    fn test_one_signed_transaction_in_a_stream_is_enough() {
        let stream = vec![json!({"Account": "rA"}), json!({"TxnSignature": "30…"})];
        assert_eq!(signature_fields(&stream), Some("a signature"));
    }
}
