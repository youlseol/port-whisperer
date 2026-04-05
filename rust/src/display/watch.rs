use crate::collector;
use colored::Colorize;
use std::collections::HashMap;
use std::thread::sleep;
use std::time::Duration;

pub fn start_watch(interval_ms: u64) -> anyhow::Result<()> {
    println!("{}", "  Watching for port changes...".cyan().bold());
    println!("{}", "  Press Ctrl+C to stop".dimmed());
    println!();

    let mut previous = HashMap::new();
    loop {
        let current = collector::collect_ports(false)?
            .into_iter()
            .map(|entry| (entry.port, entry))
            .collect::<HashMap<_, _>>();

        for (port, entry) in &current {
            if !previous.contains_key(port) {
                println!(
                    "  {} :{} ← {}",
                    "▲ NEW".green(),
                    port.to_string().bold(),
                    entry.process_name
                );
            }
        }

        for port in previous.keys() {
            if !current.contains_key(port) {
                println!("  {} :{}", "▼ CLOSED".red(), port.to_string().bold());
            }
        }

        previous = current;
        sleep(Duration::from_millis(interval_ms));
    }
}
