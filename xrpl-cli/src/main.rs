use std::process::ExitCode;

use clap::Parser;

use xrpl_cli::commands::Cli;
use xrpl_cli::error::exit;

fn main() -> ExitCode {
    // Not `Cli::parse()`, which prints and exits with clap's own code 2 for a
    // usage error. This binary documents usage as exit 1, and a CLI whose exit
    // codes disagree with its own documentation is worse than one that
    // documents none.
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(error) => {
            let _ = error.print();
            // `--help` and `--version` arrive here too, and they are successes.
            return if error.use_stderr() {
                ExitCode::from(exit::USAGE)
            } else {
                ExitCode::SUCCESS
            };
        }
    };

    match cli.run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            // Errors and diagnostics go to stderr; stdout carries the machine
            // artifact and nothing else.
            eprintln!("error: {error}");
            // Not `ExitCode::FAILURE`. A bad flag, an unreachable node and a
            // validated ledger failure are different things to a caller, and a
            // retry wrapper has to tell them apart. See `Error::exit_code`.
            error.exit_code()
        }
    }
}
