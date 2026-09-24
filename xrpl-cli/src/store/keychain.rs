//! Keeping the encrypted blob in the OS credential store.
//!
//! # What this is and is not
//!
//! It is a **location**, not a protection. What goes in is the same age blob
//! `store::secret` writes to a file — see that module for why the ordering is
//! encrypt-then-place and not the other way round. A plaintext secret in a
//! keychain defends against a stolen disk and a committed dotfile, and against
//! nothing else.
//!
//! So this buys one thing over the file backend: the blob is not on the
//! filesystem, so it is not in a dotfiles sync, a backup, or a `tar` of the home
//! directory. The passphrase is still required to use it.
//!
//! # Why it is optional
//!
//! The dependency pulls a platform backend in, and on Linux a D-Bus client. The
//! encrypted-file backend works everywhere and is the default, so a build that
//! does not want a Secret Service client should not have to carry one.
//!
//! The *variant* is unconditional even so ([`crate::store::KeySource`] always
//! carries `SecureStore`), because Cargo features are additive and the shape of
//! a public type must not depend on the feature set (#14). A binary built
//! without the feature reads such a record, says what it is, and refuses at the
//! point of use rather than failing to parse the file.
//!
//! # Timeouts
//!
//! Every call runs on a worker thread with a deadline. On Linux the Secret
//! Service is a D-Bus service that may not be running, may be waiting on a
//! desktop prompt nobody can see, or may be an `ssh` session with no session
//! bus at all — and the failure mode there is a hang, which is the worst answer
//! a CLI can give. The thread is abandoned on timeout: a blocked D-Bus call
//! cannot be safely cancelled, and the alternative is blocking on it anyway.

use std::time::Duration;

use crate::error::{Error, SignerError};

/// The service name every entry is filed under.
///
/// One constant, because it is the key to finding these again by hand — in
/// Keychain Access, in `secret-tool`, in Credential Manager — and a per-store
/// or per-version name would make that folklore.
pub const SERVICE: &str = "xrpl-cli";

/// How long to wait for the credential store before giving up.
///
/// Long enough for a keychain prompt a person has to answer, short enough that
/// a script against a dead Secret Service fails rather than appearing to hang.
pub const TIMEOUT: Duration = Duration::from_secs(30);

/// Whether this binary was built with the backend compiled in.
pub const fn is_compiled_in() -> bool {
    cfg!(feature = "secure-store")
}

/// The error a binary without the backend gives, everywhere it could be needed.
#[cfg(not(feature = "secure-store"))]
fn not_compiled_in<T>() -> Result<T, Error> {
    Err(SignerError::BackendNotEnabled("secure-store").into())
}

#[cfg(not(feature = "secure-store"))]
mod backend {
    use super::*;

    pub fn probe() -> Result<(), Error> {
        not_compiled_in()
    }

    pub fn store(_entry: &str, _blob: &str) -> Result<(), Error> {
        not_compiled_in()
    }

    pub fn load(_entry: &str) -> Result<String, Error> {
        not_compiled_in()
    }

    pub fn remove(_entry: &str) -> Result<(), Error> {
        not_compiled_in()
    }
}

#[cfg(feature = "secure-store")]
mod backend {
    use super::*;

