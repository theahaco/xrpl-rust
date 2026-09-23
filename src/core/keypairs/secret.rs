//! Secret-carrying types.
//!
//! Key material used to live in plain `String`s and `[u8; 16]`s that were never
//! wiped: `derive_keypair` returned an owned `(String, String)`, `_format_key`
//! allocated through `format!`, `_private_key_to_str` allocated through
//! `hex::encode_upper`, and `decode_seed` returned a bare array. A single
//! signature therefore left several copies of the private key on the heap, in
//! uppercase hex, for the lifetime of the process — trivially recoverable from a
//! core dump, a swap page or a same-UID heap snapshot with a `[0-9A-F]{64}`
//! search.
//!
//! The types here wipe on drop and refuse to print. They do not make a secret
//! safe from a process that is already reading this one's memory: the honest
//! claim is that the exposure window shrinks from the process lifetime to the
//! span of one call. `zeroize`'s own documentation disclaims moves, realloc
//! copies, stack spills, registers and swap.

use alloc::string::String;
use alloc::vec::Vec;
use core::fmt::{Debug, Formatter, Result as FmtResult};
use zeroize::{Zeroize, ZeroizeOnDrop};

use crate::constants::CryptoAlgorithm;
use crate::core::keypairs::utils::ED25519_PREFIX;

/// A private key in uppercase hex.
///
/// Wipes on drop, prints as `<redacted>`, and has no `Display` and no
/// `Serialize`: reaching the key material takes a deliberate call to
/// [`PrivateKey::as_str`].
#[derive(Clone, PartialEq, Eq, Zeroize, ZeroizeOnDrop)]
pub struct PrivateKey(String);

impl PrivateKey {
    /// Wrap an already-hex-encoded private key.
    pub fn new(hex: String) -> Self {
        Self(hex)
    }

    /// The key material, as uppercase hex.
    ///
    /// Every use of this is a place where the secret exists unprotected. Keep
    /// the borrow as short as possible and never copy it into an owned `String`.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The algorithm this key belongs to, read from its hex prefix.
    ///
    /// XRPL distinguishes the two curves by an `ED` prefix on the key, so this
    /// is the one place that inspects it. A signer that never exposes its key
    /// has no such channel and must carry its algorithm explicitly.
    pub fn algorithm(&self) -> CryptoAlgorithm {
        match self.0.get(..2) {
            Some(ED25519_PREFIX) => CryptoAlgorithm::ED25519,
            _ => CryptoAlgorithm::SECP256K1,
        }
    }

    /// Whether the key is empty. Present so callers need not reach for
    /// [`PrivateKey::as_str`] to ask.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl From<String> for PrivateKey {
    fn from(hex: String) -> Self {
        Self(hex)
    }
}

impl Debug for PrivateKey {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        f.write_str("PrivateKey(<redacted>)")
    }
}

/// A base58 family seed (`s…`), the value an account is recoverable from.
///
/// Wipes on drop and prints as `<redacted>`.
#[derive(Clone, PartialEq, Eq, Zeroize, ZeroizeOnDrop)]
pub struct Seed(String);

impl Seed {
    /// Wrap a base58 family seed.
    pub fn new(seed: String) -> Self {
        Self(seed)
    }

    /// The seed, as the base58 string the ledger and every other client use.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Whether the seed is empty.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl From<String> for Seed {
    fn from(seed: String) -> Self {
        Self(seed)
    }
}

impl Debug for Seed {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        f.write_str("Seed(<redacted>)")
    }
}

/// The decoded bytes of a family seed, before key derivation.
///
/// These are the bytes both curves derive from, so they are as sensitive as the
/// private key itself.
#[derive(Clone, PartialEq, Eq, Zeroize, ZeroizeOnDrop)]
pub struct SeedBytes(Vec<u8>);

impl SeedBytes {
    /// Wrap decoded seed bytes.
    pub fn new(bytes: Vec<u8>) -> Self {
        Self(bytes)
    }

    /// The decoded bytes.
    pub fn as_slice(&self) -> &[u8] {
        &self.0
    }

    /// How many bytes the seed decoded to.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Whether the seed decoded to nothing.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl AsRef<[u8]> for SeedBytes {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl Debug for SeedBytes {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        f.write_str("SeedBytes(<redacted>)")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::format;
    use alloc::string::ToString;
    use alloc::vec;

    const ED25519_KEY: &str = "EDB4C4E046826BD26190D09715FC31F4E6A728204EADD112905B08B14B7F15C4F3";
    const SECP256K1_KEY: &str =
        "00D78B9735C3F26501C7337B8A5727FD53A6EFDBC6AA55984F098488561F985E23";

    #[test]
    fn test_private_key_debug_is_redacted() {
        let key = PrivateKey::new(ED25519_KEY.to_string());

        let rendered = format!("{key:?}");
        assert!(!rendered.contains(ED25519_KEY), "{rendered}");
        assert!(rendered.contains("redacted"), "{rendered}");
    }

    #[test]
    fn test_seed_debug_is_redacted() {
        let seed = Seed::new("sEdSKaCy2JT7JaM7v95H9SxkhP9wS2r".to_string());

        let rendered = format!("{seed:?}");
        assert!(!rendered.contains("sEdSKaCy"), "{rendered}");
        assert!(rendered.contains("redacted"), "{rendered}");
    }

    #[test]
    fn test_seed_bytes_debug_is_redacted() {
        let bytes = SeedBytes::new(vec![0xAB; 16]);

        let rendered = format!("{bytes:?}");
        assert!(!rendered.contains("171"), "{rendered}");
        assert!(rendered.contains("redacted"), "{rendered}");
    }

    #[test]
    fn test_algorithm_is_read_from_the_hex_prefix() {
        assert_eq!(
            PrivateKey::new(ED25519_KEY.to_string()).algorithm(),
            CryptoAlgorithm::ED25519
        );
        assert_eq!(
            PrivateKey::new(SECP256K1_KEY.to_string()).algorithm(),
            CryptoAlgorithm::SECP256K1
        );
    }

    #[test]
    fn test_algorithm_does_not_panic_on_a_short_key() {
        // `&key[..2]` panicked here before; a key shorter than its prefix is
        // malformed input, not a reason to abort the process.
        assert_eq!(
            PrivateKey::new("E".to_string()).algorithm(),
            CryptoAlgorithm::SECP256K1
        );
        assert_eq!(
            PrivateKey::new(String::new()).algorithm(),
            CryptoAlgorithm::SECP256K1
        );
    }
}
