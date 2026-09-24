//! Talking to the terminal directly, rather than through stdin or stdout.
//!
//! Every stage of the transaction pipeline reads its transaction from stdin and
//! writes its result to stdout, so neither descriptor is free for talking to a
//! human: a prompt that read stdin would eat the transaction, and one that wrote
//! stdout would corrupt the artifact.
//!
//! So prompts go to the controlling terminal. When there is none — in CI, in a
//! container, inside a pipeline — there is nothing to prompt on, and the caller
//! must fail with a message naming the non-interactive way in rather than
//! blocking on a question nobody can answer.

use std::fs::File;
use std::io::IsTerminal;

use crate::error::{Error, SignerError};

/// Whether a human could answer a prompt right now.
///
/// Checks for a controlling terminal rather than for stdin, deliberately: stdin
/// is a pipe in every pipeline, and that says nothing about whether a person is
/// watching.
pub fn is_interactive() -> bool {
    std::io::stderr().is_terminal() && File::open("/dev/tty").is_ok()
}

/// Prompt on the terminal and read one line back with echo disabled.
///
/// Returns [`SignerError::Declined`] when there is no controlling terminal, so a
/// pipeline fails immediately and names its alternative instead of hanging.
pub fn prompt_secret(prompt: &str) -> Result<String, Error> {
    if !is_interactive() {
        return Err(SignerError::Declined(format!(
            "{prompt}: no controlling terminal, and stdin carries the transaction. \
             Use --seed-file or XRPL_SEED."
        ))
        .into());
    }

    // `rpassword` opens the tty itself and restores the terminal state on the
    // way out, including on error. Rolling this by hand means a `termios`
    // struct whose layout differs between macOS and Linux, for one prompt.
    rpassword::prompt_password(format!("{prompt}: ")).map_err(Error::Io)
}
