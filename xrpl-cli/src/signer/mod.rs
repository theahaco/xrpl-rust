//! Signing backends for the CLI.
//!
//! The library's `RawSigner` trait says what it means to be able to sign; this
//! is where the CLI's implementations of it live. Today there is one:
//! [`ephemeral`], a seed supplied for a single invocation and then forgotten.
//!
//! The account and key records, an encrypted keystore and an OS credential
//! store are the backends that come next, and they arrive as siblings here
//! rather than as changes to any pipeline stage — which is the point of putting
//! the seam in the library.

pub mod ephemeral;
pub mod stored;

pub use ephemeral::SigningArgs;
pub use stored::StoredSigner;
