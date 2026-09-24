//! Every way this binary can produce a signature.
//!
//! One list rather than three places that each know about one backend. It
//! exists because the answer is **not** the same for every build: `secure-store`
//! is behind a Cargo feature, and a `KeySource` naming it parses whether or not
//! the backend is there (#14 — features are additive, so the shape of a public
//! type must not depend on the feature set). Without somewhere to ask, "can this
//! binary use that key" is a question you answer by trying it.
//!
//! `ephemeral` is in the list beside the record-backed ones deliberately. It is
//! a backend, not a special case in the signing stage: a seed supplied for one
//! invocation, used, and forgotten. What separates it from the others is that
//! it enrols nothing and journals nothing — which is what keeps `tx sign
//! --seed-file` a pure crypto stage with no filesystem dependency, so it works
//! on a read-only container, in a sandbox, and in CI.

use crate::store::keychain;

/// One way of producing a signature.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Backend {
    /// The name that appears in a key record's `source`, and in errors.
    pub name: &'static str,
    /// Whether this build can actually use it.
    pub available: bool,
    /// Whether a signature through it is recorded in the signing journal.
    pub journals: bool,
    /// Whether it involves a stored record at all.
    pub enrolled: bool,
    /// One line, for `--help` and for the listing.
    pub summary: &'static str,
}

/// Every backend this CLI knows the name of, available here or not.
///
/// Listing an unavailable backend is the point: a record written elsewhere
/// names it, and "unknown backend" and "backend not compiled in" are different
/// problems with different remedies.
pub fn backends() -> [Backend; 4] {
    [
        Backend {
            name: "ephemeral",
            available: true,
            journals: false,
            enrolled: false,
            summary: "a seed supplied for one invocation, then forgotten \
                      (--seed-file, XRPL_SEED, or a prompt)",
        },
        Backend {
            name: "encrypted-file",
            available: true,
            journals: true,
            enrolled: true,
            summary: "a passphrase-encrypted blob in the data directory",
        },
        Backend {
            name: "secure-store",
            available: keychain::is_compiled_in(),
            journals: true,
            enrolled: true,
            summary: "the same encrypted blob, kept in the OS credential store \
                      instead of on the filesystem",
        },
        Backend {
            name: "watch-only",
            available: true,
            journals: false,
            enrolled: true,
            // Not a degenerate backend: it is what lets a transaction be
            // prepared on one machine and signed on another.
            summary: "a public key and no secret: it can be recognized, never used",
        },
    ]
}

/// Look one up by the name a key record uses.
pub fn find(name: &str) -> Option<Backend> {
    backends().into_iter().find(|backend| backend.name == name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_every_key_source_label_is_in_the_registry() {
        // The registry is only useful if it is exhaustive. A new `KeySource`
        // whose label is missing here would list as an unknown backend.
        use crate::store::KeySource;

        for source in [
            KeySource::EncryptedFile {
                path: String::new(),
            },
            KeySource::SecureStore {
                entry: String::new(),
            },
            KeySource::WatchOnly,
        ] {
            assert!(find(source.label()).is_some(), "{}", source.label());
        }
    }

    #[test]
    fn test_the_two_backends_that_never_journal_are_the_two_without_a_secret_at_rest() {
        for backend in backends() {
            assert_eq!(
                backend.journals,
                backend.enrolled && backend.name != "watch-only",
                "{}",
                backend.name
            );
        }
    }

    #[test]
    fn test_ephemeral_is_always_available_and_never_enrolled() {
        let ephemeral = find("ephemeral").expect("in the registry");

        // It has no dependency to be missing: a seed and a curve.
        assert!(ephemeral.available);
        // And nothing it touches outlives the process.
        assert!(!ephemeral.enrolled);
        assert!(!ephemeral.journals);
    }

    #[test]
    fn test_an_unknown_name_is_not_a_backend() {
        assert!(find("ghostsig").is_none());
    }

    #[cfg(not(feature = "secure-store"))]
    #[test]
    fn test_secure_store_is_listed_even_when_it_is_not_available() {
        let secure = find("secure-store").expect("listed regardless");

        // "Unknown backend" and "not compiled in" are different problems.
        assert!(!secure.available);
    }
}
