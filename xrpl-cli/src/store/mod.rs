//! Reading and writing account and key records.

pub mod journal;
pub mod keychain;
pub mod locator;
pub mod record;
pub mod resolve;
pub mod secret;

use std::fs;
use std::path::Path;

use serde::de::DeserializeOwned;
use serde::Serialize;

pub use locator::Locator;
pub use record::{AccountRecord, KeyRecord, KeySource};

use crate::error::{Error, SignerError};

/// The store: a locator plus the operations over it.
#[derive(Debug, Clone)]
pub struct Store {
    locator: Locator,
}

impl Store {
    pub fn from_env() -> Result<Self, Error> {
        Ok(Self {
            locator: Locator::from_env()?,
        })
    }

    pub fn at(root: impl Into<std::path::PathBuf>) -> Self {
        Self {
            locator: Locator::at(root),
        }
    }

    pub fn locator(&self) -> &Locator {
        &self.locator
    }

    // -- accounts ----------------------------------------------------------

    pub fn account(&self, alias: &str) -> Result<AccountRecord, Error> {
        let record: AccountRecord = read(&self.locator.account_path(alias), "account", alias)?;
        record.validate(alias)?;
        Ok(record)
    }

    pub fn write_account(&self, alias: &str, record: &AccountRecord) -> Result<(), Error> {
        check_alias(alias)?;
        record.validate(alias)?;
        write(&self.locator.account_path(alias), record)
    }

    pub fn remove_account(&self, alias: &str) -> Result<(), Error> {
        remove(&self.locator.account_path(alias), "account", alias)
    }

    pub fn account_aliases(&self) -> Result<Vec<String>, Error> {
        list(&self.locator.accounts_dir())
    }

    // -- keys --------------------------------------------------------------

    pub fn key(&self, id: &str) -> Result<KeyRecord, Error> {
        let record: KeyRecord = read(&self.locator.key_path(id), "key", id)?;
        record.validate(id)?;
        Ok(record)
    }

    pub fn write_key(&self, id: &str, record: &KeyRecord) -> Result<(), Error> {
        check_alias(id)?;
        record.validate(id)?;
        write(&self.locator.key_path(id), record)
    }

    pub fn remove_key(&self, id: &str) -> Result<(), Error> {
        remove(&self.locator.key_path(id), "key", id)
    }

    pub fn key_ids(&self) -> Result<Vec<String>, Error> {
        list(&self.locator.keys_dir())
    }

    /// Every account that references a key, for the `rm` cascade check.
    pub fn accounts_using_key(&self, id: &str) -> Result<Vec<String>, Error> {
        let mut using = Vec::new();

        for alias in self.account_aliases()? {
            if let Ok(account) = self.account(&alias) {
                if account.keys.iter().any(|key| key == id) {
                    using.push(alias);
                }
            }
        }

        Ok(using)
    }

    // -- secrets -----------------------------------------------------------

    /// Write an encrypted blob and return the path to record.
    ///
    /// The path is relative to the data directory so a store stays portable: a
    /// record hard-coding `/Users/someone/...` would be wrong the moment it
    /// reached another machine, and records are meant to travel.
    pub fn write_secret(&self, id: &str, blob: &str) -> Result<String, Error> {
        check_alias(id)?;
        let relative = format!("secrets/{id}.age");
        write_text(&self.locator.data_dir().join(&relative), blob)?;

        Ok(relative)
    }

    /// Read an encrypted blob named by a record.
    pub fn read_secret(&self, relative: &str) -> Result<String, Error> {
        let path = self.locator.data_dir().join(relative);

        fs::read_to_string(&path).map_err(|error| {
            if error.kind() == std::io::ErrorKind::NotFound {
                // The record is here and the secret is not. That happens when
                // records sync between machines and secrets do not, which is
                // the normal state in a ceremony rather than a fault.
                Error::Signer(SignerError::Unavailable(format!(
                    "{} is not on this machine",
                    path.display()
                )))
            } else {
                Error::Io(error)
            }
        })
    }

    /// Read the encrypted blob a key record points at, wherever it lives.
    ///
    /// One reader for every backend, so signing, `key export` and `account
    /// doctor` cannot disagree about where a record's secret is or what its
    /// absence means.
    pub fn read_key_secret(&self, id: &str, record: &KeyRecord) -> Result<String, Error> {
        match &record.source {
            KeySource::EncryptedFile { path } => self.read_secret(path),
            KeySource::SecureStore { entry } => keychain::load(entry),
            // Exit 6, not 4: the record is here, and only the secret is not.
            KeySource::WatchOnly => Err(SignerError::Unavailable(format!(
                "{id} is watch-only: it has a public key and no secret"
            ))
            .into()),
        }
    }

    /// Delete the secret a key record points at, wherever it lives.
    ///
    /// Irreversible, and separate from forgetting the record: `key rm` only
    /// does this when asked.
    pub fn remove_key_secret(&self, record: &KeyRecord) -> Result<(), Error> {
        match &record.source {
            KeySource::EncryptedFile { path } => self.remove_secret(path),
            KeySource::SecureStore { entry } => keychain::remove(entry),
            KeySource::WatchOnly => Ok(()),
        }
    }

