//! Signing with a key the store holds.
//!
//! A key record points at an encrypted blob. Unlocking one decrypts the seed,
//! derives the key, and holds it for as long as the command runs — which is the
//! window the honest claim is about: encrypted at rest, plaintext in process
//! memory at signing time.
//!
//! # One unlock per invocation
//!
//! The signer is resolved and unlocked **once**, after reading the transaction
//! accounts, and the same instance signs every line of a stream. A passphrase prompt
//! per transaction would make a stream unusable, and there is no session agent
//! here to paper over it.
//!
//! It is also unlocked **before** any Tokio runtime is entered. A blocking
//! prompt inside a `block_on` panics with "Cannot start a runtime from within a
//! runtime", and only on the interactive path — so CI stays green and it
//! detonates the first time a person is asked for a passphrase.

use xrpl::constants::CryptoAlgorithm;
use xrpl::signer::{RawSigner, SignOutcome, SigningDomain, XRPLSignerResult};
use xrpl::wallet::Wallet;
use zeroize::Zeroize;

use crate::error::Error;
use crate::store::Store;
use crate::tty;

/// The environment variable a script supplies a passphrase through.
///
/// stdin carries the transaction, so a passphrase cannot travel that way. This
/// is the non-interactive path, and it is named in the error a command gives
/// when there is no terminal to prompt on.
pub const PASSPHRASE_ENV: &str = "XRPL_PASSPHRASE";

/// A signer backed by a key record.
///
/// No `Debug`: it holds a derived key, and a struct that can be printed is one
/// that eventually is.
pub struct StoredSigner {
    id: String,
    wallet: Wallet,
    journal: Option<crate::store::journal::Journal>,
}

impl StoredSigner {
    /// Decrypt a key record's secret and hold it for this command.
    pub fn unlock(store: &Store, id: &str) -> Result<Self, Error> {
        let record = store.key(id)?;

        let blob = store.read_key_secret(id, &record)?;

        let passphrase = passphrase_for(id)?;
        let mut seed = crate::store::secret::decrypt(&blob, &passphrase)?;
        let wallet = Wallet::new(&seed, 0);
        seed.zeroize();
        let wallet = wallet?;

        if wallet.public_key != record.public_key {
            // The record and the blob disagree. Signing anyway would produce a
            // valid signature for an account nobody expected.
            return Err(Error::other(format!(
                "{id}: the decrypted key is {} but the record says {}. \
                 The record and the secret have come apart.",
                wallet.public_key, record.public_key
            )));
        }

        Ok(Self {
            id: id.to_string(),
            wallet,
            journal: crate::store::journal::Journal::open(store),
        })
    }

    /// Note a signature in the local journal.
    ///
    /// Best-effort by design: an unwritable journal is a warning, never a
    /// failed signature. A backend that refuses to sign because it could not
    /// write a log is worse than one that tells you the log is missing.
    fn record_signature(&self, domain: &SigningDomain<'_>) {
        if let Some(journal) = &self.journal {
            journal.note(&self.id, &self.wallet.classic_address, domain);
        }
    }
}

impl RawSigner for StoredSigner {
    fn public_key(&self) -> &str {
        &self.wallet.public_key
    }

    fn algorithm(&self) -> CryptoAlgorithm {
        self.wallet.algorithm()
    }

    fn classic_address(&self) -> XRPLSignerResult<String> {
        Ok(self.wallet.classic_address.clone())
    }

    fn sign(&self, domain: SigningDomain<'_>, payload: &[u8]) -> XRPLSignerResult<SignOutcome> {
        let outcome = self.wallet.sign(domain.clone(), payload)?;
        self.record_signature(&domain);

        Ok(outcome)
    }
}

