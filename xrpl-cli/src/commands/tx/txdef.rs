//! The command surface, read from rippled's own definitions.
//!
//! `tx new` used to be a hand-written `match` over five transaction-type
//! strings, erroring for the other 77. The ledger's field, type and flag tables
//! are vendored in the library as `definitions.json`, so the surface is
//! generated from them instead: the marginal cost of a transaction type is zero
//! lines, and a definitions refresh shows up in `--help` without a rebuild of
//! anything hand-written.
//!
//! # What this costs
//!
//! No compile-time check on the generated arguments, runtime `clap` errors
//! rather than type errors, and no `grep` from a transaction type to a Rust
//! file. The mitigations are a checked-in surface snapshot diffed in CI and a
//! test that builds every command and asserts each required field is required.

use std::collections::BTreeMap;
use std::sync::OnceLock;

use serde_json::Value;

use crate::error::Error;

/// Fields the pipeline owns. Never generated as flags.
///
/// `SigningPubKey`, `TxnSignature` and `Signers` are written by the signing
/// stages; `TransactionType` is the subcommand name; `SponsorSignature` is a
/// non-signing field with no pre-image yet, reachable only through `--field`.
const PIPELINE_OWNED: &[&str] = &[
    "TransactionType",
    "SigningPubKey",
    "TxnSignature",
    "Signers",
    "SponsorSignature",
];

/// A ledger-entry field that appears in the common format but never on a
/// transaction a user builds.
const NOT_GENERATED: &[&str] = &["PreviousTxnID"];

/// Common fields that are `optionality: 0` in the definitions but must not be
/// required flags.
///
/// The `common` format entry marks `Account`, `Sequence`, `Fee` and
/// `SigningPubKey` all required. Taking that literally would make `--fee` and
/// `--sequence` mandatory on all 82 subcommands, which contradicts `tx autofill`
/// outright — the whole point of that stage is that a node supplies them.
///
/// `Account` is here now that the account store exists: it is
/// required-*after-resolution*, satisfiable by `--account`, `XRPL_ACCOUNT` or
/// the configured default. When none of the three resolve, the resolver is what
/// reports it, and it can say which sources it tried.
const NEVER_REQUIRED: &[&str] = &["Fee", "Sequence", "LastLedgerSequence", "Account"];

/// Acronyms that must not be split when a field name is kebab-cased.
///
/// `MPTokenIssuanceID` would otherwise become `--m-p-token-issuance-i-d`.
const ACRONYMS: &[(&str, &str)] = &[
    // Longest first: the first match wins, and these are prefix rules, so
    // `MPTokenIssuanceID` becomes `mptoken-` plus the kebab of the rest.
    ("UNLModify", "unlmodify"),
    ("XChain", "xchain"),
    ("MPToken", "mptoken"),
    ("NFToken", "nftoken"),
    ("AMM", "amm"),
    ("DID", "did"),
    ("URI", "uri"),
];

/// One field on a transaction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldDef {
    /// The protocol name, e.g. `Destination`.
    pub name: &'static str,
    /// The command-line spelling, e.g. `destination`.
    pub flag: &'static str,
    /// Its serialization type, which decides how a value is parsed.
    pub serialization_type: &'static str,
    /// Whether a value must be given.
    pub required: bool,
}

/// One transaction type.
#[derive(Debug, Clone)]
pub struct TransactionDef {
    /// The protocol name, e.g. `Payment`.
    pub name: &'static str,
    /// The subcommand spelling, e.g. `payment`.
    pub command: &'static str,
    /// Fields specific to this type, then the shared common fields.
    pub fields: Vec<FieldDef>,
    /// This type's flag names, mapped to their bits, plus the universal ones.
    pub flags: BTreeMap<&'static str, u32>,
}

/// Fields the command surface exposes through sugar rather than a raw flag.
///
/// Each is a real field with a real generated spelling, so leaving them in the
/// generated set would give clap two arguments of one name. `--field Flags=98`
/// still reaches them.
pub const SUGARED: &[&str] = &["Flags", "SignerEntries", "Memos", "SetFlag", "ClearFlag"];

