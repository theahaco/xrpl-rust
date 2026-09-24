//! Turning a seed into a key record plus an encrypted blob.
//!
//! Shared by `key add` and `key generate`, so the two cannot disagree about
//! what enrolment means.

use xrpl::signer::RawSigner;
use xrpl::wallet::Wallet;
use zeroize::Zeroize;

use crate::error::Error;
use crate::signer::stored::PASSPHRASE_ENV;
use crate::store::{secret, KeyRecord, KeySource, Store};
use crate::tty;

/// Encrypt a seed and write both halves of the record.
pub fn enrol(store: &Store, id: &str, seed: &str) -> Result<KeyRecord, Error> {
    let wallet = Wallet::new(seed, 0)?;
    let passphrase = passphrase_for_new_key(id)?;

    let blob = secret::encrypt(seed, &passphrase)?;
    let path = store.write_secret(id, &blob)?;

    let record = KeyRecord::new(
        wallet.public_key.clone(),
        format!("{:?}", wallet.algorithm()).to_lowercase(),
        wallet.classic_address.clone(),
        KeySource::EncryptedFile { path },
    );

    store.write_key(id, &record)?;

    Ok(record)
}

/// Ask for a passphrase twice, or take one from the environment.
///
/// Twice, because a mistyped passphrase on a key that exists nowhere else is
/// unrecoverable and there is nothing to compare against later.
fn passphrase_for_new_key(id: &str) -> Result<String, Error> {
    if let Ok(passphrase) = std::env::var(PASSPHRASE_ENV) {
        if !passphrase.is_empty() {
            return Ok(passphrase);
        }
    }

    let first = tty::prompt_secret(&format!("new passphrase for {id}"))?;
    if first.is_empty() {
        return Err(Error::other(
            "an empty passphrase encrypts nothing; choose one",
        ));
    }

    let mut second = tty::prompt_secret("confirm passphrase")?;
    let matched = first == second;
    second.zeroize();

    if !matched {
        return Err(Error::other("the passphrases did not match"));
    }

    Ok(first)
}
