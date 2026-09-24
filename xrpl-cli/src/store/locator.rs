//! Where records live on disk.
//!
//! # Two directories, and why accounts are not in the config one
//!
//! Accounts and keys go in the **data** directory (`~/.local/share/xrpl`), not
//! the config directory. `~/.config` is what chezmoi, yadm and every other
//! dotfiles tool sweeps into a git repository, and an account record binds a
//! name to an r-address. Committing `alice = rXXXX` to a public dotfiles repo
//! permanently links a human identity to that account's entire transaction
//! graph — the address is on-chain forever and the git history is on GitHub
//! forever, so no later commit undoes it.
//!
//! Accounts are state, not configuration. The data directory is both the
//! semantically correct home and the cheapest thing that stops them being
//! synced by accident. `config.toml` holds defaults and nothing identifying.
//!
//! # One tier, deliberately
//!
//! There is no project-local `./.xrpl/` tier and there never will be.
//! stellar-cli built exactly that, shipped it, and then stopped reading it —
//! their locator now walks past every local directory and prints a deprecation
//! notice, with the enum left behind as vestigial code. Rebuilding it is a
//! known dead end.

use std::path::{Path, PathBuf};

use crate::error::Error;

/// Override for the data directory: accounts, keys, and anything else that is
/// state rather than preference.
pub const DATA_DIR_ENV: &str = "XRPL_DATA_DIR";

/// Override for the config directory.
pub const CONFIG_DIR_ENV: &str = "XRPL_CONFIG_DIR";

/// Where records are read from and written to.
#[derive(Debug, Clone)]
pub struct Locator {
    data: PathBuf,
    config: PathBuf,
}

impl Locator {
    /// Resolve both directories from the environment.
    pub fn from_env() -> Result<Self, Error> {
        Ok(Self {
            data: resolve(DATA_DIR_ENV, "XDG_DATA_HOME", ".local/share")?,
            config: resolve(CONFIG_DIR_ENV, "XDG_CONFIG_HOME", ".config")?,
        })
    }

    /// Point both directories at one place. For tests.
    pub fn at(root: impl Into<PathBuf>) -> Self {
        let root = root.into();
        Self {
            data: root.join("data"),
            config: root.join("config"),
        }
    }

    pub fn data_dir(&self) -> &Path {
        &self.data
    }

    pub fn config_dir(&self) -> &Path {
        &self.config
    }

    /// The directory holding account records.
    pub fn accounts_dir(&self) -> PathBuf {
        self.data.join("accounts")
    }

    /// The directory holding key records.
    pub fn keys_dir(&self) -> PathBuf {
        self.data.join("keys")
    }

    /// The path of one account record.
    pub fn account_path(&self, alias: &str) -> PathBuf {
        self.accounts_dir().join(format!("{alias}.toml"))
    }

    /// The path of one key record.
    pub fn key_path(&self, id: &str) -> PathBuf {
        self.keys_dir().join(format!("{id}.toml"))
    }

    /// The config file holding defaults.
    pub fn config_path(&self) -> PathBuf {
        self.config.join("config.toml")
    }

    /// Warn when the data directory is inside a git work tree with a remote.
    ///
    /// A warning rather than a refusal: someone may have decided to version
    /// their records deliberately, and that is theirs to decide. What they
    /// should not do is discover it after pushing.
    pub fn warn_if_versioned(&self) {
        let mut dir = self.data.as_path();

        loop {
            if dir.join(".git").exists() {
                crate::output::warn(format!(
                    "{} is inside a git work tree ({}). An account record binds a \
                     name to an r-address, and an address's transaction history is \
                     public and permanent — committing one links the two forever.",
                    self.data.display(),
                    dir.display()
                ));
                return;
            }

            match dir.parent() {
                Some(parent) => dir = parent,
                None => return,
            }
        }
    }
}

fn resolve(override_env: &str, xdg_env: &str, fallback: &str) -> Result<PathBuf, Error> {
    if let Some(path) = std::env::var_os(override_env) {
        return Ok(PathBuf::from(path));
    }

    if let Some(path) = std::env::var_os(xdg_env).filter(|p| !p.is_empty()) {
        return Ok(PathBuf::from(path).join("xrpl"));
    }

    let home = std::env::var_os("HOME")
        .filter(|h| !h.is_empty())
        .ok_or_else(|| {
            Error::other(format!(
                "cannot find a home directory; set {override_env} to say where records live"
            ))
        })?;

    Ok(PathBuf::from(home).join(fallback).join("xrpl"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_the_override_wins() {
        let dir = tempfile::tempdir().expect("tempdir");
        with_env(&[(DATA_DIR_ENV, Some(dir.path()))], || {
            let locator = Locator::from_env().expect("resolves");
            assert_eq!(locator.data_dir(), dir.path());
        });
    }

    #[test]
    fn test_xdg_is_honoured_when_there_is_no_override() {
        let dir = tempfile::tempdir().expect("tempdir");
        with_env(
            &[(DATA_DIR_ENV, None), ("XDG_DATA_HOME", Some(dir.path()))],
            || {
                let locator = Locator::from_env().expect("resolves");
                assert_eq!(locator.data_dir(), dir.path().join("xrpl"));
            },
        );
    }

    #[test]
    fn test_records_land_in_the_data_directory_not_the_config_one() {
        // The whole point: `~/.config` is what dotfiles tools sync, and an
        // account record binds a name to a permanent public address.
        let locator = Locator::at("/tmp/example");

        assert!(locator.accounts_dir().starts_with("/tmp/example/data"));
        assert!(locator.keys_dir().starts_with("/tmp/example/data"));
        assert!(locator.config_path().starts_with("/tmp/example/config"));
    }

    #[test]
    fn test_paths_are_one_file_per_record() {
        let locator = Locator::at("/tmp/example");

        assert!(locator
            .account_path("alice")
            .ends_with("accounts/alice.toml"));
        assert!(locator
            .key_path("alice-master")
            .ends_with("keys/alice-master.toml"));
    }

    /// Set several environment variables for the duration of a closure.
    ///
    /// Takes every variable at once rather than nesting: the lock is not
    /// reentrant, so a nested call would deadlock against itself — which is
    /// exactly what the first version of this helper did.
    fn with_env(vars: &[(&str, Option<&Path>)], body: impl FnOnce()) {
        use std::sync::{Mutex, OnceLock};
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();

        let _guard = LOCK
            .get_or_init(|| Mutex::new(()))
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());

        let previous: Vec<(&str, Option<std::ffi::OsString>)> = vars
            .iter()
            .map(|(key, _)| (*key, std::env::var_os(key)))
            .collect();

        for (key, value) in vars {
            // Safety: the lock above serializes every test that touches the
            // environment, and nothing else in this process reads it concurrently.
            match value {
                Some(path) => unsafe { std::env::set_var(key, path) },
                None => unsafe { std::env::remove_var(key) },
            }
        }

        body();

        for (key, value) in previous {
            match value {
                Some(value) => unsafe { std::env::set_var(key, value) },
                None => unsafe { std::env::remove_var(key) },
            }
        }
    }
}