impl TransactionDef {
    /// The fields that become flags of their own.
    ///
    /// One definition of the surface, used by both the argument builder and the
    /// snapshot renderer, so the two cannot disagree about what exists.
    pub fn generated_fields(&self) -> impl Iterator<Item = &FieldDef> {
        self.fields
            .iter()
            .filter(|field| !SUGARED.contains(&field.name))
    }

    /// Look up a field by its protocol name or its flag spelling.
    pub fn field(&self, name: &str) -> Option<&FieldDef> {
        self.fields
            .iter()
            .find(|field| field.name == name || field.flag == name)
    }
}

struct Tables {
    transactions: Vec<TransactionDef>,
    /// Every field name the ledger knows, for the strict-name check.
    known_fields: BTreeMap<&'static str, &'static str>,
    account_set_flags: BTreeMap<&'static str, u32>,
}

/// Parse the vendored definitions once.
///
/// `clap` requires `&'static str` for every name, and `Str: From<String>` is not
/// implemented, so the strings are leaked out of this one-time parse. Leaking
/// per call — inside `augment_subcommands`, which `clap` may call more than once
/// — would leak on every invocation.
fn tables() -> &'static Tables {
    static TABLES: OnceLock<Tables> = OnceLock::new();

    TABLES.get_or_init(|| {
        let raw: Value =
            serde_json::from_str(xrpl::core::binarycodec::definitions::DEFINITIONS_JSON)
                .expect("the vendored definitions.json parses");

        let known_fields = parse_fields(&raw);
        let common = parse_common(&raw, &known_fields);
        let flags = parse_flags(&raw);
        let universal = flags.get("universal").cloned().unwrap_or_default();

        let mut transactions = Vec::new();
        for (name, code) in raw["TRANSACTION_TYPES"]
            .as_object()
            .expect("TRANSACTION_TYPES is an object")
        {
            // `Invalid: -1` is a sentinel, not a transaction type.
            if code.as_i64().unwrap_or(-1) < 0 {
                continue;
            }

            let Some(format) = raw["TRANSACTION_FORMATS"]
                .get(name)
                .and_then(Value::as_array)
            else {
                continue;
            };

            let leaked_name: &'static str = Box::leak(name.clone().into_boxed_str());
            let mut fields: Vec<FieldDef> = format
                .iter()
                .filter_map(|entry| field_def(entry, &known_fields, false))
                .collect();

            // Common fields last, so a type's own field wins a name collision.
            for field in &common {
                if !fields.iter().any(|existing| existing.name == field.name) {
                    fields.push(field.clone());
                }
            }

            let mut type_flags = universal.clone();
            if let Some(own) = flags.get(name) {
                type_flags.extend(own.iter().map(|(k, v)| (*k, *v)));
            }

            transactions.push(TransactionDef {
                name: leaked_name,
                command: leak(kebab_case(leaked_name)),
                fields,
                flags: type_flags,
            });
        }

        transactions.sort_by_key(|tx| tx.name);

        Tables {
            transactions,
            known_fields,
            account_set_flags: parse_account_set_flags(&raw),
        }
    })
}

fn leak(value: String) -> &'static str {
    Box::leak(value.into_boxed_str())
}

fn parse_fields(raw: &Value) -> BTreeMap<&'static str, &'static str> {
    let mut known = BTreeMap::new();

    for entry in raw["FIELDS"].as_array().expect("FIELDS is an array") {
        let Some(pair) = entry.as_array() else {
            continue;
        };
        let (Some(name), Some(info)) = (pair.first().and_then(Value::as_str), pair.get(1)) else {
            continue;
        };
        let Some(serialization_type) = info["type"].as_str() else {
            continue;
        };

        known.insert(leak(name.to_string()), leak(serialization_type.to_string()));
    }

    known
}

