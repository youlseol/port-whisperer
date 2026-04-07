use crate::cli::{Cli, Commands};
use crate::display::tui;
use anyhow::Result;
use dialoguer::Confirm;
use std::io::IsTerminal;

pub fn run(cli: Cli) -> Result<()> {
    if should_use_tui(&cli) {
        match tui::run(&cli) {
            Ok(()) => return Ok(()),
            Err(error) => {
                eprintln!("TUI unavailable (tip: try a modern terminal like Windows Terminal), falling back to plain mode: {error}");
            }
        }
    }

    run_plain(cli)
}

fn should_use_tui(cli: &Cli) -> bool {
    !cli.plain && std::io::stdin().is_terminal() && std::io::stdout().is_terminal()
}

fn run_plain(cli: Cli) -> Result<()> {
    if std::io::stdout().is_terminal() {
        crate::display::banner::print_plain();
    }

    if let Some(port_num) = cli.port_number {
        if let Some((entry, tree)) = crate::collector::get_port_detail(port_num)? {
            crate::display::detail::print_port_detail(&entry, &tree);
            if std::io::stdin().is_terminal()
                && Confirm::new()
                    .with_prompt(format!("Kill process on :{}?", port_num))
                    .default(false)
                    .interact()?
            {
                crate::platform::kill_process(entry.pid)?;
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
            let entries = crate::collector::collect_ports(cli.all)?;
            crate::display::table::print_port_table(&entries, !cli.all);
        }
        Some(Commands::Ps { all }) => {
            let processes = crate::collector::collect_processes(all);
            crate::display::table::print_process_table(&processes, !all);
        }
        Some(Commands::Clean) => {
            let entries = crate::collector::collect_ports(false)?;
            let cleanup_targets: Vec<_> = entries
                .into_iter()
                .filter(|entry| {
                    matches!(
                        entry.status,
                        crate::types::PortStatus::Orphaned | crate::types::PortStatus::Zombie
                    )
                })
                .collect();

            if cleanup_targets.is_empty() {
                crate::display::detail::print_clean_results(&[]);
                return Ok(());
            }

            if !std::io::stdin().is_terminal()
                || !Confirm::new()
                    .with_prompt(format!(
                        "Kill {} orphaned/zombie process(es)?",
                        cleanup_targets.len()
                    ))
                    .default(false)
                    .interact()?
            {
                println!("  Aborted.");
                println!();
                return Ok(());
            }

            let results = cleanup_targets
                .into_iter()
                .map(|entry| match crate::platform::kill_process(entry.pid) {
                    Ok(()) => crate::types::CleanResult {
                        entry,
                        killed: true,
                        error: None,
                    },
                    Err(error) => crate::types::CleanResult {
                        entry,
                        killed: false,
                        error: Some(error.to_string()),
                    },
                })
                .collect::<Vec<_>>();
            crate::display::detail::print_clean_results(&results);
        }
        Some(Commands::Watch) => {
            crate::display::watch::start_watch(cli.interval_ms)?;
        }
    }

    Ok(())
}
