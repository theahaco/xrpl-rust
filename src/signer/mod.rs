//! What it means to be able to sign for an XRPL account.
//!
//! This module is deliberately small and deliberately low. It is gated on the
//! `core` feature alone — not the `core + models + wallet` triple that
//! [`crate::signing`] carries — because its whole surface is `&[u8]` in and hex
//! out, which is `no_std`-safe. Backends that need `std`, a filesystem, a
//! credential store, a device or a human live above it, in the CLI.
//!
//! # Framing, and why the signer does it
//!
//! XRPL has no envelope: the signing domain is encoded *inside* the bytes being
//! signed, as a four-byte prefix (`STX\0` single, `SMT\0` multisign with the
//! signer's own AccountID appended, `CLM\0` payment-channel claim, `BCH\0`
//! batch inner).
//!
//! A trait shaped `fn sign(&self, bytes: &[u8])` would therefore make every
//! signer a universal signing oracle for its account. The moment anything can
//! choose the bytes — a message-signing verb, a caller passing through
//! user-supplied input — it can hand over a valid `CLM\0` pre-image labelled as
//! something harmless and walk away with a payment-channel claim. A claim is 44
//! bytes, carries no sequence number, and authorizes a drain.
//!
//! So [`RawSigner::sign`] takes a [`SigningDomain`] and an **unframed** payload,
//! and the implementation frames it with [`frame`]. A signer never accepts bytes
//! a caller has already framed. [`SigningDomain::Message`] has a reserved prefix
//! that is not a valid XRPL hash prefix, so a signed message can never be
//! replayed as a transaction.
//!
//! # Synchronous, on purpose
//!
//! Every backend in scope — an in-memory [`crate::wallet::Wallet`], a seed read
//! from a file, an age-encrypted blob, an OS credential store — is blocking. A
//! synchronous trait stays object-safe and `no_std`-safe, which the `core`-only
//! gate requires. A backend that genuinely needs the network owns its runtime
//! internally, and callers resolve and unlock a signer *before* entering one.

pub mod exceptions;

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::fmt::{Display, Formatter, Result as FmtResult};

use crate::constants::CryptoAlgorithm;
use crate::core::addresscodec::decode_classic_address;
use crate::core::binarycodec::{
    BATCH_PREFIX, PAYMENT_CHANNEL_CLAIM_PREFIX, TRANSACTION_MULTISIG_PREFIX,
    TRANSACTION_SIGNATURE_PREFIX,
};
use crate::core::keypairs::derive_classic_address;

pub use exceptions::XRPLSignerException;

/// Result of a signing operation.
pub type XRPLSignerResult<T = ()> = Result<T, XRPLSignerException>;

/// The prefix for a signed message (`MSG\0`).
///
/// Deliberately not one of XRPL's four transaction hash prefixes, so bytes
/// signed as a message can never be replayed as a transaction, a payment-channel
/// claim or a batch inner.
pub const MESSAGE_PREFIX: [u8; 4] = (0x4D534700u32).to_be_bytes();

/// Which signing domain a payload belongs to.
///
/// The domain decides the framing, and the framing is what keeps a signature
/// obtained for one purpose from being valid for another.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum SigningDomain<'a> {
    /// A single-signature transaction (`STX\0`).
    Single,
    /// One signer's contribution to a multisigned transaction (`SMT\0`).
    ///
    /// Carries the signer's own classic address, which is appended to the
    /// payload as a 20-byte AccountID. In a 2-of-3 ceremony this is neither the
    /// transaction's `Account` nor the account being signed for — it is the
    /// signing key's own address, and only that belongs here.
    MultiAs(&'a str),
    /// A payment-channel claim (`CLM\0`).
    PaymentChannelClaim,
    /// An inner transaction of an XLS-56 batch (`BCH\0`).
    BatchInner,
    /// An arbitrary message, under a prefix that is not a transaction prefix.
    Message,
    /// XLS-68 sponsorship. Reserved, not implemented: `SponsorSignature` is a
    /// non-signing field, so nothing can fill it until this has a pre-image.
    Sponsor,
}

impl Display for SigningDomain<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            SigningDomain::Single => f.write_str("Single"),
            SigningDomain::MultiAs(account) => write!(f, "MultiAs({account})"),
            SigningDomain::PaymentChannelClaim => f.write_str("PaymentChannelClaim"),
            SigningDomain::BatchInner => f.write_str("BatchInner"),
            SigningDomain::Message => f.write_str("Message"),
            SigningDomain::Sponsor => f.write_str("Sponsor"),
        }
    }
}

