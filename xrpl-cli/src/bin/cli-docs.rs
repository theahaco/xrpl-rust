//! Regenerates `xrpl-cli/CLI.md`.
//!
//! The sibling of `tx-surface`, over the other half of the surface: that one
//! generates the `tx new` fields from `definitions.json`, this one generates
//! the commands from the `clap` definitions. Neither covers the other, and
//! this walker stops at `tx new` rather than re-emitting what `TX_SURFACE.md`
//! already holds.
//!
//! Hidden items are dropped. `xrpl wallet` and `--seed` are hidden because they
//! are deprecated — a seed in `argv` is readable by `ps` and the shell history —
//! and a generated reference that documented them would outlive the deprecation
//! it exists to end. `is_hide_set()` on both `Command` and `Arg` is the one
//! source of truth for that, so nothing here maintains a list of names.
//!
//! Hand-written rather than `clap-markdown`, which was the obvious candidate:
//! it skips hidden subcommands and hidden options, but its positional loop
//! (`get_positionals()`) has no `is_hide_set()` filter, so the exclusion this
//! file exists for holds only until someone hides a positional. It also
//! recurses unconditionally, which would re-emit every `tx new` type here, and
//! offers no hook for the key-material disclosure below.
//!
//! The command is deliberately not `build()`-ed. An unbuilt `Command` has no
//! generated `help` subcommand and no `--help`/`--version` arguments, and the
//! `--quiet` global has not yet been copied onto every leaf — so the walk sees
//! the surface as written, and `--quiet` is documented once, where it is
//! declared.

use clap::{Arg, ArgAction, Command, CommandFactory};

use xrpl_cli::commands::Cli;

/// The one command whose children are documented elsewhere.
///
/// `tx new` has one subcommand per transaction type, each with the fields that
/// type accepts. That is generated from `definitions.json` and already checked
/// in; repeating it here would be a second copy to keep in step.
const GENERATED_ELSEWHERE: &str = "xrpl tx new";

const PREAMBLE: &str = "\
# `xrpl` command reference

Generated from the `clap` definitions. Regenerate with
`cargo run -p xrpl-cli --bin cli-docs > xrpl-cli/CLI.md`; CI fails if this file
and the code disagree.

Hidden commands and flags are left out. `xrpl wallet` and `--seed` are
deprecated and hidden because a seed in `argv` is readable by `ps` and the shell
history; documenting them here would outlive the deprecation.

The fields each `xrpl tx new <type>` accepts are generated separately, from
`definitions.json`, and live in [TX_SURFACE.md](TX_SURFACE.md). This file stops
at the command.

Key material is **encrypted at rest, plaintext in process memory at signing
time**, and nothing stronger: it defends against a stolen laptop, a record
committed to a repository, a dotfiles sync and a backup, but not against malware
running as you while you sign.
";

fn main() {
    print!("{}", document());
}

/// The whole reference, so the tests below can read what CI diffs.
fn document() -> String {
    let mut out = String::from(PREAMBLE);
    render(&mut out, &Cli::command(), "xrpl");
    out
}

/// Write one command and, unless its children live elsewhere, its subcommands.
fn render(out: &mut String, command: &Command, path: &str) {
    out.push_str(&format!("\n## `{path}`\n"));

    // The long about is the whole explanation; the short about is its first
    // line. Printing both would repeat that line.
    if let Some(about) = command.get_long_about().or_else(|| command.get_about()) {
        out.push_str(&format!("\n{}\n", about.to_string().trim_end()));
    }

    // Before the subcommand list, not after: `tx new` has one subcommand per
    // transaction type, and naming all of them here is the duplication this
    // stops. Its own arguments are still worth printing, so this is not a
    // return.
    let generated_elsewhere = path == GENERATED_ELSEWHERE;
    if generated_elsewhere {
        out.push_str(
            "\nOne subcommand per transaction type, each with the fields that type\n\
             accepts: see [TX_SURFACE.md](TX_SURFACE.md).\n",
        );
    }

    let subcommands: Vec<&Command> = command
        .get_subcommands()
        .filter(|sub| !sub.is_hide_set())
        .collect();

    if !generated_elsewhere && !subcommands.is_empty() {
        out.push_str("\nSubcommands:\n\n");
        for sub in &subcommands {
            out.push_str(&format!(
                "- `{}` — {}\n",
                sub.get_name(),
                sub.get_about().map(one_line).unwrap_or_default()
            ));
        }
    }

    let visible = || command.get_arguments().filter(|arg| !arg.is_hide_set());

    let positionals: Vec<&Arg> = visible().filter(|arg| arg.is_positional()).collect();
    if !positionals.is_empty() {
        out.push_str("\nArguments:\n\n");
        for arg in positionals {
            out.push_str(&render_arg(arg));
        }
    }

    let options: Vec<&Arg> = visible().filter(|arg| !arg.is_positional()).collect();
    if !options.is_empty() {
        out.push_str("\nOptions:\n\n");
        for arg in options {
            out.push_str(&render_arg(arg));
        }
    }

    // Only the root declares one, and it carries the environment variables and
    // the exit codes — the part of the contract that is not a flag anywhere.
    if let Some(after) = command.get_after_long_help() {
        out.push_str(&format!(
            "\n```text\n{}\n```\n",
            after.to_string().trim_end()
        ));
    }

    if generated_elsewhere {
        return;
    }

    for sub in subcommands {
        render(out, sub, &format!("{path} {}", sub.get_name()));
    }
}

