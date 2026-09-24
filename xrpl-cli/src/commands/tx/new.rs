//! `xrpl tx new <Type>` — build an unsigned transaction, offline.
//!
//! The subcommands are generated from the ledger's own definitions rather than
//! written out: one per transaction type, with one flag per field that type
//! accepts. All 82 work, including the sixteen that have no Rust model, and a
//! definitions refresh changes the surface without anyone editing a match arm.
//!
//! # Why the field names are checked strictly
//!
//! `encode(json!({…, "NotAField": "x"}))` returns **byte-identical** hex to the
//! same transaction without that key: the binary codec skips anything it does
//! not recognize, silently, for parity with other clients. So `--desination`
//! would otherwise produce a perfectly valid, perfectly signed transaction that
//! pays nobody. Every field name is checked against the type's own format entry,
//! with a did-you-mean, and `--allow-unknown-fields` is the deliberate escape.

use clap::{Arg, ArgAction, ArgMatches, Command};
use serde_json::{Map, Value};

use crate::commands::tx::{io, txdef, value, EMPTY_SIGNING_PUB_KEY};
use crate::error::Error;

/// Flags this command adds to every generated subcommand.
const FLAGS_ARG: &str = "flags";
const FLAG_ARG: &str = "flag";
const SET_FLAG_ARG: &str = "set-flag";
const CLEAR_FLAG_ARG: &str = "clear-flag";
const FIELD_ARG: &str = "field";
const SIGNER_ENTRY_ARG: &str = "signer-entry";
const MEMO_ARG: &str = "memo";
const ALLOW_UNKNOWN_ARG: &str = "allow-unknown-fields";

/// A built transaction, before anything has signed it.
#[derive(Debug, Clone)]
pub struct Cmd {
    transaction: Value,
}

impl Cmd {
    pub fn run(&self) -> Result<(), Error> {
        io::write_txs(std::slice::from_ref(&self.transaction))
    }

    /// The transaction this command built. Exposed for tests.
    pub fn transaction(&self) -> &Value {
        &self.transaction
    }
}

impl clap::Subcommand for Cmd {
    fn augment_subcommands(command: Command) -> Command {
        let mut command = command;

        for definition in txdef::transactions() {
            let mut subcommand = Command::new(definition.command)
                // The exact protocol spelling works too, so anything copied
                // from xrpl.org runs as typed.
                .alias(definition.name)
                .about(format!("Build a {} transaction", definition.name))
                .arg(
                    Arg::new(FLAGS_ARG)
                        .long(FLAGS_ARG)
                        .value_name("INT")
                        .help("Set Flags to this integer directly"),
                )
                .arg(
                    Arg::new(FIELD_ARG)
                        .long(FIELD_ARG)
                        .value_name("NAME=VALUE")
                        .action(ArgAction::Append)
                        .help("Set a field by its protocol name, for anything without a flag"),
                )
                .arg(
                    Arg::new(ALLOW_UNKNOWN_ARG)
                        .long(ALLOW_UNKNOWN_ARG)
                        .action(ArgAction::SetTrue)
                        .help("Accept field names this build's definitions do not carry"),
                )
                .arg(
                    Arg::new(MEMO_ARG)
                        .long(MEMO_ARG)
                        .value_name("TEXT")
                        .action(ArgAction::Append)
                        .help("Attach a memo. Repeatable"),
                );

            if !definition.flags.is_empty() {
                let names: Vec<&'static str> = definition.flags.keys().copied().collect();
                subcommand = subcommand.arg(
                    Arg::new(FLAG_ARG)
                        .long(FLAG_ARG)
                        .value_name("NAME")
                        .action(ArgAction::Append)
                        .value_parser(names)
                        .help("Set a named flag. Repeatable"),
                );
            }

            if definition.name == "AccountSet" {
                let names: Vec<&'static str> = txdef::account_set_flags().keys().copied().collect();
                subcommand = subcommand
                    .arg(
                        Arg::new(SET_FLAG_ARG)
                            .long(SET_FLAG_ARG)
                            .value_name("NAME")
                            .value_parser(names.clone())
                            .help("Set an account flag, by name"),
                    )
                    .arg(
                        Arg::new(CLEAR_FLAG_ARG)
                            .long(CLEAR_FLAG_ARG)
                            .value_name("NAME")
                            .value_parser(names)
                            .help("Clear an account flag, by name"),
                    );
            }

            if definition.field("SignerEntries").is_some() {
                subcommand = subcommand.arg(
                    Arg::new(SIGNER_ENTRY_ARG)
                        .long(SIGNER_ENTRY_ARG)
                        .alias("signer")
                        .value_name("ADDRESS:WEIGHT")
                        .action(ArgAction::Append)
                        .help("Add a signer entry. Repeatable"),
                );
            }

            for field in definition.generated_fields() {
                subcommand = subcommand.arg(
                    Arg::new(field.flag)
                        .long(field.flag)
                        // The exact protocol spelling always works, so anything
                        // copied from xrpl.org can be pasted straight in.
                        .alias(field.name)
                        .value_name(field.serialization_type)
                        .required(field.required)
                        .help(format!("{} ({})", field.name, field.serialization_type)),
                );
            }

            command = command.subcommand(subcommand);
        }

        command
    }