/// Why a signature is only part of what the transaction needs.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum PartialReason {
    /// The collected signatures do not yet meet the account's `SignerQuorum`.
    BelowQuorum,
    /// The backend deliberately did not submit, and handed the blob back for
    /// the next signer.
    HandOver,
}

/// What a backend did with a transaction it broadcast itself.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct SubmitReport {
    /// The transaction hash, if the backend knows it.
    pub hash: Option<String>,
    /// The backend's own description of the outcome.
    pub outcome: String,
}

/// What came back from a signing backend.
///
/// Not a bare signature, even though every backend in scope today returns one.
/// A backend that broadcasts on its own behalf and a caller that then submits
/// again is a double-submission bug, and a below-quorum multisignature is a
/// normal result rather than an error — both are outcomes a `Result<Signature>`
/// cannot express, and adding them later would be a breaking change to every
/// implementation.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum SignOutcome {
    /// A signature, as uppercase hex.
    Signed(String),
    /// A signature that does not by itself authorize the transaction.
    Partial {
        signature: String,
        reason: PartialReason,
    },
    /// The backend signed and broadcast the transaction itself. The caller must
    /// not submit it again.
    SignedAndSubmitted(SubmitReport),
}

impl SignOutcome {
    /// The signature, when the backend produced one and did not submit it.
    ///
    /// Callers that can only handle a plain signature — the library's own
    /// [`crate::signing::sign`] among them — use this and surface the other two
    /// outcomes as errors rather than silently treating them as success.
    pub fn signature(&self) -> Option<&str> {
        match self {
            SignOutcome::Signed(signature) => Some(signature),
            SignOutcome::Partial { signature, .. } => Some(signature),
            SignOutcome::SignedAndSubmitted(_) => None,
        }
    }
}

/// Frame an unframed payload for its signing domain.
///
/// This is the only place prefixes and suffixes are applied. Every
/// [`RawSigner`] implementation must route through it, so that no
/// implementation can be talked into signing bytes it did not frame.
pub fn frame(domain: &SigningDomain<'_>, payload: &[u8]) -> XRPLSignerResult<Vec<u8>> {
    let mut framed = Vec::with_capacity(payload.len() + 24);

    match domain {
        SigningDomain::Single => framed.extend_from_slice(&TRANSACTION_SIGNATURE_PREFIX),
        SigningDomain::PaymentChannelClaim => {
            framed.extend_from_slice(&PAYMENT_CHANNEL_CLAIM_PREFIX)
        }
        SigningDomain::BatchInner => framed.extend_from_slice(&BATCH_PREFIX),
        SigningDomain::Message => framed.extend_from_slice(&MESSAGE_PREFIX),
        SigningDomain::MultiAs(account) => {
            let account_id =
                decode_classic_address(account).map_err(|error| XRPLSignerException::Framing {
                    domain: domain.to_string(),
                    reason: error.to_string(),
                })?;
            framed.extend_from_slice(&TRANSACTION_MULTISIG_PREFIX);
            framed.extend_from_slice(payload);
            framed.extend_from_slice(&account_id);
            return Ok(framed);
        }
        SigningDomain::Sponsor => {
            return Err(XRPLSignerException::UnsupportedDomain(domain.to_string()))
        }
    }

    framed.extend_from_slice(payload);

    Ok(framed)
}

/// Something that can produce a signature for one XRPL key.
///
/// A signer holds *how to sign*, not *which account it signs for*. On XRPL those
/// really are separate: an account's address derives from its original master
/// public key and never changes, while `SetRegularKey` and `SignerListSet` decide
/// which keys may currently authorize it — that binding is ledger state, not
/// local configuration. Matching a signer to an account is done by comparing
/// public keys, and reconciling either against the ledger is a separate concern.
pub trait RawSigner {
    /// This signer's public key, as uppercase hex.
    ///
    /// Infallible and free of I/O by contract: listing accounts must never
    /// prompt, unlock a credential store or reach a device.
    fn public_key(&self) -> &str;

    /// Which curve this signer's key is on.
    ///
    /// Explicit rather than inferred. XRPL distinguishes the curves by an `ED`
    /// prefix on the key itself, so a backend that never exposes its private key
    /// has no channel to say which it is — and a signer paired with an account
    /// on the other curve fails only after the transaction has been built.
    fn algorithm(&self) -> CryptoAlgorithm;