fn parse_common(raw: &Value, known: &BTreeMap<&'static str, &'static str>) -> Vec<FieldDef> {
    raw["TRANSACTION_FORMATS"]["common"]
        .as_array()
        .map(|entries| {
            entries
                .iter()
                .filter_map(|entry| field_def(entry, known, true))
                .collect()
        })
        .unwrap_or_default()
}

fn field_def(
    entry: &Value,
    known: &BTreeMap<&'static str, &'static str>,
    is_common: bool,
) -> Option<FieldDef> {
    let name = entry["name"].as_str()?;

    if PIPELINE_OWNED.contains(&name) || NOT_GENERATED.contains(&name) {
        return None;
    }

    let (&name, &serialization_type) = known.get_key_value(name)?;
    let optionality = entry["optionality"].as_u64().unwrap_or(1);

    Some(FieldDef {
        name,
        flag: leak(kebab_case(name)),
        serialization_type,
        // `optionality: 0` is required, 1 is optional, 2 is defaulted. A common
        // field is never required as a flag: see `NEVER_REQUIRED`.
        required: optionality == 0 && !(is_common && NEVER_REQUIRED.contains(&name)),
    })
}

fn parse_flags(raw: &Value) -> BTreeMap<String, BTreeMap<&'static str, u32>> {
    let mut all = BTreeMap::new();

    if let Some(groups) = raw["TRANSACTION_FLAGS"].as_object() {
        for (transaction_type, names) in groups {
            let mut map = BTreeMap::new();
            if let Some(entries) = names.as_object() {
                for (flag, bit) in entries {
                    if let Some(bit) = bit.as_u64() {
                        map.insert(leak(flag.clone()), bit as u32);
                    }
                }
            }
            all.insert(transaction_type.clone(), map);
        }
    }

    all
}

fn parse_account_set_flags(raw: &Value) -> BTreeMap<&'static str, u32> {
    let mut map = BTreeMap::new();

    if let Some(entries) = raw["ACCOUNT_SET_FLAGS"].as_object() {
        for (flag, bit) in entries {
            if let Some(bit) = bit.as_u64() {
                map.insert(leak(flag.clone()), bit as u32);
            }
        }
    }

    map
}

/// Turn a protocol field name into its command-line spelling.
///
/// `Destination` → `destination`, `DestinationTag` → `destination-tag`, and the
/// acronym table handles the names that would otherwise shatter.
pub fn kebab_case(name: &str) -> String {
    for (protocol, spelling) in ACRONYMS {
        if name == *protocol {
            return (*spelling).to_string();
        }
        if let Some(rest) = name.strip_prefix(protocol) {
            return format!("{spelling}-{}", kebab_case(rest));
        }
    }

    // Split into words, keeping runs of capitals together. `CredentialIDs`
    // becomes `credential-ids` rather than `credential-i-ds`, and `EPrice`
    // becomes `eprice` rather than `e-price`: a single leading capital belongs
    // to the word that follows it.
    let chars: Vec<char> = name.chars().collect();
    let mut words: Vec<String> = Vec::new();
    let mut index = 0;

    while index < chars.len() {
        let start = index;

        if chars[index].is_uppercase() {
            // Take the whole run of capitals.
            while index < chars.len() && chars[index].is_uppercase() {
                index += 1;
            }

            // A run longer than one capital that is followed by lowercase has
            // handed the last capital to the next word: `XRPLedger` is `xrp` +
            // `ledger`. A trailing `s` is a plural of the acronym, not a new
            // word, so `IDs` stays whole.
            let followed_by_lowercase = index < chars.len() && chars[index].is_lowercase();
            let plural_s = followed_by_lowercase
                && chars[index] == 's'
                && chars.get(index + 1).is_none_or(|c| c.is_uppercase());

            if followed_by_lowercase && index - start > 1 && !plural_s {
                index -= 1;
            } else if plural_s {
                index += 1;
            }
        }

        // Take the lowercase tail of this word.
        while index < chars.len() && !chars[index].is_uppercase() {
            index += 1;
        }

        words.push(
            chars[start..index]
                .iter()
                .collect::<String>()
                .to_lowercase(),
        );
    }

    words.join("-")
}

