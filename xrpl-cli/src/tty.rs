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

use std::fs::{File, OpenOptions};
use std::io::{IsTerminal, Write};
use std::process::Command;

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

/// Open a value in the user's editor and read back what they saved.
///
/// The editor's three descriptors are bound to `/dev/tty`, not inherited.
/// Inheriting them would hand `vi` a transaction on stdin and let it paint the
/// pipe with escape sequences — the artifact and the human share no descriptor
/// anywhere else in this CLI, and an editor is no exception.
///
/// The tempfile is 0600 and is removed on the way out, including on error: an
/// unsigned transaction is not a secret, but a file in `/tmp` that every process
/// can read is a bad habit to make an exception for.
pub fn edit(contents: &str, suffix: &str) -> Result<String, Error> {
    if !is_interactive() {
        return Err(SignerError::Declined(
            "there is no controlling terminal to open an editor on. \
             Edit the transaction with `jq`, or write it to a file and pass that."
                .into(),
        )
        .into());
    }

    let mut file = tempfile::Builder::new()
        .prefix("xrpl-tx-")
        .suffix(suffix)
        .rand_bytes(12)
        .tempfile()
        .map_err(Error::Io)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(file.path(), std::fs::Permissions::from_mode(0o600))
            .map_err(Error::Io)?;
    }

    file.write_all(contents.as_bytes()).map_err(Error::Io)?;
    file.flush().map_err(Error::Io)?;

    let command = editor_command();
    let mut words = command.split_whitespace();
    // `editor_command` never returns an empty string, so there is always a
    // first word; `vi` is a fallback rather than an `unwrap`.
    let program = words.next().unwrap_or("vi");

    let status = Command::new(program)
        .args(words)
        .arg(file.path())
        .stdin(File::open("/dev/tty").map_err(Error::Io)?)
        .stdout(tty_for_writing()?)
        .stderr(tty_for_writing()?)
        .status()
        .map_err(|error| {
            Error::other(format!(
                "could not run the editor `{program}`: {error}. \
                 Set $XRPL_EDITOR to one that exists."
            ))
        })?;

    if !status.success() {
        // A non-zero editor is how `:cq` says "discard this". Treating it as
        // success would submit the edit the user just refused.
        return Err(Error::other(format!(
            "`{program}` exited with {status}; the transaction is unchanged"
        )));
    }

    std::fs::read_to_string(file.path()).map_err(Error::Io)
}

/// The editor to run.
///
/// `XRPL_EDITOR` first, so a transaction can be opened somewhere other than
/// where prose is, and `vi` last because POSIX requires it to exist.
fn editor_command() -> String {
    for name in ["XRPL_EDITOR", "EDITOR", "VISUAL"] {
        if let Ok(value) = std::env::var(name) {
            if !value.trim().is_empty() {
                return value;
            }
        }
    }

    "vi".to_string()
}

fn tty_for_writing() -> Result<File, Error> {
    OpenOptions::new()
        .write(true)
        .open("/dev/tty")
        .map_err(Error::Io)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Run with one environment variable set, serialized against the other
    /// tests that read the same ones.
    fn with_editor_env<T>(values: &[(&str, Option<&str>)], body: impl FnOnce() -> T) -> T {
        use std::sync::{Mutex, OnceLock};
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        let _guard = LOCK
            .get_or_init(|| Mutex::new(()))
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());

        let names = ["XRPL_EDITOR", "EDITOR", "VISUAL"];
        let previous: Vec<_> = names.iter().map(|n| (*n, std::env::var_os(n))).collect();

        // Safety: the lock serializes every test that touches these.
        for name in names {
            unsafe { std::env::remove_var(name) };
        }
        for (name, value) in values {
            if let Some(value) = value {
                unsafe { std::env::set_var(name, value) };
            }
        }

        let result = body();

        for (name, value) in previous {
            match value {
                Some(value) => unsafe { std::env::set_var(name, value) },
                None => unsafe { std::env::remove_var(name) },
            }
        }

        result
    }

    #[test]
    fn test_the_editor_falls_back_to_vi() {
        // POSIX requires `vi`, so there is always something to open.
        assert_eq!(with_editor_env(&[], editor_command), "vi");
    }

    #[test]
    fn test_xrpl_editor_wins() {
        let chosen = with_editor_env(
            &[("XRPL_EDITOR", Some("my-editor")), ("EDITOR", Some("nano"))],
            editor_command,
        );

        assert_eq!(chosen, "my-editor");
    }

    #[test]
    fn test_an_empty_variable_is_not_a_choice() {
        // `EDITOR=` in a profile is a common way to mean "unset", and treating
        // it as a program name produces "could not run the editor ``".
        let chosen = with_editor_env(
            &[("XRPL_EDITOR", Some("  ")), ("EDITOR", Some("nano"))],
            editor_command,
        );

        assert_eq!(chosen, "nano");
    }

    #[test]
    fn test_an_editor_with_arguments_survives() {
        let chosen = with_editor_env(&[("EDITOR", Some("code --wait"))], editor_command);
        assert_eq!(chosen, "code --wait");
    }
}
