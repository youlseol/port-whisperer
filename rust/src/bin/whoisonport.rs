#[path = "../cli.rs"]
mod cli;
#[path = "../collector/mod.rs"]
mod collector;
#[path = "../display/mod.rs"]
mod display;
#[path = "../enricher/mod.rs"]
mod enricher;
#[path = "../platform/mod.rs"]
mod platform;
#[path = "../types.rs"]
mod types;

use anyhow::Result;
use clap::Parser;
use dialoguer::Confirm;
use std::io::IsTerminal;

fn main() -> Result<()> {
    let cli = cli::Cli::parse();

    if let Some(port_num) = cli.port_number {
        if let Some((entry, tree)) = collector::get_port_detail(port_num)? {
            display::detail::print_port_detail(&entry, &tree);
            if std::io::stdin().is_terminal()
                && Confirm::new()
                    .with_prompt(format!("Kill process on :{}?", port_num))
                    .default(false)
                    .interact()?
            {
                platform::kill_process(entry.pid)?;
                println!("  ✓ Killed PID {}", entry.pid);
                println!();
            }
        } else {
            println!("  No process found on that port.");
            println!();
        }
        return Ok(());
    }

    match cli.command {
        None => {
            let entries = collector::collect_ports(cli.all)?;
            display::table::print_port_table(&entries, !cli.all);
        }
        Some(cli::Commands::Ps { all }) => {
            let processes = collector::collect_processes(all);
            display::table::print_process_table(&processes, !all);
        }
        Some(cli::Commands::Clean) => {
            let entries = collector::collect_ports(false)?;
            let cleanup_targets: Vec<_> = entries
                .into_iter()
                .filter(|entry| {
                    matches!(
                        entry.status,
                        types::PortStatus::Orphaned | types::PortStatus::Zombie
                    )
                })
                .collect();

            if cleanup_targets.is_empty() {
                display::detail::print_clean_results(&[]);
                return Ok(());
            }

            if !std::io::stdin().is_terminal()
                || !Confirm::new()
                    .with_prompt(format!("Kill {} orphaned/zombie process(es)?", cleanup_targets.len()))
                    .default(false)
                    .interact()?
            {
                println!("  Aborted.");
                println!();
                return Ok(());
            }

            let results = cleanup_targets
                .into_iter()
                .map(|entry| match platform::kill_process(entry.pid) {
                    Ok(()) => types::CleanResult {
                        entry,
                        killed: true,
                        error: None,
                    },
                    Err(error) => types::CleanResult {
                        entry,
                        killed: false,
                        error: Some(error.to_string()),
                    },
                })
                .collect::<Vec<_>>();
            display::detail::print_clean_results(&results);
        }
        Some(cli::Commands::Watch { interval_ms }) => {
            display::watch::start_watch(interval_ms)?;
        }
    }

    Ok(())
}