/// Every transaction type that can be built.
pub fn transactions() -> &'static [TransactionDef] {
    &tables().transactions
}

/// One transaction type, by protocol name or command spelling.
pub fn transaction(name: &str) -> Option<&'static TransactionDef> {
    tables()
        .transactions
        .iter()
        .find(|tx| tx.name.eq_ignore_ascii_case(name) || tx.command == name)
}

/// The serialization type of a field the ledger knows about, if it knows it.
pub fn known_field(name: &str) -> Option<&'static str> {
    tables().known_fields.get(name).copied()
}

/// The `AccountSet` flag table, for `--set-flag` / `--clear-flag`.
pub fn account_set_flags() -> &'static BTreeMap<&'static str, u32> {
    &tables().account_set_flags
}

/// Suggest a field name close to one that was not recognized.
///
/// `--desination` producing a valid signed transaction that pays nobody is the
/// failure this exists to prevent: the binary codec skips any key it does not
/// know, silently, so a typo encodes to a transaction missing that field.
pub fn did_you_mean(transaction: &TransactionDef, name: &str) -> Option<&'static str> {
    let lower = name.to_lowercase();

    transaction
        .fields
        .iter()
        .map(|field| {
            (
                edit_distance(&lower, &field.flag.to_lowercase()),
                field.flag,
            )
        })
        .filter(|(distance, _)| *distance <= 3)
        .min_by_key(|(distance, _)| *distance)
        .map(|(_, flag)| flag)
}

fn edit_distance(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut previous: Vec<usize> = (0..=b.len()).collect();

    for (i, ca) in a.iter().enumerate() {
        let mut current = vec![i + 1];
        for (j, cb) in b.iter().enumerate() {
            let cost = usize::from(ca != cb);
            current.push(
                (previous[j + 1] + 1)
                    .min(current[j] + 1)
                    .min(previous[j] + cost),
            );
        }
        previous = current;
    }

    previous[b.len()]
}