    /// Whether a recorded secret is present here.
    pub fn has_secret(&self, relative: &str) -> bool {
        self.locator.data_dir().join(relative).exists()
    }

    /// Forget an encrypted blob.
    pub fn remove_secret(&self, relative: &str) -> Result<(), Error> {
        let path = self.locator.data_dir().join(relative);
        if path.exists() {
            fs::remove_file(path).map_err(Error::Io)?;
        }

        Ok(())
    }

    // -- defaults ----------------------------------------------------------

    /// The default account alias, if one is set.
    pub fn default_account(&self) -> Option<String> {
        let text = fs::read_to_string(self.locator.config_path()).ok()?;
        let config: toml::Value = toml::from_str(&text).ok()?;

        config.get("default_account")?.as_str().map(str::to_string)
    }

    pub fn set_default_account(&self, alias: Option<&str>) -> Result<(), Error> {
        let path = self.locator.config_path();
        let mut config: toml::Table = fs::read_to_string(&path)
            .ok()
            .and_then(|text| toml::from_str(&text).ok())
            .unwrap_or_default();

        match alias {
            Some(alias) => {
                config.insert(
                    "default_account".into(),
                    toml::Value::String(alias.to_string()),
                );
            }
            None => {
                config.remove("default_account");
            }
        }

        write_text(&path, &toml::to_string_pretty(&config)?)
    }
}

/// Reject a name that would escape its directory or collide with the format.
fn check_alias(alias: &str) -> Result<(), Error> {
    if alias.is_empty() {
        return Err(Error::other("a name cannot be empty"));
    }

    // A record is one file named after it, so a name containing a separator
    // would write outside the store.
    if alias.contains(['/', '\\']) || alias.contains("..") {
        return Err(Error::other(format!(
            "{alias:?} cannot be used as a name: it would write outside the store"
        )));
    }

    if alias.starts_with('.') {
        return Err(Error::other(format!(
            "{alias:?} cannot be used as a name: names beginning with a dot are hidden"
        )));
    }

    Ok(())
}

fn read<T: DeserializeOwned>(path: &Path, kind: &'static str, name: &str) -> Result<T, Error> {
    let text = fs::read_to_string(path).map_err(|error| {
        if error.kind() == std::io::ErrorKind::NotFound {
            Error::Signer(SignerError::NotFound {
                kind,
                name: name.to_string(),
            })
        } else {
            Error::Io(error)
        }
    })?;

    toml::from_str(&text).map_err(|error| Error::other(format!("{}: {error}", path.display())))
}

/// Write a record atomically, with the permissions a record should have.
///
/// Through a temporary file and a rename, so a crash or a concurrent reader
/// never sees a half-written record — and so two concurrent writers produce one
/// whole record rather than an interleaved one.
fn write<T: Serialize>(path: &Path, value: &T) -> Result<(), Error> {
    write_text(path, &toml::to_string_pretty(value)?)
}

fn write_text(path: &Path, contents: &str) -> Result<(), Error> {
    let dir = path
        .parent()
        .ok_or_else(|| Error::other(format!("{} has no parent directory", path.display())))?;

    fs::create_dir_all(dir).map_err(Error::Io)?;
    harden_dir(dir)?;

    let temporary = tempfile_in(dir)?;
    fs::write(&temporary, contents).map_err(Error::Io)?;
    harden_file(&temporary)?;
    fs::rename(&temporary, path).map_err(Error::Io)?;

    Ok(())
}

fn tempfile_in(dir: &Path) -> Result<std::path::PathBuf, Error> {
    // A name nothing else will pick: the process id plus a counter.
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);

    Ok(dir.join(format!(
        ".xrpl-{}-{}.tmp",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    )))
}

fn remove(path: &Path, kind: &'static str, name: &str) -> Result<(), Error> {
    fs::remove_file(path).map_err(|error| {
        if error.kind() == std::io::ErrorKind::NotFound {
            Error::Signer(SignerError::NotFound {
                kind,
                name: name.to_string(),
            })
        } else {
            Error::Io(error)
        }
    })
}

fn list(dir: &Path) -> Result<Vec<String>, Error> {
    let Ok(entries) = fs::read_dir(dir) else {
        // An empty store is not an error; it is where everyone starts.
        return Ok(Vec::new());
    };

    let mut names: Vec<String> = entries
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let path = entry.path();
            if path.extension()? != "toml" {
                return None;
            }
            Some(path.file_stem()?.to_string_lossy().into_owned())
        })
        .filter(|name| !name.starts_with('.'))
        .collect();

    names.sort();

    Ok(names)
}

#[cfg(unix)]
fn harden_dir(dir: &Path) -> Result<(), Error> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(dir, fs::Permissions::from_mode(0o700)).map_err(Error::Io)
}

#[cfg(unix)]
fn harden_file(path: &Path) -> Result<(), Error> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o600)).map_err(Error::Io)
}

#[cfg(not(unix))]
fn harden_dir(_dir: &Path) -> Result<(), Error> {
    Ok(())
}

