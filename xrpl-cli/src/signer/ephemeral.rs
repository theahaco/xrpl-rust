//! The `ephemeral` signing backend: a seed supplied for one invocation.
//!
//! The seed is read, used, and forgotten. Nothing is enrolled and nothing is
//! written — which is what makes `--seed-file <(op read op://vault/issuer/seed)`
//! the intended shape: the CLI never authors a seed file, it only reads one the
//! user already owns.
//!
//! # Where a seed may come from, in order
//!
//! 1. `--seed-file <path>` — refused if the file or its directory is group- or
//!    world-readable.
//! 2. `XRPL_SEED` — declared with clap's `env`, so it shows in `--help` without
//!    showing its value.
//! 3. `--seed` — hidden, deprecated, and warned about on stderr. It puts a
//!    secret in `argv`, where `ps` and the shell history can see it.
//! 4. A prompt on the controlling terminal, when there is one.
//!
//! **Not stdin.** Every stage of the pipeline reads its transaction from stdin,
//! so the seed cannot travel that way. `key add --seed-stdin` is the enrolment
//! path, where nothing else is competing for the descriptor.
//!
//! `-s` is deliberately absent. It means `--seed` on six commands today, and
//! repointing a short flag at a key-selection flag would leave every existing
//! script parsing while silently reinterpreting a seed as an alias — a
//! wrong-key signature, not an error.

use std::fs;
use std::path::{Path, PathBuf};

use xrpl::wallet::Wallet;
use zeroize::Zeroize;

use crate::error::Error;
use crate::output;
use crate::tty;

/// How a signing command is told which key to use.
///
/// Flattened into the leaves that sign, and only those: `tx new` builds JSON and
/// does not sign, so it does not carry this.
#[derive(Clone, clap::Args)]
pub struct SigningArgs {
    /// Path to a file whose first line is a seed.
    ///
    /// The file is plaintext and protecting it is yours to do; the CLI reads it
    /// and never writes one. Refused if it, or its directory, is readable by
    /// anyone else. Process substitution works: `--seed-file <(op read op://…)`.
    #[arg(long, value_name = "PATH")]
    pub seed_file: Option<PathBuf>,

    /// A seed, from the environment.
    #[arg(
        long = "xrpl-seed",
        env = "XRPL_SEED",
        hide_env_values = true,
        hide = true
    )]
    pub seed_env: Option<String>,

    /// Deprecated: a seed in `argv`, where `ps` and shell history can read it.
    #[arg(long, hide = true)]
    pub seed: Option<String>,

    /// Sign with a recorded key, by id.
    ///
    /// Repeatable: passing it twice means "produce a multisigned transaction",
    /// which is the same as running the signing stage once per key.
    #[arg(long = "sign-with", value_name = "KEY_ID")]
    pub sign_with: Vec<String>,

    /// Reserved: the seed's algorithm is what decides the curve today.
    #[arg(long, value_name = "ALGORITHM", hide = true)]
    pub algorithm: Option<String>,
}

/// `Debug` never shows a seed, whichever rung it arrived on.
///
/// Every seed-bearing command struct in this crate derives `Debug` over a plain
/// `String` today, which is the exact inverse of the care `Wallet` takes.
impl core::fmt::Debug for SigningArgs {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("SigningArgs")
            .field("seed_file", &self.seed_file)
            .field("seed_env", &self.seed_env.as_ref().map(|_| "<redacted>"))
            .field("seed", &self.seed.as_ref().map(|_| "<redacted>"))
            .field("sign_with", &self.sign_with)
            .field("algorithm", &self.algorithm)
            .finish()
    }
}

/// Whatever is going to sign: a recorded key, or a seed for this invocation.
///
/// Both are `RawSigner`s, so nothing downstream of here knows the difference —
/// which is the point of having put the trait in the library.
pub enum ResolvedSigner {
    Stored(crate::signer::StoredSigner),
    Ephemeral(Box<Wallet>),
}

impl xrpl::signer::RawSigner for ResolvedSigner {
    fn public_key(&self) -> &str {
        match self {
            ResolvedSigner::Stored(signer) => signer.public_key(),
            ResolvedSigner::Ephemeral(wallet) => wallet.public_key(),
        }
    }

