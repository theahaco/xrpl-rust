//! What an account record and a key record contain.
//!
//! # Three nouns, two files
//!
//! An **account** is an address and the things that are true of it offline. A
//! **key** is a public key and a pointer to where its secret lives. A **signer**
//! is the runtime thing that can produce a signature for a key, and is never
//! persisted — it is resolved from the key's `source` plus whichever backends
//! this binary was built with.
//!
//! Keeping keys out of the account record is not tidiness. On XRPL an address
//! derives from its *original* master public key and never changes, while
//! `SetRegularKey` and `SignerListSet` decide which keys may currently authorize
//! it. One account legitimately has several keys — a master, a regular key, and
//! up to 32 signer-list members — and which of them works is ledger state that
//! changes over time, not local configuration.
//!
//! # Invariants
//!
//! 1. **A record never contains key material.** Not a seed, not a private key,
//!    not a mnemonic. A public key and a pointer, and nothing else.
//! 2. **`address`, `public_key` and `algorithm` are answerable with zero I/O.**
//!    No credential store, no network, no prompt. This is what stops `account ls`
//!    from prompting once per row.
//! 3. **No sequence number, anywhere.** It is ledger state that changes under
//!    you; `tx autofill` reads it from a node at build time and stores nothing.
//! 4. **A record whose secret is unreachable here is a normal state**, not a
//!    corrupt one — records sync between machines and secrets do not.

use serde::{Deserialize, Serialize};

use crate::error::{Error, SignerError};

/// The record format version.
///
/// An explicit `version` and an explicit `kind`, from the first release. The
/// alternative — `#[serde(untagged)]`, deciding what a file is by which fields
/// happen to be present — is what forced stellar-cli to add a required
/// `hardware = "ledger"` field whose own comment apologises for existing, and
/// every backend added after that is a fresh compatibility hazard. A tag costs
/// nothing now and is a breaking file-format change later.
pub const VERSION: u32 = 1;

/// Where a key's secret lives.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "source", rename_all = "kebab-case")]
pub enum KeySource {
    /// A passphrase-encrypted blob on disk.
    EncryptedFile { path: String },
    /// A passphrase-encrypted blob in the OS credential store.
    SecureStore { entry: String },
    /// No secret at all: this key can be recognized, never used.
    ///
    /// A first-class state rather than a degenerate one. A watch-only account
    /// is what lets a transaction be prepared on one machine and signed on
    /// another, which is the air-gapped workflow the pipeline exists for.
    WatchOnly,
}

impl KeySource {
    /// Human-readable name, for `account doctor` and error messages.
    pub fn label(&self) -> &'static str {
        match self {
            KeySource::EncryptedFile { .. } => "encrypted-file",
            KeySource::SecureStore { .. } => "secure-store",
            KeySource::WatchOnly => "watch-only",
        }
    }
}

/// A public key and a pointer to its secret.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeyRecord {
    pub version: u32,
    /// Always `"key"`. Present so a file says what it is.
    pub kind: String,
    /// Uppercase hex, as the ledger spells it.
    pub public_key: String,
    /// `secp256k1` or `ed25519`, recorded rather than inferred.
    ///
    /// XRPL distinguishes the curves by an `ED` prefix on the key, so a backend
    /// that never exposes a private key has no channel to say which it is. A
    /// signer paired with an account on the other curve otherwise fails only
    /// after the transaction is built.
    pub algorithm: String,
    #[serde(flatten)]
    pub source: KeySource,
    /// What the classic address of this key is, cached so nothing has to derive
    /// it to list records.
    pub classic_address: String,
}

impl KeyRecord {
    pub fn new(
        public_key: String,
        algorithm: String,
        classic_address: String,
        source: KeySource,
    ) -> Self {
        Self {
            version: VERSION,
            kind: "key".into(),
            public_key,
            algorithm,
            source,
            classic_address,
        }
    }

