use crate::types::{CleanResult, PortEntry, ProcessTreeNode, PortStatus};
use colored::{ColoredString, Colorize};
use std::time::{SystemTime, UNIX_EPOCH};

pub fn print_port_detail(entry: &PortEntry, tree: &[ProcessTreeNode]) {
    let status = status_label(&entry.status).to_string();
    println!("  {}", format!("Port :{}", entry.port).bold());
    println!("  {}", "──────────────────────".dimmed());
    println!();
    print_field("Process", &entry.process_name);
    print_field("PID", &entry.pid.to_string());
    print_field("Status", &status);
    print_field("Framework", entry.framework.as_deref().unwrap_or("—"));
    print_field("Memory", &format_memory(entry.memory_kb));
    print_field(
        "Uptime",
        entry.start_time
            .map(format_uptime_from_epoch)
            .unwrap_or_else(|| "—".into())
            .as_str(),
    );
    print_field(
        "Started",
        entry.start_time
            .map(format_started_at)
            .unwrap_or_else(|| "—".into())
            .as_str(),
    );
    println!();
    println!("  {}", "Location".bold().cyan());
    println!("  {}", "──────────────────────".dimmed());
    print_field(
        "Directory",
        entry
            .cwd
            .as_ref()
            .map(|path| path.to_string_lossy().into_owned())
            .unwrap_or_else(|| "—".into())
            .as_str(),
    );
    print_field("Project", entry.project_name.as_deref().unwrap_or("—"));
    print_field("Git Branch", entry.git_branch.as_deref().unwrap_or("—"));
    println!();
    println!("  {}", "Process Tree".bold().cyan());
    println!("  {}", "──────────────────────".dimmed());
    if tree.is_empty() {
        println!("  {}", "—".dimmed());
    } else {
        for (index, node) in tree.iter().enumerate() {
            let prefix = if index == 0 { "→" } else { "└─" };
            println!(
                "  {} {} {}",
                prefix.dimmed(),
                node.name.as_str().bold(),
                format!("({})", node.pid).dimmed()
            );
        }
    }
    println!();
    println!(
        "  {}",
        format!("Kill this process with: {}", kill_hint(entry.pid)).dimmed()
    );
    println!();
}

pub fn print_clean_results(results: &[CleanResult]) {
    if results.is_empty() {
        println!("{}", "  ✓ No orphaned or zombie processes found.".green());
        println!();
        return;
    }

    println!(
        "{}",
        format!(
            "  Found {} orphaned/zombie process{}:",
            results.len(),
            if results.len() == 1 { "" } else { "es" }
        )
        .yellow()
        .bold()
    );
    println!();

    let killed = results.iter().filter(|result| result.killed).count();
    let failed = results.iter().filter(|result| !result.killed).count();

    for result in results {
        let icon = if result.killed {
            "✓".green()
        } else {
            "✕".red()
        };
        println!(
            "  {} :{} — {} {}",
            icon,
            result.entry.port.to_string().bold(),
            result.entry.process_name,
            format!("(PID {})", result.entry.pid).dimmed()
        );
        if let Some(error) = &result.error {
            println!("    {}", error.red());
        }
    }

    println!();
    if killed > 0 {
        println!(
            "{}",
            format!("  Cleaned {} process{}.", killed, if killed == 1 { "" } else { "es" })
                .green()
        );
    }
    if failed > 0 {
        println!(
            "{}",
            format!(
                "  Failed to clean {} process{}.",
                failed,
                if failed == 1 { "" } else { "es" }
            )
            .red()
        );
    }
    println!();
}

fn print_field(label: &str, value: &str) {
    println!("  {:<12} {}", label.dimmed(), value);
}

fn format_memory(kb: u64) -> String {
    if kb >= 1_048_576 {
        format!("{:.1} GB", kb as f64 / 1_048_576.0)
    } else if kb >= 1024 {
        format!("{:.1} MB", kb as f64 / 1024.0)
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

fn format_started_at(start_time: u64) -> String {
    format!("unix:{}", start_time)
}

fn status_label(status: &PortStatus) -> ColoredString {
    match status {
        PortStatus::Healthy => "healthy".green(),
        PortStatus::Zombie => "zombie".red(),
        PortStatus::Orphaned => "orphaned".yellow(),
    }
}

fn kill_hint(pid: u32) -> String {
    if cfg!(windows) {
        format!("taskkill /F /PID {pid}")
    } else {
        format!("kill -TERM {pid}")
    }
}
