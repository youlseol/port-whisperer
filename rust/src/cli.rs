use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = "ports", about = "Listen to your ports 🔊", version)]
pub struct Cli {
    /// Force plain text output instead of the full-screen TUI
    #[arg(long, global = true)]
    pub plain: bool,

    /// Show all ports, not just dev servers
    #[arg(short, long, global = true)]
    pub all: bool,

    /// Refresh interval in milliseconds
    #[arg(long, global = true, default_value = "2000")]
    pub interval_ms: u64,

    /// Inspect a specific port number
    #[arg(value_name = "PORT")]
    pub port_number: Option<u16>,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Show all running dev processes (not just port-bound)
    Ps {
        #[arg(short, long, help = "Show all processes, not just dev")]
        all: bool,
    },
    /// Kill orphaned or zombie dev server processes
    Clean,
    /// Monitor port changes in real-time
    Watch,
}