#[cfg(not(unix))]
fn harden_file(_path: &Path) -> Result<(), Error> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const ADDRESS: &str = "rHb9CJAWyB4rj91VRWn96DkukG4bwdtyTh";
    const OTHER: &str = "rPT1Sjq2YGrBMTttX4GZHjKu9dyfzbpAYe";

    fn store() -> (tempfile::TempDir, Store) {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = Store::at(dir.path());
        (dir, store)
    }

    #[test]
    fn test_an_account_round_trips() {
        let (_dir, store) = store();
        let mut record = AccountRecord::new(ADDRESS.into());
        record.network_id = Some(1);

        store.write_account("alice", &record).expect("writes");
        assert_eq!(store.account("alice").expect("reads"), record);
    }

    #[test]
    fn test_an_empty_store_lists_nothing_rather_than_failing() {
        let (_dir, store) = store();

        assert!(store.account_aliases().expect("lists").is_empty());
        assert!(store.key_ids().expect("lists").is_empty());
    }

    #[test]
    fn test_a_missing_record_is_not_found_rather_than_an_io_error() {
        let (_dir, store) = store();

        // Exit code 4 depends on this being distinguishable.
        let error = store.account("nobody").unwrap_err();
        assert!(
            matches!(error, Error::Signer(SignerError::NotFound { .. })),
            "{error:?}"
        );
    }

    #[test]
    fn test_listing_is_sorted_and_skips_non_records() {
        let (dir, store) = store();
        for alias in ["carol", "alice", "bob"] {
            store
                .write_account(alias, &AccountRecord::new(ADDRESS.into()))
                .expect("writes");
        }
        fs::write(dir.path().join("data/accounts/notes.txt"), "ignored").expect("write");

        assert_eq!(
            store.account_aliases().expect("lists"),
            ["alice", "bob", "carol"]
        );
    }

    #[cfg(unix)]
    #[test]
    fn test_records_and_their_directory_are_private() {
        use std::os::unix::fs::PermissionsExt;
        let (_dir, store) = store();

        store
            .write_account("alice", &AccountRecord::new(ADDRESS.into()))
            .expect("writes");

        let file = fs::metadata(store.locator().account_path("alice")).expect("stat");
        let dir = fs::metadata(store.locator().accounts_dir()).expect("stat");

        assert_eq!(file.permissions().mode() & 0o777, 0o600);
        assert_eq!(dir.permissions().mode() & 0o777, 0o700);
    }

    #[test]
    fn test_no_temporary_file_survives_a_write() {
        let (_dir, store) = store();
        store
            .write_account("alice", &AccountRecord::new(ADDRESS.into()))
            .expect("writes");

        // The rename is what makes a write atomic; a leftover temp file would
        // mean it did not happen.
        let leftovers: Vec<_> = fs::read_dir(store.locator().accounts_dir())
            .expect("read dir")
            .filter_map(Result::ok)
            .filter(|entry| entry.file_name().to_string_lossy().ends_with(".tmp"))
            .collect();

        assert!(leftovers.is_empty(), "{leftovers:?}");
    }

    #[test]
    fn test_a_name_cannot_escape_the_store() {
        let (_dir, store) = store();
        let record = AccountRecord::new(ADDRESS.into());

        for bad in ["../escape", "sub/dir", "..", ".hidden", ""] {
            assert!(
                store.write_account(bad, &record).is_err(),
                "{bad:?} should be refused as a name"
            );
        }
    }

    #[test]
    fn test_removing_a_missing_record_says_so() {
        let (_dir, store) = store();

        assert!(matches!(
            store.remove_account("nobody").unwrap_err(),
            Error::Signer(SignerError::NotFound { .. })
        ));
    }

    #[test]
    fn test_accounts_using_a_key_are_found() {
        let (_dir, store) = store();

        let mut alice = AccountRecord::new(ADDRESS.into());
        alice.keys = vec!["shared".into()];
        store.write_account("alice", &alice).expect("writes");

        let mut bob = AccountRecord::new(OTHER.into());
        bob.keys = vec!["shared".into(), "other".into()];
        store.write_account("bob", &bob).expect("writes");

        assert_eq!(
            store.accounts_using_key("shared").expect("scans"),
            ["alice", "bob"]
        );
        assert_eq!(store.accounts_using_key("other").expect("scans"), ["bob"]);
    }

    #[test]
    fn test_the_default_account_round_trips() {
        let (_dir, store) = store();

        assert_eq!(store.default_account(), None);
        store.set_default_account(Some("alice")).expect("sets");
        assert_eq!(store.default_account().as_deref(), Some("alice"));
        store.set_default_account(None).expect("clears");
        assert_eq!(store.default_account(), None);
    }

    #[test]
    fn test_a_corrupt_record_names_its_file() {
        let (dir, store) = store();
        fs::create_dir_all(dir.path().join("data/accounts")).expect("mkdir");
        fs::write(dir.path().join("data/accounts/alice.toml"), "not = [toml").expect("write");

        let message = store.account("alice").unwrap_err().to_string();
        assert!(message.contains("alice.toml"), "{message}");
    }
}
