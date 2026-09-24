//! A local record of what was signed.
//!
//! For a tool whose whole job is producing signatures, "was my key used, and for
//! what" should be answerable after the fact. Nothing else here can answer it:
//! the ledger knows what was *submitted*, which is not the same set.
//!
//! # What it does not contain
//!
//! Never the payload, never the secret. A signing log that copies the thing
//! being signed is a second place the interesting data lives.
//!
//! A multisign entry records the signing **digest** rather than a transaction
//! hash, because the hash is not knowable until the last signature is attached —
//! `tx submit` is the first stage that knows it.
//!
//! # It belongs to the backends, not to the stages
//!
//! Only key-record backends journal. The `ephemeral` backend — a seed supplied
//! for one invocation — writes nothing, which keeps `tx sign --seed-file` a pure
//! crypto stage with no filesystem dependency: it works on a read-only
//! container, in a sandbox, and in CI.
//!
//! Writing is **best-effort**. An unwritable journal is a warning on stderr and
//! never a failed signature; a backend that refuses to sign because it could not
//! write a log is worse than one that says the log is missing.

use std::fmt::Write as _;
use std::fs::OpenOptions;
use std::io::Write as _;
use std::path::PathBuf;

use xrpl::signer::SigningDomain;

use crate::store::Store;

/// An append-only log of signatures made on this machine.
pub struct Journal {
    path: PathBuf,
}

impl Journal {
    /// Open the journal for a store, if there is somewhere to put it.
    pub fn open(store: &Store) -> Option<Self> {
        Some(Self {
            path: store.locator().data_dir().join("signing.log"),
        })
    }

    /// Note one signature.
    pub fn note(&self, key_id: &str, address: &str, domain: &SigningDomain<'_>) {
        let mut line = String::new();

        // Seconds since the epoch: no date formatting dependency, and it sorts.
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|elapsed| elapsed.as_secs())
            .unwrap_or_default();

        let _ = write!(line, "{stamp}\t{key_id}\t{address}\t{domain}");

        if let Err(error) = self.append(&line) {
            crate::output::warn(format!(
                "could not write the signing journal ({}): {error}. The signature is unaffected.",
                self.path.display()
            ));
        }
    }

    fn append(&self, line: &str) -> std::io::Result<()> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            // Not a secret, but it is a list of what this machine signs.
            let _ = file.set_permissions(std::fs::Permissions::from_mode(0o600));
        }

        writeln!(file, "{line}")
    }

    /// Where the journal lives.
    pub fn path(&self) -> &PathBuf {
        &self.path
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_an_entry_records_the_key_and_the_domain_and_nothing_else() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = Store::at(dir.path());
        let journal = Journal::open(&store).expect("opens");

        journal.note(
            "alice-master",
            "rHb9CJAWyB4rj91VRWn96DkukG4bwdtyTh",
            &SigningDomain::Single,
        );

        let contents = std::fs::read_to_string(journal.path()).expect("read");
        assert!(contents.contains("alice-master"), "{contents}");
        assert!(
            contents.contains("rHb9CJAWyB4rj91VRWn96DkukG4bwdtyTh"),
            "{contents}"
        );
        assert!(contents.contains("Single"), "{contents}");
    }

    #[test]
    fn test_entries_append_rather_than_replace() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = Store::at(dir.path());
        let journal = Journal::open(&store).expect("opens");

        journal.note("one", "rA", &SigningDomain::Single);
        journal.note("two", "rB", &SigningDomain::Single);

        let contents = std::fs::read_to_string(journal.path()).expect("read");
        assert_eq!(contents.lines().count(), 2, "{contents}");
    }

    #[test]
    fn test_a_multisign_entry_names_the_signer_not_a_hash() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = Store::at(dir.path());
        let journal = Journal::open(&store).expect("opens");

        journal.note("alice", "rA", &SigningDomain::MultiAs("rSIGNER"));

        // The transaction hash is not knowable until the last signature is
        // attached, so there is nothing truthful to record here.
        let contents = std::fs::read_to_string(journal.path()).expect("read");
        assert!(contents.contains("MultiAs(rSIGNER)"), "{contents}");
    }

    #[test]
    fn test_an_unwritable_journal_does_not_panic() {
        // The whole point of best-effort: a signature must not depend on this.
        let store = Store::at("/this/path/does/not/exist/and/cannot/be/made");
        let journal = Journal::open(&store).expect("opens");

        journal.note("alice", "rA", &SigningDomain::Single);
    }
}