    fn augment_subcommands_for_update(command: Command) -> Command {
        Self::augment_subcommands(command)
    }

    fn has_subcommand(name: &str) -> bool {
        txdef::transaction(name).is_some()
    }
}

impl clap::FromArgMatches for Cmd {
    fn from_arg_matches(matches: &ArgMatches) -> Result<Self, clap::Error> {
        let (name, sub) = matches.subcommand().ok_or_else(|| {
            clap::Error::raw(
                clap::error::ErrorKind::MissingSubcommand,
                "a transaction type is required; `xrpl tx new --help` lists them\n",
            )
        })?;

        let definition = txdef::transaction(name).ok_or_else(|| {
            clap::Error::raw(
                clap::error::ErrorKind::InvalidSubcommand,
                format!("unknown transaction type {name:?}\n"),
            )
        })?;

        build(definition, sub)
            .map(|transaction| Self { transaction })
            .map_err(|error| {
                clap::Error::raw(
                    clap::error::ErrorKind::ValueValidation,
                    format!("{error}\n"),
                )
            })
    }

    fn update_from_arg_matches(&mut self, matches: &ArgMatches) -> Result<(), clap::Error> {
        *self = Self::from_arg_matches(matches)?;
        Ok(())
    }
}

/// Assemble the transaction JSON from what was typed.
fn build(definition: &txdef::TransactionDef, matches: &ArgMatches) -> Result<Value, Error> {
    let mut object = Map::new();
    object.insert(
        "TransactionType".into(),
        Value::String(definition.name.to_string()),
    );

    for field in definition.generated_fields() {
        if let Some(raw) = matches.get_one::<String>(field.flag) {
            object.insert(
                field.name.to_string(),
                value::parse(field.flag, field.serialization_type, raw)?,
            );
        }
    }

    if let Some(entries) = matches
        .try_get_many::<String>(SIGNER_ENTRY_ARG)
        .ok()
        .flatten()
    {
        let entries: Vec<String> = entries.cloned().collect();
        object.insert(
            "SignerEntries".into(),
            value::parse_signer_entries(&entries)?,
        );
    }

    if let Some(memos) = matches.get_many::<String>(MEMO_ARG) {
        let memos: Vec<String> = memos.cloned().collect();
        object.insert("Memos".into(), value::parse_memos(&memos));
    }

    apply_flags(definition, matches, &mut object)?;
    apply_account_set_flags(matches, &mut object)?;
    apply_raw_fields(definition, matches, &mut object)?;

    // Always, and unconditionally: absent and "" are different bytes, and a
    // transaction that loses the empty key makes two signers sign two different
    // digests.
    object.insert(
        "SigningPubKey".into(),
        Value::String(EMPTY_SIGNING_PUB_KEY.into()),
    );

    Ok(Value::Object(object))
}

fn apply_flags(
    definition: &txdef::TransactionDef,
    matches: &ArgMatches,
    object: &mut Map<String, Value>,
) -> Result<(), Error> {
    let mut bits = 0u32;
    let mut any = false;

    if let Some(raw) = matches.get_one::<String>(FLAGS_ARG) {
        bits |= raw
            .replace('_', "")
            .parse::<u32>()
            .map_err(|_| Error::other(format!("--flags: {raw:?} is not an integer")))?;
        any = true;
    }

    if let Some(names) = matches.try_get_many::<String>(FLAG_ARG).ok().flatten() {
        for name in names {
            let bit = definition
                .flags
                .get(name.as_str())
                .ok_or_else(|| Error::other(format!("{} has no flag {name:?}", definition.name)))?;
            bits |= bit;
            any = true;
        }
    }

    if any {
        // `Flags` is always an integer on the wire; names exist only on the
        // command line.
        object.insert("Flags".into(), Value::from(bits));
    }

    Ok(())
}

