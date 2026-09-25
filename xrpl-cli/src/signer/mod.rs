//! Signing backends for the CLI.
//!
//! The library's `RawSigner` trait says what it means to be able to sign; this
//! is where the CLI's implementations of it live:
//!
//! - [`ephemeral`] — a seed supplied for a single invocation and then
//!   forgotten. Enrols nothing, journals nothing, touches no store.
//! - [`stored`] — a key record, whose secret is a passphrase-encrypted blob in
//!   a file or in the OS credential store.
//!
//! They are siblings rather than special cases in any pipeline stage, which is
//! the point of putting the seam in the library: a new backend is a new
//! implementation here and a new `source` on a key record, and no stage of the
//! `tx` pipeline changes. [`registry`] is the one list of them, because the
//! answer is not the same for every build.

mod account;
pub mod ephemeral;
pub mod registry;
pub mod stored;

pub use ephemeral::SigningArgs;
pub use registry::Backend;
pub use stored::StoredSigner;
