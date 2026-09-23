//! Methods for working with XRPL wallets.

pub mod exceptions;
#[cfg(feature = "helpers")]
pub mod faucet_generation;

use crate::constants::CryptoAlgorithm;
use crate::core::addresscodec::classic_address_to_xaddress;
use crate::core::keypairs::derive_classic_address;
use crate::core::keypairs::derive_keypair;
use crate::core::keypairs::generate_seed;
use crate::core::keypairs::{PrivateKey, Seed};
use alloc::string::String;
use core::fmt::{Debug, Display};
use exceptions::XRPLWalletResult;

/// The cryptographic keys needed to control an
/// XRP Ledger account.
///
/// See Cryptographic Keys:
/// `<https://xrpl.org/cryptographic-keys.html>`
pub struct Wallet {
    /// The seed from which the public and private keys
    /// are derived.
    pub seed: Seed,
    /// The public key that is used to identify this wallet's
    /// signatures, as a hexadecimal string.
    pub public_key: String,
    /// The private key that is used to create signatures.
    ///
    /// A [`PrivateKey`] rather than a `String`: it wipes on drop, prints as
    /// `<redacted>`, and has no `Display`, so reaching the key material takes a
    /// deliberate `as_str()`.
    pub private_key: PrivateKey,
    /// The address that publicly identifies this wallet, as
    /// a base58 string.
    pub classic_address: String,
    /// The next available sequence number to use for
    /// transactions from this wallet. Must be updated by the
    /// user. Increments on the ledger with every successful
    /// transaction submission, and stays the same with every
    /// failed transaction submission.
    ///
    /// **Note:** This field duplicates the `Sequence` field of the account's
    /// `AccountRoot` ledger object and must be kept in sync manually. It is
    /// retained for backwards compatibility and is slated for removal in a
    /// future major version. Prefer querying the ledger directly for the
    /// authoritative sequence number.
    pub sequence: u64,
}

// There is no hand-written `Drop` here any more. `seed` and `private_key` are
// `Seed` and `PrivateKey`, which wipe themselves; `public_key`,
// `classic_address` and `sequence` are not secret and were only being wiped
// because the whole struct was. Dropping the impl also makes `Wallet`
// destructurable: a type with a manual `Drop` cannot have its fields moved out,
// which forced every conversion to clone the secret one more time.

impl Debug for Wallet {
    /// Custom Debug implementation that hides sensitive key material.
    ///
    /// `seed` and `private_key` redact themselves, but they are spelled out
    /// here too so a future `#[derive(Debug)]` cannot quietly widen this.
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Wallet")
            .field("seed", &"***REDACTED***")
            .field("public_key", &self.public_key)
            .field("private_key", &"***REDACTED***")
            .field("classic_address", &self.classic_address)
            .field("sequence", &self.sequence)
            .finish()
    }
}

impl Wallet {
    /// Generate a new Wallet.
    pub fn new(seed: &str, sequence: u64) -> XRPLWalletResult<Self> {
        let (public_key, private_key) = derive_keypair(seed, false)?;
        let classic_address = derive_classic_address(&public_key)?;

        Ok(Wallet {
            seed: Seed::new(seed.into()),
            public_key,
            private_key,
            classic_address,
            sequence,
        })
    }

    /// Generates a new seed and Wallet.
    pub fn create(crypto_algorithm: Option<CryptoAlgorithm>) -> XRPLWalletResult<Self> {
        Self::new(&generate_seed(None, crypto_algorithm)?, 0)
    }

    /// Returns the X-Address of the Wallet's account.
    pub fn get_xaddress(
        &self,
        tag: Option<u64>,
        is_test_network: bool,
    ) -> XRPLWalletResult<String> {
        Ok(classic_address_to_xaddress(
            &self.classic_address,
            tag,
            is_test_network,
        )?)
    }
}

#[cfg(all(test, feature = "wallet"))]
mod tests {
    use super::*;

    /// Seed for the well-known genesis account used throughout XRPL tests.
    const GENESIS_SEED: &str = "snoPBrXtMeMyMHUVTgbuqAfg1SUTb";

    #[test]
    fn test_debug_redacts_secrets_and_shows_public_key() {
        let wallet = Wallet::new(GENESIS_SEED, 0).expect("genesis wallet");
        let debug_output = alloc::format!("{:?}", wallet);

        // public_key must be visible in Debug (not secret)
        assert!(
            debug_output.contains(&wallet.public_key),
            "public_key should appear in Debug output"
        );
        // classic_address must be visible
        assert!(
            debug_output.contains(&wallet.classic_address),
            "classic_address should appear in Debug output"
        );
        // seed must NOT be visible
        assert!(
            !debug_output.contains(wallet.seed.as_str()),
            "seed must not appear in Debug output"
        );
        // private_key must NOT be visible
        assert!(
            !debug_output.contains(wallet.private_key.as_str()),
            "private_key must not appear in Debug output"
        );
    }
}

impl Display for Wallet {
    /// Returns a string representation of a Wallet.
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        write!(
            f,
            "Wallet {{ public_key: {}, private_key: -HIDDEN-, classic_address: {} }}",
            self.public_key, self.classic_address
        )
    }
}
