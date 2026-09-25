//! Turning `--account`, and every other address a command is given, into an
//! address.
//!
//! One flag with one meaning across the whole binary: it names an **account**.
//! An alias resolves to that record's address; a literal r-address passes
//! through verbatim; anything that parses as a family seed is refused.
//!
//! # `--account` never selects a key
//!
//! Nor does any other address. `--destination`, `--issuer`, `--holder`, the
//! address half of `--signer-entry` and every other AccountID field of `tx new`
//! also accept an alias, through [`alias_address`], and it hands back the
//! record's address and nothing else: no key is read, nothing is unlocked, and
//! the network is never asked. What stays confined to `--account` is the
//! ladder: no other field inherits a value from the environment or the config.
//! The alternative to that split is the union type stellar-cli ended up with,
//! where one argument could be an alias *or* a secret key, and a literal
//! address silently could not sign.
//!
//! # The ladder
//!
//! `--account` → `XRPL_ACCOUNT` → the default in `config.toml`. No project-local
//! tier, ever.
//!
//! Two guards travel with it. Any resolution that did not come from an explicit
//! `--account` says so on stderr, so an inherited value is never invisible. And
//! an implicit default is **refused on mainnet** — decided offline from the
//! record's own `network_id`, not from `--url`, because the point is to catch
//! the case where those two disagree.

use crate::error::{Error, SignerError};
use crate::output;
use crate::store::{AccountRecord, Store};

/// Where a resolved account came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    /// Named explicitly on the command line.
    Explicit,
    /// From `XRPL_ACCOUNT`.
    Environment,
    /// From the default in `config.toml`.
    ConfigDefault,
}

impl Source {
    fn describe(self) -> &'static str {
        match self {
            Source::Explicit => "--account",
            Source::Environment => "XRPL_ACCOUNT",
            Source::ConfigDefault => "the config default",
        }
    }
}

/// An account, and how it was chosen.
#[derive(Debug, Clone)]
pub struct Resolved {
    pub address: String,
    /// The record, when the value named one. A literal address has none.
    pub record: Option<AccountRecord>,
    /// The alias, when one was used.
    pub alias: Option<String>,
    pub source: Source,
}

/// Resolve `--account`, applying the ladder and both guards.
///
/// `explicit` is whatever `--account` carried, if anything.
pub fn account(store: &Store, explicit: Option<&str>) -> Result<Resolved, Error> {
    let (value, source) = match explicit {
        Some(value) => (value.to_string(), Source::Explicit),
        None => match std::env::var("XRPL_ACCOUNT").ok().filter(|v| !v.is_empty()) {
            Some(value) => (value, Source::Environment),
            None => match store.default_account() {
                Some(alias) => (alias, Source::ConfigDefault),
                None => {
                    return Err(SignerError::NotFound {
                        kind: "account",
                        name: "none given".into(),
                    }
                    .into())
                }
            },
        },
    };

    let resolved = resolve_value(store, &value, source)?;

    if source != Source::Explicit {
        // An inherited account is never invisible. Two resolution rules in one
        // binary would be worse than one rule with a visible trace.
        output::note(format!(
            "account {} resolved from {}",
            resolved.alias.as_deref().unwrap_or(&resolved.address),
            source.describe()
        ));

        if resolved
            .record
            .as_ref()
            .is_none_or(AccountRecord::is_mainnet)
        {
            return Err(Error::other(format!(
                "refusing to use {} on mainnet without an explicit --account. \
                 An inherited account decides who pays.",
                resolved.alias.as_deref().unwrap_or(&resolved.address)
            )));
        }
    }

    Ok(resolved)
}

fn resolve_value(store: &Store, value: &str, source: Source) -> Result<Resolved, Error> {
    reject_secret("account", value)?;

    if xrpl::core::addresscodec::is_valid_classic_address(value) {
        return Ok(Resolved {
            address: value.to_string(),
            record: None,
            alias: None,
            source,
        });
    }

    let record = store.account(value)?;

    Ok(Resolved {
        address: record.address.clone(),
        record: Some(record),
        alias: Some(value.to_string()),
        source,
    })
}

/// Look an alias up for an address field other than `--account`.
///
/// `field` is the flag the value came from, for messages. The caller has
/// already let a literal address through; this answers `Ok(None)` for a name
/// with no record, so the caller can refuse it naming its own field rather
/// than sending the raw string.
pub fn alias_address(store: &Store, field: &str, alias: &str) -> Result<Option<String>, Error> {
    reject_secret(field, alias)?;

    // A value that could never be a record's name is not looked up: it is a
    // mistyped address, and `../x` must not become a path.
    if super::check_alias(alias).is_err() {
        return Ok(None);
    }

    match store.account(alias) {
        Ok(record) => Ok(Some(record.address)),
        Err(Error::Signer(SignerError::NotFound { .. })) => Ok(None),
        Err(error) => Err(error),
    }
}

