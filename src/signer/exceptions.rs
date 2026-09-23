use alloc::string::String;
use thiserror_no_std::Error;

use crate::core::exceptions::XRPLCoreException;

/// What a signing backend can fail with.
///
/// `#[non_exhaustive]` from the first variant: backends will be added — an
/// encrypted keystore, an OS credential store, a device, a subprocess — and each
/// brings failure modes a caller must be able to tell apart. Callers that match
/// on this decide retry behaviour and process exit codes, so collapsing them
/// into one opaque string makes every retry wrapper wrong: a signature the user
/// deliberately cancelled must not look like a transport blip.
#[derive(Debug, PartialEq, Error)]
#[non_exhaustive]
pub enum XRPLSignerException {
    /// The payload could not be framed for the requested signing domain.
    #[error("Cannot frame a payload for {domain}: {reason}")]
    Framing { domain: String, reason: String },

    /// The signing domain is reserved but not implemented.
    #[error("Signing domain {0} is reserved and not implemented")]
    UnsupportedDomain(String),

    /// The backend cannot sign for the account it was asked about.
    #[error("Signer holds no key for {0}")]
    NoKeyFor(String),

    /// The backend holds a pointer to a secret it cannot reach from here — a
    /// credential-store entry on another machine, a device that is not plugged
    /// in. Distinct from "no such key": the record exists and the secret does
    /// not, which is a recoverable state with a different remedy.
    #[error("Signer is unavailable: {0}")]
    Unavailable(String),

    /// A human declined the signature.
    #[error("Signature declined")]
    Declined,

    /// The underlying cryptography or codec failed.
    #[error("Core error: {0}")]
    Core(#[from] XRPLCoreException),
}