    fn algorithm(&self) -> xrpl::constants::CryptoAlgorithm {
        match self {
            ResolvedSigner::Stored(signer) => signer.algorithm(),
            ResolvedSigner::Ephemeral(wallet) => wallet.algorithm(),
        }
    }

    fn classic_address(&self) -> xrpl::signer::XRPLSignerResult<String> {
        match self {
            ResolvedSigner::Stored(signer) => signer.classic_address(),
            ResolvedSigner::Ephemeral(wallet) => wallet.classic_address(),
        }
    }

    fn sign(
        &self,
        domain: xrpl::signer::SigningDomain<'_>,
        payload: &[u8],
    ) -> xrpl::signer::XRPLSignerResult<xrpl::signer::SignOutcome> {
        match self {
            ResolvedSigner::Stored(signer) => signer.sign(domain, payload),
            ResolvedSigner::Ephemeral(wallet) => wallet.sign(domain, payload),
        }
    }
}

impl SigningArgs {
    /// Which key ids were named, if any.
    pub fn key_ids(&self) -> &[String] {
        &self.sign_with
    }

    /// Resolve whatever is going to sign.
    ///
    /// A recorded key wins when one is named; otherwise the seed ladder. The
    /// unlock happens here, before any runtime is entered and before the first
    /// transaction is read, so a prompt can never sit inside a `block_on` and a
    /// stream never asks twice.
    pub fn signer(&self) -> Result<ResolvedSigner, Error> {
        match self.sign_with.as_slice() {
            [] => Ok(ResolvedSigner::Ephemeral(Box::new(self.resolve()?))),
            [id] => {
                let store = crate::store::Store::from_env()?;
                Ok(ResolvedSigner::Stored(crate::signer::StoredSigner::unlock(
                    &store, id,
                )?))
            }
            _ => Err(Error::other(
                "--sign-with was given more than once, which means a multisigned \
                 transaction: use `tx sign --multisign` once per key",
            )),
        }
    }

    /// Resolve a seed and build the wallet it derives.
    ///
    /// The parsed seed is zeroized before returning, whichever rung produced it.
    /// That shrinks the window; it does not close it, because the ledger's own
    /// encoding means the seed exists as a `String` for as long as it takes to
    /// derive a key.
    pub fn resolve(&self) -> Result<Wallet, Error> {
        if self.algorithm.is_some() {
            return Err(Error::other(
                "--algorithm is reserved: the seed's own prefix decides the curve today",
            ));
        }

        let mut seed = self.read_seed()?;
        let wallet = Wallet::new(&seed, 0);
        seed.zeroize();

        Ok(wallet?)
    }

    fn read_seed(&self) -> Result<String, Error> {
        if let Some(path) = &self.seed_file {
            return read_seed_file(path);
        }

        if let Some(seed) = &self.seed_env {
            return Ok(seed.trim().to_string());
        }

        if let Some(seed) = &self.seed {
            output::warn(
                "--seed puts a secret in argv, where `ps` and your shell history can read it. \
                 Use --seed-file or XRPL_SEED; this flag will be removed.",
            );
            return Ok(seed.trim().to_string());
        }

        tty::prompt_secret("seed")
    }
}

/// Read the first line of a seed file, refusing one others can read.
pub(crate) fn read_seed_file(path: &Path) -> Result<String, Error> {
    reject_if_group_or_world_readable(path)?;
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        // A 0600 file in a 0755 directory is still a file anyone can find; the
        // directory is half of the protection.
        reject_if_group_or_world_readable(parent)?;
    }

    let contents = fs::read_to_string(path).map_err(Error::Io)?;
    let seed = contents
        .lines()
        .next()
        .unwrap_or_default()
        .trim()
        .to_string();

    if seed.is_empty() {
        return Err(Error::other(format!(
            "{} is empty; expected a seed on its first line",
            path.display()
        )));
    }

    Ok(seed)
}

#[cfg(unix)]
fn reject_if_group_or_world_readable(path: &Path) -> Result<(), Error> {
    use std::os::unix::fs::PermissionsExt;

    let metadata = fs::metadata(path).map_err(Error::Io)?;
    // Process substitution (`<(op read …)`) produces a pipe or /dev/fd entry
    // whose mode is not the caller's to control, and which no other user can
    // open anyway. Only regular files and directories are checked.
    let file_type = metadata.file_type();
    if !file_type.is_file() && !file_type.is_dir() {
        return Ok(());
    }

    let mode = metadata.permissions().mode();
    if mode & 0o077 != 0 {
        return Err(Error::other(format!(
            "{} is readable by other users (mode {:o}); run `chmod go-rwx {}`",
            path.display(),
            mode & 0o777,
            path.display()
        )));
    }

    Ok(())
}