/// Refuse anything that looks like key material.
///
/// A seed reaching a flag that names an account is the conflation this design
/// exists to prevent: at best it is "no such account", at worst it matches a
/// stored alias and signs with the wrong key.
fn reject_secret(field: &str, value: &str) -> Result<(), Error> {
    // A family seed is base58 starting with `s`, and short. Checking the shape
    // rather than decoding it means nothing has to touch the value.
    let looks_like_seed = value.starts_with('s')
        && (25..=31).contains(&value.len())
        && value.chars().all(|c| c.is_ascii_alphanumeric());

    if looks_like_seed {
        return Err(Error::other(format!(
            "--{field} was given something that looks like a seed. It names an \
             account, never a key: use --seed-file or XRPL_SEED to supply key material."
        )));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const ADDRESS: &str = "rHb9CJAWyB4rj91VRWn96DkukG4bwdtyTh";
    const SEED: &str = "snoPBrXtMeMyMHUVTgbuqAfg1SUTb";

    fn store_with(alias: &str, network_id: Option<u32>) -> (tempfile::TempDir, Store) {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = Store::at(dir.path());

        let mut record = AccountRecord::new(ADDRESS.into());
        record.network_id = network_id;
        store.write_account(alias, &record).expect("writes");

        (dir, store)
    }

    #[test]
    fn test_a_literal_address_passes_through() {
        let (_dir, store) = store_with("alice", None);
        let resolved = account(&store, Some(ADDRESS)).expect("resolves");

        assert_eq!(resolved.address, ADDRESS);
        // A literal address has no record, so it can never select a key.
        assert!(resolved.record.is_none());
        assert!(resolved.alias.is_none());
    }

    #[test]
    fn test_an_alias_resolves_to_its_address() {
        let (_dir, store) = store_with("alice", None);
        let resolved = account(&store, Some("alice")).expect("resolves");

        assert_eq!(resolved.address, ADDRESS);
        assert_eq!(resolved.alias.as_deref(), Some("alice"));
    }

    #[test]
    fn test_a_seed_is_refused_rather_than_looked_up() {
        let (_dir, store) = store_with("alice", None);

        // The conflation this exists to prevent: at worst a seed matches a
        // stored alias and signs with the wrong key.
        let message = account(&store, Some(SEED)).unwrap_err().to_string();
        assert!(message.contains("looks like a seed"), "{message}");
    }

    #[test]
    fn test_an_unknown_alias_is_not_found() {
        let (_dir, store) = store_with("alice", None);

        assert!(matches!(
            account(&store, Some("nobody")).unwrap_err(),
            Error::Signer(SignerError::NotFound { .. })
        ));
    }

    #[test]
    fn test_an_alias_in_another_field_is_its_address() {
        let (_dir, store) = store_with("alice", None);

        assert_eq!(
            alias_address(&store, "destination", "alice").expect("reads"),
            Some(ADDRESS.to_string())
        );
    }

    #[test]
    fn test_an_unknown_alias_in_another_field_is_none() {
        let (_dir, store) = store_with("alice", None);

        assert_eq!(
            alias_address(&store, "destination", "nobody").expect("reads"),
            None
        );
        // Never turned into a path outside the store.
        assert_eq!(
            alias_address(&store, "destination", "../alice").expect("reads"),
            None
        );
    }

    #[test]
    fn test_a_seed_in_another_field_is_refused_naming_it() {
        let (_dir, store) = store_with("alice", None);

        let message = alias_address(&store, "destination", SEED)
            .unwrap_err()
            .to_string();
        assert!(message.contains("--destination"), "{message}");
        assert!(message.contains("looks like a seed"), "{message}");
    }

    #[test]
    fn test_nothing_at_all_is_not_found() {
        let (_dir, store) = store_with("alice", None);

        assert!(matches!(
            account(&store, None).unwrap_err(),
            Error::Signer(SignerError::NotFound { .. })
        ));
    }

    #[test]
    fn test_an_implicit_mainnet_account_is_refused() {
        // Network 0, and an account with no recorded network, are both mainnet.
        let (_dir, store) = store_with("alice", Some(0));
        store.set_default_account(Some("alice")).expect("sets");

        let message = account(&store, None).unwrap_err().to_string();
        assert!(message.contains("mainnet"), "{message}");
        assert!(message.contains("--account"), "{message}");
    }

    #[test]
    fn test_an_implicit_testnet_account_is_allowed() {
        let (_dir, store) = store_with("alice", Some(1));
        store.set_default_account(Some("alice")).expect("sets");

        let resolved = account(&store, None).expect("resolves");
        assert_eq!(resolved.source, Source::ConfigDefault);
    }

    #[test]
    fn test_an_explicit_mainnet_account_is_fine() {
        // The refusal is about *inheriting* an account, not about mainnet.
        let (_dir, store) = store_with("alice", Some(0));

        assert!(account(&store, Some("alice")).is_ok());
    }
}
