use std::process::ExitCode;

use clap::Parser;

use xrpl_cli::commands::Cli;

fn main() -> ExitCode {
    let cli = Cli::parse();

    match cli.run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}
