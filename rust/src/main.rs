mod cli;
mod collector;
mod display;
mod enricher;
mod platform;
mod types;

use anyhow::Result;
use clap::Parser;
use cli::{Cli, Commands};

fn main() -> Result<()> {
    let cli = Cli::parse();

    // `ports <number>` — positional port argument takes priority
    if let Some(port_num) = cli.port_number {
        // Wave 3: detail view + interactive kill
        println!("Port detail for :{port_num} — coming in Wave 3");
        return Ok(());
    }

    match cli.command {
        None => {
            let entries = collector::collect_ports(cli.all)?;
            display::table::print_port_table(&entries, !cli.all);
        }
        Some(Commands::Ps { all: _ }) => {
            println!("ports ps — coming in Wave 3");
        }
        Some(Commands::Clean) => {
            println!("ports clean — coming in Wave 3");
        }
        Some(Commands::Watch { interval_ms }) => {
            display::watch::start_watch(interval_ms)?;
        }
    }

    Ok(())
}
