//! `xrpl key backends` — what this binary can sign with.
//!
//! Worth a verb because the answer differs between builds: `secure-store` is
//! behind a Cargo feature, and a key record naming it parses either way. Asking
//! is better than finding out at the moment you need a signature.

use serde_json::json;

use crate::error::Error;
use crate::output;
use crate::signer::registry;

#[derive(Debug, Clone, clap::Args)]
pub struct Cmd {
    /// Emit JSON on stdout.
    #[arg(long)]
    pub json: bool,
}

impl Cmd {
    pub fn run(&self) -> Result<(), Error> {
        let backends = registry::backends();

        if self.json {
            let value: Vec<_> = backends
                .iter()
                .map(|backend| {
                    json!({
                        "name": backend.name,
                        "available": backend.available,
                        "journals": backend.journals,
                        "enrolled": backend.enrolled,
                        "summary": backend.summary,
                    })
                })
                .collect();

            return output::artifact(&json!(value));
        }

        for backend in backends {
            output::note(format!(
                "{:<16}{:<15}{}",
                backend.name,
                if backend.available {
                    "available"
                } else {
                    "not built in"
                },
                backend.summary
            ));
        }

        if let Some(secure) = registry::find("secure-store") {
            if !secure.available {
                output::note(
                    "Rebuild with `--features secure-store` to use the OS credential store.",
                );
            }
        }

        Ok(())
    }
}
