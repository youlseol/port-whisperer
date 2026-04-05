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
    let normalized_name = name_lower.strip_suffix(".exe").unwrap_or(&name_lower);
    let cmd_lower = command.to_lowercase();

    DEV_PROCESS_NAMES.iter().any(|&n| {
        let normalized_pattern = n.strip_suffix(".exe").unwrap_or(n);
        normalized_name == normalized_pattern
    })
        || DEV_COMMAND_PATTERNS.iter().any(|&p| cmd_lower.contains(p))
}
