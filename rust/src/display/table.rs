use crate::types::{PortEntry, PortStatus};
use colored::Colorize;
use comfy_table::{Attribute, Cell, Color, ContentArrangement, Table};

pub fn print_port_table(ports: &[PortEntry], dev_only: bool) {
    if ports.is_empty() {
        println!("{}", "  No listening ports found.".dimmed());
        return;
    }

    let mut table = Table::new();
    table
        .set_content_arrangement(ContentArrangement::Dynamic)
        .set_header(vec![
            Cell::new("PORT").add_attribute(Attribute::Bold),
            Cell::new("PID").add_attribute(Attribute::Bold),
            Cell::new("PROCESS").add_attribute(Attribute::Bold),
            Cell::new("FRAMEWORK").add_attribute(Attribute::Bold),
            Cell::new("MEMORY").add_attribute(Attribute::Bold),
            Cell::new("GIT").add_attribute(Attribute::Bold),
        ]);

    for entry in ports {
        let port_str = format!(":{}", entry.port);
        let port_cell = match entry.status {
            PortStatus::Healthy => Cell::new(&port_str).fg(Color::Cyan).add_attribute(Attribute::Bold),
            PortStatus::Zombie => Cell::new(&port_str).fg(Color::Red).add_attribute(Attribute::Bold),
            PortStatus::Orphaned => Cell::new(&port_str).fg(Color::Yellow).add_attribute(Attribute::Bold),
        };

        let process_label = if let Some(docker) = &entry.docker {
            format!("🐳 {}", docker.name)
        } else {
            entry.process_name.clone()
        };

        let framework_cell = match entry.framework.as_deref() {
            Some(fw) => Cell::new(fw).fg(Color::Green),
            None => Cell::new("—").fg(Color::DarkGrey),
        };

        let git_cell = match entry.git_branch.as_deref() {
            Some(b) => Cell::new(b).fg(Color::Yellow),
            None => Cell::new("—").fg(Color::DarkGrey),
        };

        table.add_row(vec![
            port_cell,
            Cell::new(entry.pid.to_string()).fg(Color::DarkGrey),
            Cell::new(process_label),
            framework_cell,
            Cell::new(format_memory(entry.memory_kb)).fg(Color::DarkGrey),
            git_cell,
        ]);
    }

    println!("{table}");

    let count = ports.len();
    let summary = format!("  {} port{} active", count, if count == 1 { "" } else { "s" });
    println!("{}", summary.dimmed());
    if dev_only {
        println!("{}", "  Run with --all to show everything".dimmed());
    }
    println!();
}

fn format_memory(kb: u64) -> String {
    if kb >= 1_048_576 {
        format!("{:.1} GB", kb as f64 / 1_048_576.0)
    } else if kb >= 1024 {
        format!("{:.0} MB", kb as f64 / 1024.0)
    } else {
        format!("{} KB", kb)
    }
}
