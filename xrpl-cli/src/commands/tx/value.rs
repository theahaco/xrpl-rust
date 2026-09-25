//! Turning what someone typed into what the ledger expects.
//!
//! Each field's serialization type decides how its value is parsed. Getting this
//! wrong is not a cosmetic problem: XRPL encodes an XRP amount as a decimal
//! string and an issued amount as an object, so a JSON number where a string
//! belongs serializes to different bytes, and the node rejects it — or worse,
//! accepts something else.

use serde_json::{json, Map, Value};

use crate::error::Error;

/// Looks up a local account alias for an address field: `(field, alias)` to
/// the record's address, or `Ok(None)` when there is no such account.
pub type Lookup<'a> = dyn Fn(&str, &str) -> Result<Option<String>, Error> + 'a;

/// Parse a typed field value.
pub fn parse(
    field: &str,
    serialization_type: &str,
    raw: &str,
    lookup: &Lookup,
) -> Result<Value, Error> {
    match serialization_type {
        "AccountID" => parse_account(field, raw, lookup).map(Value::String),
        "Amount" => parse_amount(field, raw, lookup),
        "UInt8" | "UInt16" | "UInt32" => parse_unsigned(field, raw),
        // UInt64 is string-encoded on the wire, and several fields are base-10
        // rather than hex. Passing the digits through as a string is right for
        // both, and a caller who needs the other spelling can say so.
        "UInt64" => Ok(json!(raw.replace('_', ""))),
        "Blob" => parse_blob(field, raw),
        "STArray" | "STObject" | "Vector256" | "PathSet" | "Issue" | "XChainBridge" => {
            parse_json(field, raw)
        }
        // Hashes, currencies, and anything this build does not model: a string,
        // or JSON if it looks like JSON.
        _ => Ok(parse_json(field, raw).unwrap_or_else(|_| Value::String(raw.to_string()))),
    }
}

/// An address field: a literal address, or the alias of a local account.
///
/// A valid address always wins, so no record can shadow one. Anything else must
/// name a record: an unknown name is refused here rather than sent, because the
/// codec would reject it later without saying which flag it came from.
fn parse_account(field: &str, raw: &str, lookup: &Lookup) -> Result<String, Error> {
    if xrpl::core::addresscodec::is_valid_classic_address(raw)
        || xrpl::core::addresscodec::is_valid_xaddress(raw)
    {
        return Ok(raw.to_string());
    }

    lookup(field, raw)?.ok_or_else(|| {
        Error::other(format!(
            "--{field}: {raw:?} is neither a valid XRPL address nor a local account alias. \
             `xrpl account ls` lists the aliases."
        ))
    })
}

/// Parse an amount in one of its three shapes.
///
/// - `10000000` — XRP, in drops. Underscores are stripped, so `10_000_000` works.
/// - `100/USD/rIssuer…` — an issued currency. The issuer may be an alias.
/// - `100/<48-hex>` — an MPT amount.
/// - `{...}` — the escape hatch, passed through as JSON.
fn parse_amount(field: &str, raw: &str, lookup: &Lookup) -> Result<Value, Error> {
    let raw = raw.trim();

    if raw.starts_with('{') {
        return parse_json(field, raw);
    }

    let parts: Vec<&str> = raw.split('/').collect();
    match parts.as_slice() {
        [drops] => {
            let digits = drops.replace('_', "");
            digits.parse::<u64>().map_err(|_| {
                Error::other(format!(
                    "--{field}: {raw:?} is not an amount. Expected drops (10000000), \
                     an issued amount (100/USD/rIssuer…), an MPT amount (100/<48-hex>), \
                     or a JSON object."
                ))
            })?;
            // Drops are a decimal *string* on the wire. A JSON number here
            // serializes to different bytes.
            Ok(Value::String(digits))
        }
        [value, issuance_id] if issuance_id.len() == 48 => Ok(json!({
            "mpt_issuance_id": issuance_id,
            "value": value,
        })),
        [value, currency, issuer] => {
            let issuer = parse_account(field, issuer, lookup)?;
            Ok(json!({
                "currency": currency,
                "issuer": issuer,
                "value": value,
            }))
        }
        _ => Err(Error::other(format!("--{field}: {raw:?} is not an amount"))),
    }
}

fn parse_unsigned(field: &str, raw: &str) -> Result<Value, Error> {
    raw.replace('_', "")
        .parse::<u64>()
        .map(|number| json!(number))
        .map_err(|_| Error::other(format!("--{field}: {raw:?} is not a number")))
}