/// One list item for one argument.
fn render_arg(arg: &Arg) -> String {
    let value = arg
        .get_value_names()
        .and_then(|names| names.first())
        .map(ToString::to_string)
        .unwrap_or_else(|| arg.get_id().to_string().to_uppercase());

    let name = if arg.is_positional() {
        if arg.is_required_set() {
            format!("<{value}>")
        } else {
            format!("[{value}]")
        }
    } else {
        let takes = arg.get_action().takes_values();
        let flag = match (arg.get_short(), arg.get_long()) {
            (Some(short), Some(long)) => format!("-{short}, --{long}"),
            (Some(short), None) => format!("-{short}"),
            (None, Some(long)) => format!("--{long}"),
            // A named argument with neither spelling cannot be typed at all.
            (None, None) => arg.get_id().to_string(),
        };
        if takes {
            format!("{flag} <{value}>")
        } else {
            flag
        }
    };

    let help = arg.get_help().map(one_line).unwrap_or_default();

    let mut item = format!("- `{name}`");
    if !help.is_empty() {
        item.push_str(&format!(" — {help}"));
    }
    for note in notes(arg, &help) {
        item.push_str(&format!(" ({note})"));
    }
    item.push('\n');
    item
}

/// The parts of an argument's contract that its help text does not say.
fn notes(arg: &Arg, help: &str) -> Vec<String> {
    let mut notes = Vec::new();

    // A switch's possible values are `true` and `false`, which is what a switch
    // is; only an argument that takes a value has a set worth naming.
    let values: Vec<String> = if arg.get_action().takes_values() {
        arg.get_possible_values()
            .iter()
            .filter(|value| !value.is_hide_set())
            .map(|value| value.get_name().to_string())
            .collect()
    } else {
        Vec::new()
    };
    if !values.is_empty() {
        notes.push(format!("values: {}", values.join(", ")));
    }

    let defaults: Vec<String> = arg
        .get_default_values()
        .iter()
        .map(|value| value.to_string_lossy().into_owned())
        .collect();
    if !defaults.is_empty() {
        notes.push(format!("default: {}", defaults.join(", ")));
    }

    // The variable name, never a value: `hide_env_values` exists because the
    // value can be a secret, and this file is checked in.
    if let Some(env) = arg.get_env() {
        notes.push(format!("env: {}", env.to_string_lossy()));
    }

    // Several help texts already end in "Repeatable". Saying it again in the
    // same list item reads as a generator rather than a reference.
    if matches!(arg.get_action(), ArgAction::Append) && !help.contains("epeatable") {
        notes.push("repeatable".to_string());
    }

    notes
}

/// Help text as a single line.
///
/// A doc comment wrapped across source lines keeps those newlines, and a
/// newline inside a Markdown list item ends the item.
fn one_line(text: &clap::builder::StyledStr) -> String {
    text.to_string()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// How a command is written when it is documented, as opposed to mentioned.
    fn heading(path: &str) -> String {
        format!("\n## `{path}`\n")
    }

    /// How an argument is written when it is documented.
    fn item(flag: &str) -> String {
        format!("\n- `{flag}")
    }

    #[test]
    fn test_a_hidden_command_is_not_documented() {
        let document = document();

        // The preamble names `xrpl wallet` to say it is gone, which is not the
        // same as giving it a section — so this looks for the section.
        assert!(!document.contains(&heading("xrpl wallet")));
        assert!(!document.contains(&heading("xrpl wallet generate")));
    }

    #[test]
    fn test_a_hidden_flag_is_not_documented() {
        let document = document();

        // `--seed` puts a secret in `argv`. A reference that lists it teaches
        // the thing the deprecation exists to stop teaching.
        assert!(!document.contains(&item("--seed`")));
        assert!(!document.contains(&item("--seed <")));
        assert!(!document.contains(&item("--xrpl-seed")));
        // `--seed-file` is the replacement and must still be there, or this
        // test would pass on an empty document.
        assert!(document.contains(&item("--seed-file <PATH>")));
    }

    #[test]
    fn test_the_key_material_disclosure_is_in_the_generated_output() {
        // The claim belongs wherever someone reads about signing, not only in
        // the README that a generated reference tends to replace.
        assert!(
            document().contains("encrypted at rest, plaintext in process memory at signing\ntime")
        );
    }

    #[test]
    fn test_the_transaction_types_are_left_to_the_other_generated_file() {
        let document = document();

        assert!(document.contains(&heading("xrpl tx new")));
        assert!(document.contains("TX_SURFACE.md"));
        // One type standing in for the seventy: if this appears, the two
        // generated files have started covering the same ground.
        assert!(!document.contains(&heading("xrpl tx new payment")));
    }
}
