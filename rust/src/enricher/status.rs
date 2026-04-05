use crate::types::PortStatus;

const DEV_PROCESS_NAMES: &[&str] = &[
    "node", "node.exe", "deno", "bun", "python", "python3", "python3.exe",
    "ruby", "go", "cargo", "rustc", "java", "php", "dotnet", "tsx",
    "webpack", "vite", "esbuild", "parcel",
];

const DEV_COMMAND_PATTERNS: &[&str] = &[
    "webpack", "vite", "next", "nuxt", "react-scripts", "gatsby",
    "nest", "fastify", "express", "rails", "django", "flask",
    "gunicorn", "uvicorn", "hypercorn", "phoenix", "mix phx",
    "cargo run", "go run", "air ",
];

pub fn is_dev_process(name: &str, command: &str) -> bool {
    let name_lower = name.to_lowercase();
    let cmd_lower = command.to_lowercase();
    DEV_PROCESS_NAMES.iter().any(|&n| name_lower == n || name_lower.starts_with(n))
        || DEV_COMMAND_PATTERNS.iter().any(|&p| cmd_lower.contains(p))
}

pub fn detect_status(_name: &str, _command: &str, _pid: u32) -> PortStatus {
    PortStatus::Healthy
}