/// Where a passphrase comes from: the environment, or the terminal.
fn passphrase_for(id: &str) -> Result<String, Error> {
    if let Ok(passphrase) = std::env::var(PASSPHRASE_ENV) {
        if !passphrase.is_empty() {
            return Ok(passphrase);
        }
    }

    tty::prompt_secret(&format!("passphrase for {id}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::SignerError;
    use crate::store::{KeyRecord, KeySource, Store};

    const SEED: &str = "snoPBrXtMeMyMHUVTgbuqAfg1SUTb";
    const ADDRESS: &str = "rHb9CJAWyB4rj91VRWn96DkukG4bwdtyTh";

    fn store_with_key() -> (tempfile::TempDir, Store) {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = Store::at(dir.path());

        let wallet = Wallet::new(SEED, 0).expect("wallet");
        let blob = crate::store::secret::encrypt(SEED, "passphrase").expect("encrypts");
        let path = store.write_secret("alice", &blob).expect("writes blob");

        let record = KeyRecord::new(
            wallet.public_key.clone(),
            "secp256k1".into(),
            wallet.classic_address.clone(),
            KeySource::EncryptedFile { path },
        );
        store.write_key("alice", &record).expect("writes record");

        (dir, store)
    }

    /// Run with a passphrase in the environment, serialized against other tests
    /// that touch it.
    fn with_passphrase<T>(value: &str, body: impl FnOnce() -> T) -> T {
        use std::sync::{Mutex, OnceLock};
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        let _guard = LOCK
            .get_or_init(|| Mutex::new(()))
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());

        let previous = std::env::var_os(PASSPHRASE_ENV);
        // Safety: the lock serializes every test that reads this variable.
        unsafe { std::env::set_var(PASSPHRASE_ENV, value) };

        let result = body();

        match previous {
            Some(value) => unsafe { std::env::set_var(PASSPHRASE_ENV, value) },
            None => unsafe { std::env::remove_var(PASSPHRASE_ENV) },
        }

        result
    }

    #[test]
    fn test_unlocking_derives_the_recorded_key() {
        let (_dir, store) = store_with_key();

        let signer = with_passphrase("passphrase", || {
            StoredSigner::unlock(&store, "alice").expect("unlocks")
        });

        assert_eq!(signer.classic_address().expect("address"), ADDRESS);
    }

    #[test]
    fn test_a_wrong_passphrase_does_not_unlock() {
        let (_dir, store) = store_with_key();

        let error = with_passphrase("wrong", || {
            StoredSigner::unlock(&store, "alice")
                .err()
                .expect("should not unlock")
        });
        assert!(error.to_string().contains("wrong passphrase"), "{error}");
    }

    #[test]
    fn test_a_watch_only_key_is_unavailable_rather_than_missing() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = Store::at(dir.path());
        store
            .write_key(
                "watcher",
                &KeyRecord::new(
                    "ED00".into(),
                    "ed25519".into(),
                    ADDRESS.into(),
                    KeySource::WatchOnly,
                ),
            )
            .expect("writes");

        // Exit code 6, not 4: the record is here and the secret is not, which
        // is a different problem with a different remedy.
        let error = StoredSigner::unlock(&store, "watcher")
            .err()
            .expect("should not unlock");
        assert!(
            matches!(error, Error::Signer(SignerError::Unavailable(_))),
            "{error:?}"
        );
    }

    #[test]
    fn test_a_record_that_disagrees_with_its_secret_is_refused() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = Store::at(dir.path());

        let blob = crate::store::secret::encrypt(SEED, "passphrase").expect("encrypts");
        let path = store.write_secret("mismatched", &blob).expect("writes");

        // A public key that is not the one the blob holds.
        store
            .write_key(
                "mismatched",
                &KeyRecord::new(
                    "EDDEADBEEF".into(),
                    "ed25519".into(),
                    ADDRESS.into(),
                    KeySource::EncryptedFile { path },
                ),
            )
            .expect("writes");

        let error = with_passphrase("passphrase", || {
            StoredSigner::unlock(&store, "mismatched")
                .err()
                .expect("should not unlock")
        });

        // Signing anyway would produce a valid signature for an account nobody
        // expected.
        assert!(error.to_string().contains("come apart"), "{error}");
    }

    #[test]
    fn test_signing_produces_a_verifiable_signature() {
        let (_dir, store) = store_with_key();
        let signer = with_passphrase("passphrase", || {
            StoredSigner::unlock(&store, "alice").expect("unlocks")
        });

        let payload = b"a serialized transaction";
        let outcome = signer.sign(SigningDomain::Single, payload).expect("signs");

        let signature = match outcome {
            SignOutcome::Signed(signature) => signature,
            other => panic!("expected a plain signature, got {other:?}"),
        };

        let framed = xrpl::signer::frame(&SigningDomain::Single, payload).expect("frames");
        assert!(xrpl::core::keypairs::is_valid_message(
            &framed,
            &signature,
            signer.public_key()
        ));
    }
}
