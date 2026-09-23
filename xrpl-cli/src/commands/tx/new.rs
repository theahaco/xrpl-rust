//! `xrpl tx new <Type> --field K=V` — build an unsigned transaction, offline.
//!
//! The `--field` form is deliberately temporary. A generated command surface,
//! driven by the `definitions.json` the library vendors, replaces it with a real
//! flag per protocol field; this exists so the pipe contract can be proved and
//! tested before that generator lands, and it is the only code in the epic
//! expected to be thrown away.

use serde_json::{Map, Value};

use crate::commands::tx::{io, EMPTY_SIGNING_PUB_KEY};
use crate::error::Error;

#[derive(Debug, Clone, clap::Args)]
pub struct Cmd {
    /// The transaction type, e.g. `Payment` or `AccountSet`.
    #[arg(value_name = "TYPE")]
    pub tx_type: String,

    /// A transaction field, as `Name=Value`. Repeatable.
    ///
    /// The value is parsed as JSON when it parses and treated as a string
    /// otherwise, so `Amount=10000000` stays the string XRPL wants for drops
    /// while `Flags=98` becomes a number.
    #[arg(long = "field", value_name = "NAME=VALUE")]
    pub fields: Vec<String>,

    /// The sending account. Shorthand for `--field Account=…`.
    #[arg(long)]
    pub account: Option<String>,
}

impl Cmd {
    pub fn run(&self) -> Result<(), Error> {
        let transaction = self.build()?;
        io::write_txs(&[transaction])
    }

    fn build(&self) -> Result<Value, Error> {
        let mut object = Map::new();
        object.insert(
            "TransactionType".into(),
            Value::String(self.tx_type.clone()),
        );

        if let Some(account) = &self.account {
            object.insert("Account".into(), Value::String(account.clone()));
        }

        for entry in &self.fields {
            let (name, value) = entry.split_once('=').ok_or_else(|| {
                Error::other(format!("--field expects NAME=VALUE, got {entry:?}"))
            })?;

            if name.is_empty() {
                return Err(Error::other(format!(
                    "--field has an empty name: {entry:?}"
                )));
            }

            object.insert(name.to_string(), parse_field_value(name, value));
        }

        // Always, and unconditionally. See `EMPTY_SIGNING_PUB_KEY`.
        object.insert(
            "SigningPubKey".into(),
            Value::String(EMPTY_SIGNING_PUB_KEY.into()),
        );

        Ok(Value::Object(object))
    }
}

/// Parse a field value, keeping XRPL's string-typed fields as strings.
///
/// `Amount` in drops, `Fee`, and the various `Sequence`-adjacent fields are
/// string-encoded on the wire even though they are numbers, so a bare
/// `Amount=10000000` must not become a JSON number.
fn parse_field_value(name: &str, value: &str) -> Value {
    const ALWAYS_STRING: &[&str] = &[
        "Amount",
        "Fee",
        "SendMax",
        "DeliverMin",
        "TakerPays",
        "TakerGets",
    ];

    if ALWAYS_STRING.contains(&name) && value.parse::<u64>().is_ok() {
        return Value::String(value.to_string());
    }

    serde_json::from_str::<Value>(value).unwrap_or_else(|_| Value::String(value.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn build(tx_type: &str, fields: &[&str]) -> Value {
        Cmd {
            tx_type: tx_type.into(),
            fields: fields.iter().map(|f| f.to_string()).collect(),
            account: None,
        }
        .build()
        .expect("builds")
    }

    #[test]
    fn test_signing_pub_key_is_always_present_and_empty() {
        // Absent and "" are different bytes; the transaction must commit to one.
        let tx = build("Payment", &[]);
        assert_eq!(tx["SigningPubKey"], json!(""));
    }

    #[test]
    fn test_drops_stay_a_string() {
        // XRPL encodes an XRP amount as a decimal string. A JSON number here
        // serializes to different bytes and the node rejects it.
        let tx = build("Payment", &["Amount=10000000"]);
        assert_eq!(tx["Amount"], json!("10000000"));
    }

    #[test]
    fn test_flags_become_a_number() {
        let tx = build("MPTokenIssuanceCreate", &["Flags=98"]);
        assert_eq!(tx["Flags"], json!(98));
    }

    #[test]
    fn test_an_issued_currency_amount_stays_an_object() {
        let tx = build(
            "Payment",
            &[r#"Amount={"currency":"USD","issuer":"rIssuer","value":"100"}"#],
        );
        assert_eq!(tx["Amount"]["currency"], json!("USD"));
    }

    #[test]
    fn test_account_flag_is_the_account_field() {
        let tx = Cmd {
            tx_type: "Payment".into(),
            fields: Vec::new(),
            account: Some("rAlice".into()),
        }
        .build()
        .expect("builds");

        assert_eq!(tx["Account"], json!("rAlice"));
    }

    #[test]
    fn test_a_field_without_equals_is_a_usage_error() {
        let result = Cmd {
            tx_type: "Payment".into(),
            fields: vec!["Amount".into()],
            account: None,
        }
        .build();

        assert!(result.is_err());
    }
}
