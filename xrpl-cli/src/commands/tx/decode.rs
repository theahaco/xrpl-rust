//! `xrpl tx decode` — parse a hex blob back into transaction JSON. Offline.

use std::ffi::OsString;
use std::io::{IsTerminal, Read};
use std::path::Path;

use xrpl::core::binarycodec::decode;

use crate::commands::tx::io;
use crate::error::Error;

#[derive(Debug, Clone, clap::Args)]
pub struct Cmd {
    /// A hex transaction blob, a file containing one, or `-`/empty for stdin.
    #[arg(value_name = "BLOB")]
    pub blob: Option<OsString>,
}

impl Cmd {
    pub fn run(&self) -> Result<(), Error> {
        for blob in self.read_blobs()? {
            let decoded: serde_json::Value = decode(&blob)?;
            crate::output::artifact(&decoded)?;
        }

        Ok(())
    }

    fn read_blobs(&self) -> Result<Vec<String>, Error> {
        let raw = match &self.blob {
            Some(value) if value != "-" => {
                let text = value.to_string_lossy().into_owned();
                if Path::new(value).exists() {
                    std::fs::read_to_string(value).map_err(Error::Io)?
                } else {
                    text
                }
            }
            _ => {
                if std::io::stdin().is_terminal() {
                    return Err(Error::other(
                        "no blob provided: pipe one in, pass a path, or give hex as an argument",
                    ));
                }
                let mut buffer = String::new();
                std::io::stdin()
                    .read_to_string(&mut buffer)
                    .map_err(Error::Io)?;
                buffer
            }
        };

        let blobs: Vec<String> = raw
            .lines()
            .map(str::trim)
            // A blob that came through `tx blob` is a JSON string, quotes and
            // all, because stdout carries JSON. Accept both spellings.
            .map(|line| line.trim_matches('"').to_string())
            .filter(|line| !line.is_empty())
            .collect();

        if blobs.is_empty() {
            return Err(Error::other("no blob provided: input was empty"));
        }

        Ok(blobs)
    }
}

// `io` is imported for symmetry with the other stages; decode reads hex, not
// transaction JSON, so it does not use `read_txs`.
#[allow(unused_imports)]
use io as _unused_io;
