//! Command-line entry point for logloom.

mod cli;
mod diagnostics;

use std::io::{self, Write};
use std::process::ExitCode;

fn main() -> ExitCode {
    match cli::run() {
        Ok(code) => code,
        Err(error) => {
            let _ = writeln!(io::stderr().lock(), "{error}");
            error.code
        }
    }
}
