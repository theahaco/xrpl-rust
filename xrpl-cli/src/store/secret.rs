//! Encrypting key material at rest.
//!
//! # Encrypt first, then choose a location
//!
//! The seed always lives inside a passphrase-encrypted blob. *Where* that blob
//! is kept — a file, an OS credential store — is a separate, swappable concern,
//! and nothing this CLI writes, in either place, ever contains key material in
//! the clear.
//!
//! That ordering is what makes the credential store worth having. A plaintext
//! secret in a keychain defends against a stolen disk and a committed dotfile,
//! and against nothing else: on Linux any process on the session bus reads it
//! with no prompt, and on macOS the ACL is pinned to a binary's code-signing
//! identity, so a `cargo build` binary is never on it and users are pushed
//! toward "Always Allow", which widens the item to any binary at that path.
//! Encrypting first means all of those leak ciphertext.
//!
//! # What this claims, exactly
//!
//! **Encrypted at rest, plaintext in process memory at signing time.** Nothing
//! stronger. Apple says it outright of its own keychain: to use a key you must
//! briefly copy a plaintext version into memory. `zeroize`'s own documentation
//! disclaims moves, realloc copies, stack spills, registers and swap.
//!
//! So this defends against a stolen laptop, a record committed to a repository,
//! a dotfiles sync, and a backup. It does not defend against malware running as
//! you while you are signing.

use std::io::{Read, Write};

use secrecy::SecretString;
use zeroize::Zeroize;

use crate::error::Error;

/// Encrypt a seed under a passphrase.
///
/// ASCII-armored, so the result is text: it survives a credential store that
/// wants a string, a `cat`, and a copy-paste, and it stays under the 2,560-byte
/// blob ceiling Windows imposes.
pub fn encrypt(seed: &str, passphrase: &str) -> Result<String, Error> {
    let recipient = age::scrypt::Recipient::new(SecretString::from(passphrase.to_owned()));

    let mut armored = Vec::new();
    let armor =
        age::armor::ArmoredWriter::wrap_output(&mut armored, age::armor::Format::AsciiArmor)
            .map_err(|error| Error::other(format!("cannot encrypt: {error}")))?;

    let encryptor = age::Encryptor::with_recipients([&recipient as _].into_iter())
        .map_err(|error| Error::other(format!("cannot encrypt: {error}")))?;

    let mut writer = encryptor
        .wrap_output(armor)
        .map_err(|error| Error::other(format!("cannot encrypt: {error}")))?;
    writer
        .write_all(seed.as_bytes())
        .map_err(|error| Error::other(format!("cannot encrypt: {error}")))?;
    writer
        .finish()
        .and_then(|armor| armor.finish())
        .map_err(|error| Error::other(format!("cannot encrypt: {error}")))?;

    String::from_utf8(armored).map_err(|error| Error::other(format!("cannot encrypt: {error}")))
}

/// Decrypt a seed with a passphrase.
///
/// The caller owns the returned `String` and should zeroize it as soon as the
/// key is derived. This is the moment the honest claim above is about.
pub fn decrypt(blob: &str, passphrase: &str) -> Result<String, Error> {
    let identity = age::scrypt::Identity::new(SecretString::from(passphrase.to_owned()));

    let armor = age::armor::ArmoredReader::new(blob.as_bytes());
    let decryptor = age::Decryptor::new(armor)
        .map_err(|_| Error::other("this does not look like an encrypted key"))?;

    let mut reader = decryptor
        .decrypt([&identity as _].into_iter())
        // Deliberately not "wrong passphrase": the same message for a wrong
        // passphrase and a corrupt blob says nothing about which it was.
        .map_err(|_| Error::other("cannot decrypt: wrong passphrase, or the file is damaged"))?;

    let mut plaintext = Vec::new();
    reader
        .read_to_end(&mut plaintext)
        .map_err(|error| Error::other(format!("cannot decrypt: {error}")))?;

    let seed = String::from_utf8(plaintext.clone())
        .map_err(|_| Error::other("the decrypted value is not a seed"))?;
    plaintext.zeroize();

    Ok(seed.trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    const SEED: &str = "snoPBrXtMeMyMHUVTgbuqAfg1SUTb";

    #[test]
    fn test_a_seed_round_trips() {
        let blob = encrypt(SEED, "correct horse").expect("encrypts");
        assert_eq!(decrypt(&blob, "correct horse").expect("decrypts"), SEED);
    }

    #[test]
    fn test_the_blob_does_not_contain_the_seed() {
        let blob = encrypt(SEED, "passphrase").expect("encrypts");

        assert!(!blob.contains(SEED), "{blob}");
        assert!(!blob.contains("snoPBrXt"), "{blob}");
    }

    #[test]
    fn test_the_blob_is_text() {
        // It has to survive a credential store that wants a string, and stay
        // under the 2,560-byte ceiling Windows puts on one.
        let blob = encrypt(SEED, "passphrase").expect("encrypts");

        assert!(blob.is_ascii(), "{blob}");
        assert!(
            blob.starts_with("-----BEGIN AGE ENCRYPTED FILE-----"),
            "{blob}"
        );
        assert!(blob.len() < 1000, "{} bytes", blob.len());
    }

    #[test]
    fn test_a_wrong_passphrase_fails_without_saying_which() {
        let blob = encrypt(SEED, "right").expect("encrypts");
        let message = decrypt(&blob, "wrong").unwrap_err().to_string();

        // One message for a wrong passphrase and for a damaged file: telling
        // them apart tells an attacker which half they have.
        assert!(
            message.contains("wrong passphrase, or the file is damaged"),
            "{message}"
        );
    }

    #[test]
    fn test_a_damaged_blob_fails_the_same_way() {
        let blob = encrypt(SEED, "passphrase").expect("encrypts");
        let damaged = blob.replace('A', "B");

        assert!(decrypt(&damaged, "passphrase").is_err());
    }

    #[test]
    fn test_something_that_is_not_a_blob_says_so() {
        let message = decrypt("just some text", "passphrase")
            .unwrap_err()
            .to_string();

        assert!(
            message.contains("does not look like an encrypted key"),
            "{message}"
        );
    }

    #[test]
    fn test_two_encryptions_of_one_seed_differ() {
        // A fresh salt each time, so identical seeds do not produce identical
        // files and a store cannot be scanned for reused keys.
        let first = encrypt(SEED, "passphrase").expect("encrypts");
        let second = encrypt(SEED, "passphrase").expect("encrypts");

        assert_ne!(first, second);
    }
}