/// Report an unknown field name, with the type's own vocabulary.
pub fn unknown_field(transaction: &TransactionDef, name: &str) -> Error {
    let suggestion = did_you_mean(transaction, name)
        .map(|flag| format!("\n  did you mean --{flag}?"))
        .unwrap_or_default();

    Error::other(format!(
        "{} has no field {name:?}{suggestion}\n  \
         `xrpl tx fields {}` lists them. Use --field NAME=VALUE for a field this \
         build's definitions.json does not carry, or --allow-unknown-fields.",
        transaction.name, transaction.name
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_every_real_transaction_type_is_present() {
        // 83 entries, one of which is the `Invalid: -1` sentinel.
        assert_eq!(transactions().len(), 82);
        assert!(transaction("Payment").is_some());
        assert!(transaction("payment").is_some());
        // Types with no Rust model are still buildable: that is the point.
        assert!(transaction("Batch").is_some());
        assert!(transaction("DelegateSet").is_some());
    }

    #[test]
    fn test_the_invalid_sentinel_is_not_a_subcommand() {
        assert!(transaction("Invalid").is_none());
    }

    #[test]
    fn test_pipeline_owned_fields_are_never_generated() {
        let payment = transaction("Payment").expect("Payment");

        for owned in PIPELINE_OWNED {
            assert!(
                payment.field(owned).is_none(),
                "{owned} must not be a flag: the pipeline writes it"
            );
        }
    }

    #[test]
    fn test_a_types_own_required_fields_are_required() {
        let payment = transaction("Payment").expect("Payment");

        assert!(payment.field("Destination").expect("Destination").required);
        assert!(payment.field("Amount").expect("Amount").required);
        assert!(!payment.field("SendMax").expect("SendMax").required);
    }

    #[test]
    fn test_common_fields_autofill_supplies_are_not_required() {
        // The `common` entry marks these `optionality: 0`. Taking that literally
        // makes --fee and --sequence mandatory on all 82 subcommands, which is
        // the opposite of what `tx autofill` is for.
        let payment = transaction("Payment").expect("Payment");

        for name in ["Fee", "Sequence", "LastLedgerSequence"] {
            let field = payment.field(name).unwrap_or_else(|| panic!("{name}"));
            assert!(!field.required, "{name} must not be a required flag");
        }

        // `Account` joined them once the store gave it somewhere else to come
        // from: the resolver reports a missing one, and names what it tried.
        assert!(!payment.field("Account").expect("Account").required);
    }

    #[test]
    fn test_common_fields_reach_every_type() {
        // A hardcoded list goes stale; `Delegate` is the field that catches it.
        let account_set = transaction("AccountSet").expect("AccountSet");

        assert!(account_set.field("Memos").is_some());
        assert!(account_set.field("Delegate").is_some());
    }

    #[test]
    fn test_flag_names_carry_the_universal_set() {
        let payment = transaction("Payment").expect("Payment");

        assert!(payment.flags.contains_key("tfFullyCanonicalSig"));
        assert!(payment.flags.contains_key("tfPartialPayment"));
    }

    #[test]
    fn test_kebab_case_handles_ordinary_names() {
        assert_eq!(kebab_case("Destination"), "destination");
        assert_eq!(kebab_case("DestinationTag"), "destination-tag");
        assert_eq!(kebab_case("LastLedgerSequence"), "last-ledger-sequence");
    }

    #[test]
    fn test_kebab_case_does_not_shatter_acronyms() {
        // Without the override this is `--m-p-token-issuance-i-d`.
        assert_eq!(kebab_case("MPTokenIssuanceID"), "mptoken-issuance-id");
        assert_eq!(kebab_case("NFTokenTaxon"), "nftoken-taxon");
        assert_eq!(kebab_case("URI"), "uri");
    }

    #[test]
    fn test_a_capital_run_stays_one_word() {
        // `credential-i-ds` was the first spelling this produced.
        assert_eq!(kebab_case("CredentialIDs"), "credential-ids");
        assert_eq!(kebab_case("NetworkID"), "network-id");
        // A single leading capital joins the word after it; `EPrice` is a real
        // AMM field and `--e-price` is the faithful reading of it.
        assert_eq!(kebab_case("EPrice"), "e-price");
    }

    #[test]
    fn test_a_typo_gets_a_suggestion() {
        let payment = transaction("Payment").expect("Payment");

        // The binary codec silently skips a key it does not know, so `--desination`
        // would otherwise encode to a perfectly valid transaction that pays nobody.
        assert_eq!(did_you_mean(payment, "desination"), Some("destination"));
        assert_eq!(did_you_mean(payment, "amout"), Some("amount"));
        assert_eq!(did_you_mean(payment, "completely-unrelated"), None);
    }

    #[test]
    fn test_account_set_flags_are_available() {
        assert!(account_set_flags().contains_key("asfDisableMaster"));
    }

    #[test]
    fn test_every_generated_flag_name_is_unique_within_its_type() {
        for transaction in transactions() {
            let mut seen = std::collections::BTreeSet::new();
            for field in &transaction.fields {
                assert!(
                    seen.insert(field.flag),
                    "{} generates --{} twice",
                    transaction.name,
                    field.flag
                );
            }
        }
    }

    #[test]
    fn test_no_generated_flag_collides_with_a_pipeline_flag() {
        // These are the flags every signing or network stage owns. A generated
        // field with the same spelling would make clap ambiguous at best.
        const RESERVED: &[&str] = &["url", "network", "seed-file", "seed", "sign-with", "quiet"];

        for transaction in transactions() {
            for field in &transaction.fields {
                assert!(
                    !RESERVED.contains(&field.flag),
                    "{} generates --{}, which a pipeline stage owns",
                    transaction.name,
                    field.flag
                );
            }
        }
    }
}