    /// Check the file is the shape this build understands.
    pub fn validate(&self, id: &str) -> Result<(), Error> {
        if self.kind != "key" {
            return Err(Error::other(format!(
                "{id}: expected a key record, found kind {:?}",
                self.kind
            )));
        }
        if self.version > VERSION {
            return Err(Error::other(format!(
                "{id}: record version {} is newer than this build understands ({VERSION})",
                self.version
            )));
        }

        Ok(())
    }
}

/// An address and what is true of it offline.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountRecord {
    pub version: u32,
    /// Always `"account"`.
    pub kind: String,
    /// The classic r-address.
    pub address: String,
    /// Which network this account is on.
    ///
    /// Replay protection rather than convenience: `NetworkID` must be omitted
    /// for a network id at or below 1024 and is mandatory above it, and a
    /// transaction built for one chain reaching a node on another is the bug
    /// that rule exists to prevent.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub network_id: Option<u32>,
    /// An optional destination tag carried with the address.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tag: Option<u32>,
    /// Key records that can sign for this account, by id.
    ///
    /// Empty means watch-only, which is a normal thing to be.
    #[serde(default)]
    pub keys: Vec<String>,
    /// Which key to use when none is named.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_signer: Option<String>,
}

impl AccountRecord {
    pub fn new(address: String) -> Self {
        Self {
            version: VERSION,
            kind: "account".into(),
            address,
            network_id: None,
            tag: None,
            keys: Vec::new(),
            default_signer: None,
        }
    }

    /// Whether this account can sign anything at all.
    pub fn is_watch_only(&self) -> bool {
        self.keys.is_empty()
    }

    /// Whether this account is on a network where an implicit default is unsafe.
    ///
    /// Decided from the record, offline — not from `--url`. A caller who points
    /// a testnet alias at a mainnet node has made a mistake the record can
    /// still catch.
    pub fn is_mainnet(&self) -> bool {
        // Mainnet is network 0, and an account with no recorded network is
        // treated as mainnet because that is the assumption that fails safely.
        self.network_id.unwrap_or(0) == 0
    }

    pub fn validate(&self, alias: &str) -> Result<(), Error> {
        if self.kind != "account" {
            return Err(Error::other(format!(
                "{alias}: expected an account record, found kind {:?}",
                self.kind
            )));
        }
        if self.version > VERSION {
            return Err(Error::other(format!(
                "{alias}: record version {} is newer than this build understands ({VERSION})",
                self.version
            )));
        }
        if !xrpl::core::addresscodec::is_valid_classic_address(&self.address) {
            return Err(Error::other(format!(
                "{alias}: {:?} is not a valid classic address",
                self.address
            )));
        }

        Ok(())
    }

