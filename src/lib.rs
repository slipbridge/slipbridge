//! Slipbridge: an agent-first CLI for thermal receipt printers.

mod cli;
mod core;
mod diagnostics;
mod error;
mod escpos;
mod input;
mod output;
mod profiles;
mod render;
mod transport;

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    use clap::Parser;

    let cli = cli::Cli::parse();
    core::execute(cli).map_err(|err| Box::new(err) as Box<dyn std::error::Error>)
}
