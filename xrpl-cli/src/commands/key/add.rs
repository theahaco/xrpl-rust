//! `xrpl key add` — record a public key.

use std::io::Read;

use xrpl::core::keypairs::derive_classic_address;
use zeroize::Zeroize;

use crate::error::Error;
use crate::output;
use crate::store::{KeyRecord, KeySource, Store};

#[derive(Debug, Clone, clap::Args)]
pub struct Cmd {
    /// What to call this key. You choose the name; `--key` uses it.
    pub id: String,

    /// Record a public key only: it can be recognized, never used.
    #[arg(long, value_name = "HEX", conflicts_with_all = ["seed_file", "seed_stdin"])]
    pub public_key: Option<String>,

    /// Enrol the seed in this file. It is encrypted before being stored.
    #[arg(long, value_name = "PATH")]
    pub seed_file: Option<std::path::PathBuf>,

    /// Enrol a seed read from stdin.
    ///
    /// Available here and nowhere else: every `tx` stage reads its transaction
    /// from stdin, so a seed cannot travel that way once a pipeline is running.
    /// At enrolment nothing else wants the descriptor.
    #[arg(long, conflicts_with = "seed_file")]
    pub seed_stdin: bool,

    /// Replace an existing record of the same name.
    #[arg(long)]
    pub force: bool,

    /// Keep the encrypted blob in the OS credential store, not in a file.
    ///
    /// It is the same encrypted blob either way — this changes where it lives,
    /// so it is not in a dotfiles sync or a backup of the home directory.
    #[arg(long)]
    pub secure_store: bool,

    /// Enrol without confirming the derived address.
    ///
    /// The confirmation only appears when there is a terminal, so a script
    /// never needs this — it is for a person who would rather not be asked.
    #[arg(long, short = 'y')]
    pub yes: bool,
}

impl Cmd {
    pub fn run(&self) -> Result<(), Error> {
        let store = Store::from_env()?;
        store.locator().warn_if_versioned();

        if !self.force && store.key(&self.id).is_ok() {
            return Err(Error::other(format!(
                "a key named {:?} already exists; pass --force to replace it",
                self.id
            )));
        }

        if let Some(seed) = self.read_seed()? {
            let mut seed = seed;
            let record = super::enrol::enrol(&store, &self.id, &seed, self.enrolment())?;
            seed.zeroize();

            output::note(format!(
                "enrolled {} in the {} backend, encrypted at rest",
                self.id,
                record.source.label()
            ));
            return output::artifact(&super::describe(&self.id, &record));
        }

        let public_key = self
            .public_key
            .as_ref()
            .ok_or_else(|| {
                Error::other(
                    "nothing to record: pass --public-key for a watch-only key, or \
                     --seed-file / --seed-stdin to enrol one that can sign",
                )
            })?
            .to_uppercase();

        // Derived rather than asked for, and cached, so listing records never
        // has to compute anything.
        let classic_address = derive_classic_address(&public_key).map_err(|error| {
            Error::other(format!(
                "{:?} is not a public key: {error}",
                self.public_key
            ))
        })?;

        // The curve is read from the key's own prefix here because the key is
        // in hand. A backend that never exposes one carries it explicitly,
        // which is why the record stores it rather than re-deriving it later.
        let algorithm = if public_key.starts_with("ED") {
            "ed25519"
        } else {
            "secp256k1"
        };

        let record = KeyRecord::new(
            public_key,
            algorithm.into(),
            classic_address,
            // The only source that can be created without a secret backend.
            KeySource::WatchOnly,
        );

        store.write_key(&self.id, &record)?;
        output::note(format!(
            "recorded {} as watch-only: it can be recognized, not used",
            self.id
        ));

        output::artifact(&super::describe(&self.id, &record))
    }

    fn enrolment(&self) -> super::enrol::Enrolment {
        super::enrol::Enrolment {
            backend: if self.secure_store {
                super::enrol::Backend::SecureStore
            } else {
                super::enrol::Backend::EncryptedFile
            },
            confirm: !self.yes,
        }
    }

    /// The seed to enrol, if one was offered.
    fn read_seed(&self) -> Result<Option<String>, Error> {
        if let Some(path) = &self.seed_file {
            // The same reader the signing ladder uses, so enrolment and signing
            // agree about what an acceptable seed file is — including refusing
            // one whose directory anyone can read.
            return Ok(Some(crate::signer::ephemeral::read_seed_file(path)?));
        }

        if self.seed_stdin {
            let mut buffer = String::new();
            std::io::stdin()
                .read_to_string(&mut buffer)
                .map_err(Error::Io)?;
            let seed = buffer.lines().next().unwrap_or_default().trim().to_string();
            buffer.zeroize();

            if seed.is_empty() {
                return Err(Error::other("nothing on stdin"));
            }

            return Ok(Some(seed));
        }

        Ok(None)
    }
}