#[cfg(not(unix))]
fn reject_if_group_or_world_readable(_path: &Path) -> Result<(), Error> {
    // No portable equivalent of the Unix mode bits. Saying so is better than
    // implying a check that is not happening.
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn write_seed(dir: &Path, name: &str, contents: &str, mode: u32) -> PathBuf {
        let path = dir.join(name);
        let mut file = fs::File::create(&path).expect("create");
        file.write_all(contents.as_bytes()).expect("write");
        drop(file);

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&path, fs::Permissions::from_mode(mode)).expect("chmod");
        }
        let _ = mode;

        path
    }

    fn private_dir() -> tempfile::TempDir {
        let dir = tempfile::tempdir().expect("tempdir");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o700)).expect("chmod");
        }
        dir
    }

    #[test]
    fn test_reads_the_first_line_only() {
        let dir = private_dir();
        let path = write_seed(
            dir.path(),
            "seed",
            "snoPBrXtMeMyMHUVTgbuqAfg1SUTb\nnot the seed\n",
            0o600,
        );

        assert_eq!(
            read_seed_file(&path).expect("reads"),
            "snoPBrXtMeMyMHUVTgbuqAfg1SUTb"
        );
    }

    #[test]
    fn test_trailing_whitespace_is_tolerated() {
        let dir = private_dir();
        let path = write_seed(
            dir.path(),
            "seed",
            "  snoPBrXtMeMyMHUVTgbuqAfg1SUTb  \n",
            0o600,
        );

        assert_eq!(
            read_seed_file(&path).expect("reads"),
            "snoPBrXtMeMyMHUVTgbuqAfg1SUTb"
        );
    }

    #[test]
    fn test_an_empty_file_is_an_error_not_an_empty_seed() {
        let dir = private_dir();
        let path = write_seed(dir.path(), "seed", "\n", 0o600);

        assert!(read_seed_file(&path).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn test_a_world_readable_seed_file_is_refused() {
        let dir = private_dir();
        let path = write_seed(dir.path(), "seed", "snoPBrXtMeMyMHUVTgbuqAfg1SUTb\n", 0o644);

        let message = read_seed_file(&path).unwrap_err().to_string();
        assert!(message.contains("readable by other users"), "{message}");
    }

    #[cfg(unix)]
    #[test]
    fn test_a_private_file_in_a_public_directory_is_refused() {
        // 0600 in a 0755 directory is still a file anyone can find and watch.
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().expect("tempdir");
        fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o755)).expect("chmod");
        let path = write_seed(dir.path(), "seed", "snoPBrXtMeMyMHUVTgbuqAfg1SUTb\n", 0o600);

        assert!(read_seed_file(&path).is_err());
    }

    #[test]
    fn test_debug_never_prints_a_seed() {
        let args = SigningArgs {
            seed_file: None,
            seed_env: Some("snoPBrXtMeMyMHUVTgbuqAfg1SUTb".into()),
            seed: Some("snoPBrXtMeMyMHUVTgbuqAfg1SUTb".into()),
            sign_with: Vec::new(),
            algorithm: None,
        };

        let rendered = format!("{args:?}");
        assert!(!rendered.contains("snoPBrXt"), "{rendered}");
        assert!(rendered.contains("redacted"), "{rendered}");
    }

    #[test]
    fn test_two_keys_means_multisign_and_says_so() {
        let args = SigningArgs {
            seed_file: None,
            seed_env: None,
            seed: None,
            sign_with: vec!["alice".into(), "bob".into()],
            algorithm: None,
        };

        // Two keys is a multisigned transaction, which is the multisign stage
        // run twice — not one signer with two keys.
        let message = args.signer().err().expect("should refuse").to_string();
        assert!(message.contains("--multisign"), "{message}");
    }

    #[test]
    fn test_key_ids_are_reported() {
        let args = SigningArgs {
            seed_file: None,
            seed_env: None,
            seed: None,
            sign_with: vec!["alice".into()],
            algorithm: None,
        };

        assert_eq!(args.key_ids(), ["alice"]);
    }
}