    /// The classic address of this signer's own key.
    ///
    /// Derived from the public key, so it needs no I/O. Backends that already
    /// hold the address override this to avoid re-deriving it.
    fn classic_address(&self) -> XRPLSignerResult<String> {
        Ok(derive_classic_address(self.public_key())?)
    }

    /// Sign an **unframed** payload for `domain`.
    ///
    /// Implementations must frame the payload with [`frame`] and must not accept
    /// pre-framed bytes: see this module's documentation for why.
    fn sign(&self, domain: SigningDomain<'_>, payload: &[u8]) -> XRPLSignerResult<SignOutcome>;
}

#[cfg(test)]
mod tests {
    use super::*;

    const ADDRESS: &str = "rHb9CJAWyB4rj91VRWn96DkukG4bwdtyTh";

    #[test]
    fn test_each_domain_gets_its_own_prefix() {
        let payload = [0xAAu8; 8];

        for (domain, prefix) in [
            (SigningDomain::Single, TRANSACTION_SIGNATURE_PREFIX),
            (
                SigningDomain::PaymentChannelClaim,
                PAYMENT_CHANNEL_CLAIM_PREFIX,
            ),
            (SigningDomain::BatchInner, BATCH_PREFIX),
            (SigningDomain::Message, MESSAGE_PREFIX),
        ] {
            let framed = frame(&domain, &payload).expect("frames");
            assert_eq!(&framed[..4], &prefix, "{domain}");
            assert_eq!(&framed[4..], &payload, "{domain}");
        }
    }

    #[test]
    fn test_the_message_prefix_is_not_a_transaction_prefix() {
        // This is the whole point of having a separate domain: a signature over
        // a "message" must never be replayable as a transaction or a claim.
        for prefix in [
            TRANSACTION_SIGNATURE_PREFIX,
            TRANSACTION_MULTISIG_PREFIX,
            PAYMENT_CHANNEL_CLAIM_PREFIX,
            BATCH_PREFIX,
        ] {
            assert_ne!(MESSAGE_PREFIX, prefix);
        }
    }

    #[test]
    fn test_multisign_framing_appends_the_signer_account_id() {
        let payload = [0xBBu8; 8];
        let framed = frame(&SigningDomain::MultiAs(ADDRESS), &payload).expect("frames");

        let account_id = decode_classic_address(ADDRESS).expect("decodes");
        assert_eq!(&framed[..4], &TRANSACTION_MULTISIG_PREFIX);
        assert_eq!(&framed[4..4 + payload.len()], &payload);
        assert_eq!(&framed[4 + payload.len()..], &account_id[..]);
        assert_eq!(account_id.len(), 20);
    }

    #[test]
    fn test_multisign_framing_rejects_a_malformed_account() {
        let error = frame(&SigningDomain::MultiAs("not-an-address"), &[0u8; 4]).unwrap_err();
        assert!(matches!(error, XRPLSignerException::Framing { .. }));
    }

    #[test]
    fn test_sponsor_is_reserved_and_refuses() {
        let error = frame(&SigningDomain::Sponsor, &[0u8; 4]).unwrap_err();
        assert!(matches!(error, XRPLSignerException::UnsupportedDomain(_)));
    }

    #[test]
    fn test_two_domains_never_produce_the_same_bytes() {
        let payload = [0xCCu8; 16];
        let single = frame(&SigningDomain::Single, &payload).expect("frames");
        let claim = frame(&SigningDomain::PaymentChannelClaim, &payload).expect("frames");
        let batch = frame(&SigningDomain::BatchInner, &payload).expect("frames");
        let message = frame(&SigningDomain::Message, &payload).expect("frames");

        assert_ne!(single, claim);
        assert_ne!(single, batch);
        assert_ne!(single, message);
        assert_ne!(claim, batch);
        assert_ne!(claim, message);
        assert_ne!(batch, message);
    }

    #[test]
    fn test_signature_accessor_refuses_an_already_submitted_outcome() {
        let submitted = SignOutcome::SignedAndSubmitted(SubmitReport {
            hash: None,
            outcome: "validated".into(),
        });
        // A caller that treated this as a signature would submit a second time.
        assert_eq!(submitted.signature(), None);

        assert_eq!(SignOutcome::Signed("AB".into()).signature(), Some("AB"));
        assert_eq!(
            SignOutcome::Partial {
                signature: "CD".into(),
                reason: PartialReason::BelowQuorum,
            }
            .signature(),
            Some("CD")
        );
    }
}