/// Parse a blob: hex as given, or `@path` to read and hex-encode a file.
fn parse_blob(field: &str, raw: &str) -> Result<Value, Error> {
    if let Some(path) = raw.strip_prefix('@') {
        let contents = std::fs::read(path)
            .map_err(|error| Error::other(format!("--{field}: cannot read {path}: {error}")))?;
        return Ok(Value::String(hex::encode_upper(contents)));
    }

    Ok(Value::String(raw.to_string()))
}

fn parse_json(field: &str, raw: &str) -> Result<Value, Error> {
    serde_json::from_str(raw)
        .map_err(|error| Error::other(format!("--{field}: {raw:?} is not valid JSON: {error}")))
}

/// Build a `SignerEntries` array from repeated `ADDRESS:WEIGHT` arguments. The
/// address may be an alias.
///
/// The canonical spelling is `--signer-entry`: an on-ledger `SignerEntry` and a
/// local signing backend are different nouns, and `--signer` sitting one letter
/// from `--sign-with` is a mistake waiting to be made.
pub fn parse_signer_entries(entries: &[String], lookup: &Lookup) -> Result<Value, Error> {
    let mut array = Vec::with_capacity(entries.len());

    for entry in entries {
        let (address, weight) = entry.rsplit_once(':').ok_or_else(|| {
            Error::other(format!(
                "--signer-entry expects ADDRESS:WEIGHT, got {entry:?}"
            ))
        })?;

        let address = parse_account("signer-entry", address, lookup)?;
        let weight: u16 = weight
            .parse()
            .map_err(|_| Error::other(format!("--signer-entry: {weight:?} is not a weight")))?;

        let mut signer = Map::new();
        signer.insert("Account".into(), Value::String(address));
        signer.insert("SignerWeight".into(), json!(weight));

        let mut wrapper = Map::new();
        wrapper.insert("SignerEntry".into(), Value::Object(signer));
        array.push(Value::Object(wrapper));
    }

    Ok(Value::Array(array))
}

/// Build a `Memos` array from repeated `--memo` arguments.
pub fn parse_memos(memos: &[String]) -> Value {
    let array: Vec<Value> = memos
        .iter()
        .map(|memo| {
            let mut inner = Map::new();
            inner.insert(
                "MemoData".into(),
                Value::String(hex::encode_upper(memo.as_bytes())),
            );

            let mut wrapper = Map::new();
            wrapper.insert("Memo".into(), Value::Object(inner));
            Value::Object(wrapper)
        })
        .collect();

    Value::Array(array)
}

#[cfg(test)]
mod tests {
    use super::*;

    const ISSUER: &str = "r9cZA1mLK5R5Am25ArfXFmqgNwjZgnfk59";

    /// No local accounts at all.
    fn literal(_field: &str, _alias: &str) -> Result<Option<String>, Error> {
        Ok(None)
    }

    /// One local account, `alice`, at `ISSUER`.
    fn book(_field: &str, alias: &str) -> Result<Option<String>, Error> {
        Ok((alias == "alice").then(|| ISSUER.to_string()))
    }

    #[test]
    fn test_drops_are_a_string_not_a_number() {
        // The whole reason this function exists.
        assert_eq!(
            parse("amount", "Amount", "10000000", &literal).unwrap(),
            json!("10000000")
        );
    }

    #[test]
    fn test_underscores_are_stripped_from_drops() {
        assert_eq!(
            parse("amount", "Amount", "10_000_000", &literal).unwrap(),
            json!("10000000")
        );
    }

    #[test]
    fn test_an_issued_amount_becomes_an_object() {
        let amount = parse("amount", "Amount", &format!("100/USD/{ISSUER}"), &literal).unwrap();

        assert_eq!(amount["currency"], json!("USD"));
        assert_eq!(amount["issuer"], json!(ISSUER));
        assert_eq!(amount["value"], json!("100"));
    }

    #[test]
    fn test_an_mpt_amount_is_recognised_by_its_id_length() {
        let id = "00000123456789ABCDEF0123456789ABCDEF0123456789AB";
        let amount = parse("amount", "Amount", &format!("100/{id}"), &literal).unwrap();

        assert_eq!(amount["mpt_issuance_id"], json!(id));
        assert_eq!(amount["value"], json!("100"));
    }