fn apply_account_set_flags(
    matches: &ArgMatches,
    object: &mut Map<String, Value>,
) -> Result<(), Error> {
    for (arg, field) in [(SET_FLAG_ARG, "SetFlag"), (CLEAR_FLAG_ARG, "ClearFlag")] {
        if let Some(name) = matches.try_get_one::<String>(arg).ok().flatten() {
            let bit = txdef::account_set_flags()
                .get(name.as_str())
                .ok_or_else(|| Error::other(format!("no account flag {name:?}")))?;
            object.insert(field.to_string(), Value::from(*bit));
        }
    }

    Ok(())
}

fn apply_raw_fields(
    definition: &txdef::TransactionDef,
    matches: &ArgMatches,
    object: &mut Map<String, Value>,
) -> Result<(), Error> {
    let allow_unknown = matches.get_flag(ALLOW_UNKNOWN_ARG);

    let Some(entries) = matches.get_many::<String>(FIELD_ARG) else {
        return Ok(());
    };

    for entry in entries {
        let (name, raw) = entry
            .split_once('=')
            .ok_or_else(|| Error::other(format!("--field expects NAME=VALUE, got {entry:?}")))?;

        match txdef::known_field(name) {
            Some(serialization_type) => {
                object.insert(
                    name.to_string(),
                    value::parse(name, serialization_type, raw)?,
                );
            }
            None if allow_unknown => {
                // A field from an amendment newer than the vendored definitions.
                // The codec will drop it, which is why this needs saying out loud.
                crate::output::warn(format!(
                    "{name} is not in this build's definitions and will not be serialized"
                ));
                object.insert(name.to_string(), Value::String(raw.to_string()));
            }
            None => return Err(txdef::unknown_field(definition, name)),
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::{CommandFactory, Parser, Subcommand};
    use serde_json::json;

    const ACCOUNT: &str = "rHb9CJAWyB4rj91VRWn96DkukG4bwdtyTh";
    const DESTINATION: &str = "rPT1Sjq2YGrBMTttX4GZHjKu9dyfzbpAYe";

    #[derive(Parser)]
    struct Harness {
        #[command(subcommand)]
        command: Cmd,
    }

    fn build_tx(args: &[&str]) -> Result<Value, clap::Error> {
        let mut full = vec!["harness"];
        full.extend_from_slice(args);
        Harness::try_parse_from(full).map(|harness| harness.command.transaction)
    }

    #[test]
    fn test_all_eighty_two_types_build_a_command() {
        let command = Harness::command();
        let subcommands: Vec<_> = command.get_subcommands().collect();

        assert_eq!(subcommands.len(), 82);
    }

    #[test]
    fn test_every_generated_command_renders_help() {
        // A command that panics or renders nothing is a broken surface no test
        // of one transaction type would catch.
        let mut command = Harness::command();
        command.build();

        for sub in command.get_subcommands_mut() {
            let help = sub.render_help().to_string();
            assert!(!help.is_empty(), "{} rendered no help", sub.get_name());
        }
    }

    #[test]
    fn test_a_payment_builds() {
        let tx = build_tx(&[
            "payment",
            "--account",
            ACCOUNT,
            "--destination",
            DESTINATION,
            "--amount",
            "10000000",
        ])
        .expect("builds");

        assert_eq!(tx["TransactionType"], json!("Payment"));
        assert_eq!(tx["Account"], json!(ACCOUNT));
        assert_eq!(tx["Amount"], json!("10000000"));
        assert_eq!(tx["SigningPubKey"], json!(""));
    }

    #[test]
    fn test_the_protocol_spelling_is_accepted_as_an_alias() {
        // So anything copied verbatim from xrpl.org works.
        let tx = build_tx(&[
            "payment",
            "--Account",
            ACCOUNT,
            "--Destination",
            DESTINATION,
            "--Amount",
            "10000000",
        ])
        .expect("builds");

        assert_eq!(tx["Destination"], json!(DESTINATION));
    }

    #[test]
    fn test_a_required_field_is_required() {
        // Payment without a Destination must not parse.
        let error = build_tx(&["payment", "--account", ACCOUNT, "--amount", "1"]).unwrap_err();

        assert_eq!(
            error.kind(),
            clap::error::ErrorKind::MissingRequiredArgument
        );
    }

    #[test]
    fn test_fee_and_sequence_are_not_required() {
        // The definitions mark them `optionality: 0`, but autofill supplies them.
        build_tx(&[
            "payment",
            "--account",
            ACCOUNT,
            "--destination",
            DESTINATION,
            "--amount",
            "1",
        ])
        .expect("builds without --fee or --sequence");
    }

    #[test]
    fn test_named_flags_become_one_integer() {
        let tx = build_tx(&[
            "mptoken-issuance-create",
            "--account",
            ACCOUNT,
            "--flag",
            "tfMPTCanTransfer",
            "--flag",
            "tfMPTCanLock",
            "--flag",
            "tfMPTCanClawback",
        ])
        .expect("builds");

        // 2 | 32 | 64
        assert_eq!(tx["Flags"], json!(98));
    }

    #[test]
    fn test_an_unknown_flag_name_is_rejected_by_clap() {
        let error = build_tx(&[
            "payment",
            "--account",
            ACCOUNT,
            "--destination",
            DESTINATION,
            "--amount",
            "1",
            "--flag",
            "tfNotAFlag",
        ])
        .unwrap_err();

        assert_eq!(error.kind(), clap::error::ErrorKind::InvalidValue);
    }

    #[test]
    fn test_account_set_resolves_a_flag_by_name() {
        let tx = build_tx(&[
            "account-set",
            "--account",
            ACCOUNT,
            "--set-flag",
            "asfDisableMaster",
        ])
        .expect("builds");

        assert_eq!(tx["SetFlag"], json!(4));
        // This is offline JSON like any other type: no signer, no node, no
        // confirmation. The guardrail belongs at submit time.
        assert_eq!(tx["TransactionType"], json!("AccountSet"));
    }

    #[test]
    fn test_signer_entries_are_repeatable_sugar() {
        let tx = build_tx(&[
            "signer-list-set",
            "--account",
            ACCOUNT,
            "--signer-quorum",
            "2",
            "--signer-entry",
            &format!("{DESTINATION}:1"),
            "--signer-entry",
            &format!("{ACCOUNT}:1"),
        ])
        .expect("builds");

        assert_eq!(tx["SignerQuorum"], json!(2));
        assert_eq!(tx["SignerEntries"].as_array().expect("array").len(), 2);
    }

    #[test]
    fn test_an_unknown_field_name_is_refused_with_a_suggestion() {
        // The codec silently drops an unrecognized key, so this would otherwise
        // be a valid signed transaction that pays nobody.
        let error = build_tx(&[
            "payment",
            "--account",
            ACCOUNT,
            "--destination",
            DESTINATION,
            "--amount",
            "1",
            "--field",
            "Desination=rX",
        ])
        .unwrap_err();

        let rendered = error.to_string();
        assert!(rendered.contains("has no field"), "{rendered}");
        assert!(rendered.contains("destination"), "{rendered}");
    }

    #[test]
    fn test_allow_unknown_fields_is_the_deliberate_escape() {
        build_tx(&[
            "payment",
            "--account",
            ACCOUNT,
            "--destination",
            DESTINATION,
            "--amount",
            "1",
            "--field",
            "FromANewerAmendment=1",
            "--allow-unknown-fields",
        ])
        .expect("builds");
    }

    #[test]
    fn test_a_type_with_no_rust_model_still_builds() {
        // Sixteen types have no model in this crate. The wire format never goes
        // through one, so they work anyway — which is the whole bet.
        let tx = build_tx(&[
            "delegate-set",
            "--account",
            ACCOUNT,
            "--authorize",
            DESTINATION,
            "--permissions",
            "[]",
        ])
        .expect("builds");

        assert_eq!(tx["TransactionType"], json!("DelegateSet"));
    }

    #[test]
    fn test_memos_are_sugar_over_the_array() {
        let tx = build_tx(&[
            "payment",
            "--account",
            ACCOUNT,
            "--destination",
            DESTINATION,
            "--amount",
            "1",
            "--memo",
            "mint-period=2026",
        ])
        .expect("builds");

        assert!(tx["Memos"].as_array().is_some_and(|m| m.len() == 1));
    }
}