    /// The key to sign with, given an optional explicit choice.
    pub fn signer_key<'a>(&'a self, requested: Option<&'a str>) -> Result<&'a str, Error> {
        if let Some(id) = requested {
            if !self.keys.iter().any(|key| key == id) {
                return Err(SignerError::NotFound {
                    kind: "key on this account",
                    name: id.to_string(),
                }
                .into());
            }
            return Ok(id);
        }

        if let Some(default) = &self.default_signer {
            return Ok(default);
        }

        match self.keys.as_slice() {
            [only] => Ok(only),
            [] => Err(SignerError::Unavailable(format!(
                "{} is watch-only: it has no keys",
                self.address
            ))
            .into()),
            _ => Err(Error::other(
                "this account has several keys and no default; name one with --sign-with",
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ADDRESS: &str = "rHb9CJAWyB4rj91VRWn96DkukG4bwdtyTh";

    #[test]
    fn test_an_account_record_round_trips_through_toml() {
        let mut record = AccountRecord::new(ADDRESS.into());
        record.network_id = Some(1);
        record.keys = vec!["alice-master".into()];
        record.default_signer = Some("alice-master".into());

        let text = toml::to_string_pretty(&record).expect("serializes");
        let parsed: AccountRecord = toml::from_str(&text).expect("parses");

        assert_eq!(parsed, record);
    }

    #[test]
    fn test_a_record_says_what_it_is() {
        // An explicit `kind` rather than guessing from which fields are present.
        let text = toml::to_string_pretty(&AccountRecord::new(ADDRESS.into())).expect("serializes");

        assert!(text.contains("kind = \"account\""), "{text}");
        assert!(text.contains("version = 1"), "{text}");
    }

    #[test]
    fn test_a_key_source_is_tagged_not_guessed() {
        let record = KeyRecord::new(
            "ED00".into(),
            "ed25519".into(),
            ADDRESS.into(),
            KeySource::WatchOnly,
        );

        let text = toml::to_string_pretty(&record).expect("serializes");
        assert!(text.contains("source = \"watch-only\""), "{text}");

        let parsed: KeyRecord = toml::from_str(&text).expect("parses");
        assert_eq!(parsed.source, KeySource::WatchOnly);
    }

    #[test]
    fn test_an_encrypted_file_source_carries_its_path() {
        let record = KeyRecord::new(
            "ED00".into(),
            "ed25519".into(),
            ADDRESS.into(),
            KeySource::EncryptedFile {
                path: "keys/alice.age".into(),
            },
        );

        let parsed: KeyRecord =
            toml::from_str(&toml::to_string_pretty(&record).expect("serializes")).expect("parses");
        assert_eq!(parsed.source, record.source);
    }

    #[test]
    fn test_a_record_never_carries_key_material() {
        // The invariant, asserted rather than assumed: there is no field a
        // secret could be written into.
        let record = KeyRecord::new(
            "ED00".into(),
            "ed25519".into(),
            ADDRESS.into(),
            KeySource::EncryptedFile {
                path: "keys/alice.age".into(),
            },
        );

        let text = toml::to_string_pretty(&record).expect("serializes");
        for forbidden in ["seed", "private", "secret", "mnemonic"] {
            assert!(
                !text.to_lowercase().contains(forbidden),
                "a key record must not have a {forbidden} field:\n{text}"
            );
        }
    }

    #[test]
    fn test_an_account_with_no_keys_is_watch_only() {
        let record = AccountRecord::new(ADDRESS.into());

        assert!(record.is_watch_only());
        // And that is a state with a clear error, not a crash.
        assert!(record.signer_key(None).is_err());
    }

    #[test]
    fn test_one_key_needs_no_default() {
        let mut record = AccountRecord::new(ADDRESS.into());
        record.keys = vec!["alice-master".into()];

        assert_eq!(record.signer_key(None).expect("resolves"), "alice-master");
    }

    #[test]
    fn test_several_keys_and_no_default_asks_rather_than_guessing() {
        let mut record = AccountRecord::new(ADDRESS.into());
        record.keys = vec!["master".into(), "regular".into()];

        let message = record.signer_key(None).unwrap_err().to_string();
        assert!(message.contains("--sign-with"), "{message}");
    }

    #[test]
    fn test_naming_a_key_the_account_does_not_have_is_an_error() {
        let mut record = AccountRecord::new(ADDRESS.into());
        record.keys = vec!["master".into()];

        assert!(record.signer_key(Some("someone-elses-key")).is_err());
    }

    #[test]
    fn test_a_malformed_address_is_rejected_on_read() {
        let mut record = AccountRecord::new("not-an-address".into());
        record.kind = "account".into();

        assert!(record.validate("alice").is_err());
    }

    #[test]
    fn test_a_newer_record_version_is_refused_rather_than_misread() {
        let mut record = AccountRecord::new(ADDRESS.into());
        record.version = VERSION + 1;

        let message = record.validate("alice").unwrap_err().to_string();
        assert!(message.contains("newer than this build"), "{message}");
    }

    #[test]
    fn test_an_account_with_no_network_is_treated_as_mainnet() {
        // The assumption that fails safely: an implicit default is refused on
        // mainnet, so an unknown network must not quietly opt out of that.
        assert!(AccountRecord::new(ADDRESS.into()).is_mainnet());

        let mut testnet = AccountRecord::new(ADDRESS.into());
        testnet.network_id = Some(1);
        assert!(!testnet.is_mainnet());
    }
}
