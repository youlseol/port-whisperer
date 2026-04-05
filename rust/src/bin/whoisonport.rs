use anyhow::Result;
use clap::Parser;
use port_whisperer::cli::Cli;

fn main() -> Result<()> {
    port_whisperer::run(Cli::parse())
}
