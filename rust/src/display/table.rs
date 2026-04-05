use crate::types::{PortEntry, PortStatus, ProcessEntry};
use colored::Colorize;
use comfy_table::{Attribute, Cell, Color, ContentArrangement, Table};
use std::time::{SystemTime, UNIX_EPOCH};

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
            Cell::new("PROCESS").add_attribute(Attribute::Bold),
            Cell::new("PID").add_attribute(Attribute::Bold),
            Cell::new("PROJECT").add_attribute(Attribute::Bold),
            Cell::new("FRAMEWORK").add_attribute(Attribute::Bold),
            Cell::new("UPTIME").add_attribute(Attribute::Bold),
            Cell::new("MEMORY").add_attribute(Attribute::Bold),
            Cell::new("STATUS").add_attribute(Attribute::Bold),
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

        table.add_row(vec![
            port_cell,
            Cell::new(process_label),
            Cell::new(entry.pid.to_string()).fg(Color::DarkGrey),
            optional_cell(entry.project_name.as_deref(), Color::Blue),
            framework_cell,
            optional_cell(entry.start_time.map(format_uptime_from_epoch).as_deref(), Color::Yellow),
            Cell::new(format_memory(entry.memory_kb)).fg(Color::DarkGrey),
            status_cell(&entry.status),
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

pub fn print_process_table(processes: &[ProcessEntry], show_all_hint: bool) {
    if processes.is_empty() {
        println!("{}", "  No dev processes found.".dimmed());
        if show_all_hint {
            println!("{}", "  Run with --all to show everything".dimmed());
        }
        println!();
        return;
    }

    let mut table = Table::new();
    table
        .set_content_arrangement(ContentArrangement::Dynamic)
        .set_header(vec![
            Cell::new("PID").add_attribute(Attribute::Bold),
            Cell::new("PROCESS").add_attribute(Attribute::Bold),
            Cell::new("CPU%").add_attribute(Attribute::Bold),
            Cell::new("MEM").add_attribute(Attribute::Bold),
            Cell::new("PROJECT").add_attribute(Attribute::Bold),
            Cell::new("FRAMEWORK").add_attribute(Attribute::Bold),
            Cell::new("UPTIME").add_attribute(Attribute::Bold),
            Cell::new("STATUS").add_attribute(Attribute::Bold),
            Cell::new("WHAT").add_attribute(Attribute::Bold),
        ]);

    for entry in processes {
        table.add_row(vec![
            Cell::new(entry.pid.to_string()).fg(Color::DarkGrey),
            Cell::new(&entry.process_name),
            cpu_cell(entry.cpu_pct),
            Cell::new(format_memory(entry.memory_kb)).fg(Color::DarkGrey),
            optional_cell(entry.project_name.as_deref(), Color::Blue),
            optional_cell(entry.framework.as_deref(), Color::Green),
            optional_cell(entry.start_time.map(format_uptime_from_epoch).as_deref(), Color::Yellow),
            status_cell(&entry.status),
            Cell::new(&entry.description).fg(Color::DarkGrey),
        ]);
    }

    println!("{table}");
    let count = processes.len();
    println!(
        "{}",
        format!("  {} process{} active", count, if count == 1 { "" } else { "es" }).dimmed()
    );
    if show_all_hint {
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

fn format_uptime_from_epoch(start_time: u64) -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(start_time);
    let seconds = now.saturating_sub(start_time);
    let minutes = seconds / 60;
    let hours = minutes / 60;
    let days = hours / 24;

    if days > 0 {
        format!("{}d {}h", days, hours % 24)
    } else if hours > 0 {
        format!("{}h {}m", hours, minutes % 60)
    } else if minutes > 0 {
        format!("{}m {}s", minutes, seconds % 60)
    } else {
        format!("{}s", seconds)
    }
}

fn optional_cell(value: Option<&str>, color: Color) -> Cell {
    match value {
        Some(value) if !value.is_empty() => Cell::new(value).fg(color),
        _ => Cell::new("—").fg(Color::DarkGrey),
    }
}

fn status_cell(status: &PortStatus) -> Cell {
    match status {
        PortStatus::Healthy => Cell::new("healthy").fg(Color::Green),
        PortStatus::Zombie => Cell::new("zombie").fg(Color::Red),
        PortStatus::Orphaned => Cell::new("orphaned").fg(Color::Yellow),
    }
}

fn cpu_cell(cpu_pct: f32) -> Cell {
    let formatted = format!("{:.1}", cpu_pct);
    let color = if cpu_pct > 25.0 {
        Color::Red
    } else if cpu_pct > 5.0 {
        Color::Yellow
    } else {
        Color::Green
    };
    Cell::new(formatted).fg(color)
}