    /// Run a credential-store call with a deadline.
    ///
    /// The `Entry` is constructed *inside* the worker, so nothing that the
    /// platform backend owns has to cross a thread boundary.
    fn with_deadline<T: Send + 'static>(
        what: &str,
        body: impl FnOnce() -> Result<T, Error> + Send + 'static,
    ) -> Result<T, Error> {
        let (sender, receiver) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            // A closed channel means the deadline passed and nobody is
            // listening; the value is dropped, which is the right outcome.
            let _ = sender.send(body());
        });

        receiver.recv_timeout(TIMEOUT).unwrap_or_else(|_| {
            Err(SignerError::Unavailable(format!(
                "the OS credential store did not answer within {}s ({what}). \
                 On Linux this usually means no Secret Service is running — \
                 check `systemctl --user status gnome-keyring-daemon`, or use \
                 an encrypted-file key instead.",
                TIMEOUT.as_secs()
            ))
            .into())
        })
    }

    /// Translate a keyring error into this CLI's vocabulary.
    ///
    /// `NoEntry` is [`SignerError::Unavailable`] rather than `NotFound`: the
    /// key *record* exists — that is how we knew to look — and only its secret
    /// is missing, which is exit 6 and a different remedy from exit 4.
    fn translate(entry: &str, error: keyring::Error) -> Error {
        match error {
            keyring::Error::NoEntry => SignerError::Unavailable(format!(
                "{SERVICE}/{entry} is not in this machine's credential store"
            ))
            .into(),
            keyring::Error::NoDefaultStore => SignerError::Unavailable(format!(
                "this machine has no usable credential store: {}",
                store_status_message()
            ))
            .into(),
            other => SignerError::Backend(format!("credential store: {other}")).into(),
        }
    }

    fn store_status_message() -> String {
        match keyring::Entry::store_status() {
            Ok(()) => "the store reports itself available".to_string(),
            Err(error) => error.to_string(),
        }
    }

    /// Whether the credential store can be reached at all.
    ///
    /// Separate from a lookup so `account doctor` can say "no credential store
    /// here" without naming, or needing, a particular key.
    pub fn probe() -> Result<(), Error> {
        with_deadline("probing the store", || {
            match keyring::Entry::store_status() {
                Ok(()) => Ok(()),
                Err(error) => Err(SignerError::Unavailable(format!(
                    "this machine has no usable credential store: {error}"
                ))
                .into()),
            }
        })
    }

    pub fn store(entry: &str, blob: &str) -> Result<(), Error> {
        let (name, blob) = (entry.to_string(), blob.to_string());

        with_deadline("writing a credential", move || {
            let handle =
                keyring::Entry::new(SERVICE, &name).map_err(|error| translate(&name, error))?;

            // The byte API, not `set_password`. The blob is ASCII armor, so a
            // string would survive — but Windows stores a password as UTF-16
            // and the round trip is the platform's business, not ours.
            handle
                .set_secret(blob.as_bytes())
                .map_err(|error| translate(&name, error))
        })
    }

    pub fn load(entry: &str) -> Result<String, Error> {
        let name = entry.to_string();

        with_deadline("reading a credential", move || {
            let handle =
                keyring::Entry::new(SERVICE, &name).map_err(|error| translate(&name, error))?;
            let bytes = handle
                .get_secret()
                .map_err(|error| translate(&name, error))?;

            String::from_utf8(bytes).map_err(|_| {
                Error::other(format!(
                    "{SERVICE}/{name} in the credential store is not text; \
                     it was not written by this CLI"
                ))
            })
        })
    }

    pub fn remove(entry: &str) -> Result<(), Error> {
        let name = entry.to_string();

        with_deadline("deleting a credential", move || {
            let handle =
                keyring::Entry::new(SERVICE, &name).map_err(|error| translate(&name, error))?;

            match handle.delete_credential() {
                // Already gone is the state we wanted.
                Err(keyring::Error::NoEntry) => Ok(()),
                other => other.map_err(|error| translate(&name, error)),
            }
        })
    }
}

pub use backend::{load, probe, remove, store};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_the_service_name_is_stable() {
        // It is the key to finding these by hand in Keychain Access or
        // `secret-tool`; changing it strands every existing entry.
        assert_eq!(SERVICE, "xrpl-cli");
    }

    #[cfg(not(feature = "secure-store"))]
    #[test]
    fn test_without_the_feature_every_call_says_which_backend_is_missing() {
        // Not a parse failure and not a panic: a record written on a machine
        // with the feature is readable here, and only using it is refused.
        for error in [
            probe().unwrap_err(),
            store("alice", "blob").unwrap_err(),
            load("alice").unwrap_err(),
            remove("alice").unwrap_err(),
        ] {
            assert!(
                matches!(
                    error,
                    Error::Signer(SignerError::BackendNotEnabled("secure-store"))
                ),
                "{error:?}"
            );
        }
    }

    #[cfg(not(feature = "secure-store"))]
    #[test]
    fn test_the_feature_flag_is_reported_honestly() {
        assert!(!is_compiled_in());
    }

    #[cfg(feature = "secure-store")]
    #[test]
    fn test_the_feature_flag_is_reported_honestly() {
        assert!(is_compiled_in());
    }
}