    #[test]
    fn test_a_json_amount_passes_through() {
        let amount = parse(
            "amount",
            "Amount",
            r#"{"currency":"EUR","issuer":"rX","value":"1"}"#,
            &literal,
        )
        .unwrap();
        assert_eq!(amount["currency"], json!("EUR"));
    }

    #[test]
    fn test_a_nonsense_amount_names_the_shapes_it_accepts() {
        let message = parse("amount", "Amount", "not-an-amount", &literal)
            .unwrap_err()
            .to_string();

        assert!(message.contains("drops"), "{message}");
        assert!(message.contains("MPT"), "{message}");
    }

    #[test]
    fn test_an_issued_amount_validates_its_issuer() {
        assert!(parse("amount", "Amount", "100/USD/not-an-address", &literal).is_err());
    }

    #[test]
    fn test_a_literal_address_is_never_looked_up() {
        let lookup = |_: &str, _: &str| -> Result<Option<String>, Error> {
            panic!("a valid address must not reach the lookup")
        };

        assert_eq!(
            parse("destination", "AccountID", ISSUER, &lookup).unwrap(),
            json!(ISSUER)
        );
    }

    #[test]
    fn test_an_alias_becomes_its_address() {
        assert_eq!(
            parse("destination", "AccountID", "alice", &book).unwrap(),
            json!(ISSUER)
        );
    }

    #[test]
    fn test_an_unknown_alias_is_refused_naming_the_field() {
        let message = parse("destination", "AccountID", "bob", &book)
            .unwrap_err()
            .to_string();

        assert!(message.contains("--destination"), "{message}");
        assert!(message.contains("\"bob\""), "{message}");
    }

    #[test]
    fn test_an_issuer_may_be_an_alias() {
        let amount = parse("amount", "Amount", "100/USD/alice", &book).unwrap();

        assert_eq!(amount["issuer"], json!(ISSUER));
    }

    #[test]
    fn test_a_signer_entry_may_be_an_alias() {
        let array = parse_signer_entries(&["alice:3".to_string()], &book).expect("builds");

        assert_eq!(array[0]["SignerEntry"]["Account"], json!(ISSUER));
        assert_eq!(array[0]["SignerEntry"]["SignerWeight"], json!(3));
    }

    #[test]
    fn test_numbers_stay_numbers() {
        assert_eq!(parse("flags", "UInt32", "98", &literal).unwrap(), json!(98));
        assert_eq!(
            parse("tag", "UInt32", "12_345", &literal).unwrap(),
            json!(12345)
        );
        assert!(parse("tag", "UInt32", "not-a-number", &literal).is_err());
    }

    #[test]
    fn test_a_uint64_stays_a_string() {
        // UInt64 is string-encoded on the wire; a JSON number loses precision
        // above 2^53 in anything that reads it back with a JSON parser.
        assert_eq!(
            parse("maximum-amount", "UInt64", "18446744073709551615", &literal).unwrap(),
            json!("18446744073709551615")
        );
    }

    #[test]
    fn test_a_blob_can_be_read_from_a_file() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("metadata.json");
        std::fs::write(&path, br#"{"name":"token"}"#).expect("write");

        let blob = parse(
            "mptoken-metadata",
            "Blob",
            &format!("@{}", path.display()),
            &literal,
        )
        .unwrap();
        let hex_value = blob.as_str().expect("hex");

        assert_eq!(
            String::from_utf8(hex::decode(hex_value).expect("decodes")).expect("utf8"),
            r#"{"name":"token"}"#
        );
    }

    #[test]
    fn test_signer_entries_build_the_array_shape_the_ledger_wants() {
        let entries = vec![format!("{ISSUER}:2")];
        let array = parse_signer_entries(&entries, &literal).expect("builds");

        assert_eq!(array[0]["SignerEntry"]["Account"], json!(ISSUER));
        assert_eq!(array[0]["SignerEntry"]["SignerWeight"], json!(2));
    }

    #[test]
    fn test_a_signer_entry_without_a_weight_is_a_usage_error() {
        assert!(parse_signer_entries(&[ISSUER.to_string()], &literal).is_err());
    }

    #[test]
    fn test_memos_are_hex_encoded() {
        let memos = parse_memos(&["mint-period=2026".to_string()]);
        let data = memos[0]["Memo"]["MemoData"].as_str().expect("data");

        assert_eq!(
            String::from_utf8(hex::decode(data).expect("decodes")).expect("utf8"),
            "mint-period=2026"
        );
    }
}
