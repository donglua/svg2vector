mod cli;
mod cli_error;
mod cli_output;

use clap::Parser;
use std::process::ExitCode;

fn main() -> ExitCode {
    match cli::Cli::parse().run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("svg2vector: {error}");
            ExitCode::FAILURE
        }
    }
}
