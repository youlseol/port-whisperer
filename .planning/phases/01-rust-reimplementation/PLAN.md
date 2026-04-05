# Phase 1: Rust Reimplementation — Plan

**Goal:** Deliver a single Rust binary (`ports`) that fully replaces the Node.js port-whisperer: native TCP socket enumeration, framework detection, process info, interactive kill, `ps`/`clean`/`watch` subcommands, on macOS and Windows — no Node.js runtime required.

**Waves:** 5 waves

---

## Wave 1: Project Setup + Core Port Listing

**Deliverable:** `cargo run` prints a colored table of listening ports with PID, process name, and memory. The two-phase batch architecture (netstat2 → sysinfo → rayon) is in place.

---

### Task 1.1: Scaffold Cargo Project + Full Dependency Manifest

**Objective:** Create `Cargo.toml` with all dependencies pinned to research-validated versions, and the complete `src/` directory skeleton with stub modules.

**Files to create/modify:**
- `Cargo.toml` (new — at repo root alongside `package.json`)
- `src/main.rs`
- `src/cli.rs`
- `src/collector/mod.rs`, `src/collector/ports.rs`, `src/collector/processes.rs`, `src/collector/docker.rs`, `src/collector/git.rs`
- `src/enricher/mod.rs`, `src/enricher/framework.rs`, `src/enricher/status.rs`
- `src/display/mod.rs`, `src/display/table.rs`, `src/display/detail.rs`, `src/display/watch.rs`
- `src/platform/mod.rs`, `src/platform/unix.rs`, `src/platform/windows.rs`
- `src/types.rs` (shared data model)

**Implementation:**

`Cargo.toml`:
```toml
[package]
name = "port-whisperer"
version = "0.1.0"
edition = "2021"
default-run = "ports"

[[bin]]
name = "ports"
path = "src/main.rs"

[features]
watch = ["tokio", "ratatui", "crossterm"]

[dependencies]
netstat2 = "0.11"
sysinfo = "0.38"
clap = { version = "4.6", features = ["derive"] }
rayon = "1.11"
anyhow = "1.0"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
colored = "3.1"
comfy-table = "7.2"
dialoguer = "0.12"
indicatif = "0.18"
regex = "1.11"

tokio = { version = "1.51", features = ["rt", "rt-multi-thread", "time", "macros"], optional = true }
ratatui = { version = "0.30", optional = true }
crossterm = { version = "0.29", optional = true }

[target.'cfg(unix)'.dependencies]
nix = { version = "0.31", features = ["signal", "process"] }
```

`src/types.rs` — the shared data model (define first, everything else imports from here):
```rust
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct PortEntry {
    pub port: u16,
    pub pid: u32,
    pub process_name: String,
    pub command: String,
    pub cwd: Option<PathBuf>,
    pub framework: Option<String>,
    pub git_branch: Option<String>,
    pub memory_kb: u64,
    pub cpu_pct: f32,
    pub start_time: Option<u64>,  // Unix epoch seconds
    pub status: PortStatus,
    pub docker: Option<DockerInfo>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PortStatus { Healthy, Zombie, Orphaned }

#[derive(Debug, Clone)]
pub struct DockerInfo {
    pub name: String,
    pub image: String,
}

#[derive(Debug, Clone)]
pub struct ProcessEntry {
    pub pid: u32,
    pub process_name: String,
    pub command: String,
    pub cwd: Option<PathBuf>,
    pub memory_kb: u64,
    pub cpu_pct: f32,
    pub start_time: Option<u64>,
    pub status: PortStatus,
}
```

All other `src/*/mod.rs` and `src/*.rs` files: create as stubs with `pub mod` declarations and `todo!()` function bodies so the project compiles.

`src/main.rs` stub:
```rust
mod cli;
mod collector;
mod display;
mod enricher;
mod platform;
mod types;

fn main() -> anyhow::Result<()> {
    println!("port-whisperer (Rust) — under construction");
    Ok(())
}
```

**Key APIs:**
- This task produces no runtime logic — it defines the type contracts everything else builds against.

**Pitfalls:**
- Define `PortEntry` in `src/types.rs` and import everywhere (`use crate::types::PortEntry`). Do NOT define the struct inline in `collector/mod.rs` — later tasks all depend on a single canonical type.
- Add `#[allow(dead_code)]` to stub modules to suppress warnings during incremental implementation.

**Verification:**
```bash
cd /path/to/port-whisperer && cargo build 2>&1 | tail -5
```
Expected: compiles with zero errors (warnings about dead code are fine).

---

### Task 1.2: Clap CLI Structure

**Objective:** Wire up `clap` derive-based CLI matching the current `ports` interface: default command (list ports), `ports <number>` (detail), `ports ps`, `ports clean`, `ports watch`.

**Files to create/modify:**
- `src/cli.rs` (full implementation)
- `src/main.rs` (dispatch logic)

**Implementation:**

`src/cli.rs`:
```rust
use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = "ports", about = "Listen to your ports", version)]
pub struct Cli {
    /// Show all ports, not just dev servers
    #[arg(short, long)]
    pub all: bool,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Show running dev processes (like `ports` but process-centric)
    Ps {
        #[arg(short, long, help = "Show all processes")]
        all: bool,
    },
    /// Kill orphaned/zombie dev servers
    Clean,
    /// Monitor port changes in real-time
    #[cfg(feature = "watch")]
    Watch {
        /// Refresh interval in milliseconds
        #[arg(long, default_value = "2000")]
        interval_ms: u64,
    },
    /// Show details for a specific port number
    Port {
        number: u16,
    },
}
```

`src/main.rs` dispatch:
```rust
use clap::Parser;
mod cli; mod collector; mod display; mod enricher; mod platform; mod types;
use cli::{Cli, Commands};

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        None => {
            // Default: list listening ports (Wave 1)
            let entries = collector::collect_ports(cli.all)?;
            display::table::print_port_table(&entries, !cli.all);
        }
        Some(Commands::Ps { all }) => todo!("ps command — Wave 3"),
        Some(Commands::Clean) => todo!("clean command — Wave 3"),
        #[cfg(feature = "watch")]
        Some(Commands::Watch { interval_ms }) => todo!("watch command — Wave 4"),
        Some(Commands::Port { number }) => todo!("detail command — Wave 3"),
    }
    Ok(())
}
```

**Key APIs:** `clap 4.6` derive — `#[derive(Parser)]`, `#[command(...)]`, `#[arg(...)]`, `#[command(subcommand)]`.

**Pitfalls:**
- `ports <number>` in the original is a positional argument, not a subcommand. The Rust version converts it to `ports port <number>` (explicit subcommand) OR keep positional by adding `#[arg(value_name = "PORT")]` to `Cli`. Recommended: keep a `port_number: Option<u16>` field on `Cli` for backward compatibility alongside subcommands.
- Gate `Watch` behind `#[cfg(feature = "watch")]` so the binary compiles without the optional tokio/ratatui deps by default.

**Verification:**
```bash
cargo run -- --help
cargo run -- ps --help
cargo run -- port --help
```
Expected: help text shows all subcommands; `cargo run` (no args) prints "under construction" placeholder without error.

---

### Task 1.3: Core Port Collection + Basic Table Output

**Objective:** Implement the two-phase batch collection pipeline (netstat2 → sysinfo) and the comfy-table display. Running `cargo run` should print a colored table of listening ports.

**Files to create/modify:**
- `src/collector/ports.rs` (netstat2 socket enumeration)
- `src/collector/processes.rs` (sysinfo batch refresh)
- `src/collector/mod.rs` (orchestrator)
- `src/display/table.rs` (comfy-table output)
- `src/main.rs` (wire `collect_ports` call)

**Implementation:**

`src/collector/ports.rs`:
```rust
use netstat2::{get_sockets_info, AddressFamilyFlags, ProtocolFlags, ProtocolSocketInfo, TcpState};
use anyhow::Result;

/// Returns (port, pid) pairs for all TCP LISTEN sockets.
/// Takes .first() PID only — see Pitfall 2 in research.
pub fn get_listening_sockets() -> Result<Vec<(u16, u32)>> {
    let af = AddressFamilyFlags::IPV4 | AddressFamilyFlags::IPV6;
    let sockets = get_sockets_info(af, ProtocolFlags::TCP)?;

    let mut pairs: Vec<(u16, u32)> = sockets
        .into_iter()
        .filter(|s| matches!(
            &s.protocol_socket_info,
            ProtocolSocketInfo::Tcp(tcp) if tcp.state == TcpState::Listen
        ))
        .filter_map(|s| {
            let pid = *s.associated_pids.first()?; // Pitfall 2: Vec<u32>, take first
            let port = match &s.protocol_socket_info {
                ProtocolSocketInfo::Tcp(tcp) => tcp.local_port,
                _ => return None,
            };
            Some((port, pid))
        })
        .collect();

    // Deduplicate by port (same port may appear for IPv4 + IPv6 dual-stack)
    pairs.sort_by_key(|(port, _)| *port);
    pairs.dedup_by_key(|(port, _)| *port);
    Ok(pairs)
}
```

`src/collector/processes.rs`:
```rust
use sysinfo::{
    Pid, ProcessesToUpdate, ProcessRefreshKind, System, UpdateKind,
};

/// Create a System, batch-refresh only the requested PIDs.
/// CRITICAL: call this ONCE for all PIDs, not once per PID.
/// Returns the System (caller reads via &System — read-only after this).
pub fn batch_refresh_processes(pids: &[u32]) -> System {
    let mut sys = System::new();
    let sysinfo_pids: Vec<Pid> = pids.iter().map(|&p| Pid::from(p as usize)).collect();

    // Pitfall 3: Use specifics to avoid slow CPU-differential refresh
    sys.refresh_processes_specifics(
        ProcessesToUpdate::Some(&sysinfo_pids),
        true,
        ProcessRefreshKind::new()
            .with_memory()
            .with_exe(UpdateKind::Always)
            .with_cwd(UpdateKind::Always),
        // Note: omit .with_cpu() for first call — CPU% requires two samples.
        // Add a second refresh pass later if cpu_pct is needed.
    );
    sys
}
```

`src/collector/mod.rs` (orchestrator):
```rust
pub mod ports;
pub mod processes;
pub mod docker;
pub mod git;

use crate::types::{PortEntry, PortStatus};
use crate::enricher;
use anyhow::Result;
use rayon::prelude::*;

pub fn collect_ports(show_all: bool) -> Result<Vec<PortEntry>> {
    // Phase 1a: socket list
    let raw = ports::get_listening_sockets()?;
    if raw.is_empty() { return Ok(vec![]); }

    // Phase 1b: batch process refresh — ONE call for all PIDs
    let pids: Vec<u32> = raw.iter().map(|(_, pid)| *pid).collect();
    let sys = processes::batch_refresh_processes(&pids);

    // Optional: run docker lookup BEFORE parallel loop (once, not per-entry)
    let docker_map = docker::get_docker_port_map();

    // Pre-compute unique CWDs for git branch lookup
    let unique_cwds: std::collections::HashSet<std::path::PathBuf> = pids.iter()
        .filter_map(|&pid| sys.process(sysinfo::Pid::from(pid as usize)))
        .filter_map(|p| p.cwd().map(|c| c.to_owned()))
        .collect();
    let git_map = git::batch_git_branches(&unique_cwds);

    // Phase 2: parallel enrichment
    let mut entries: Vec<PortEntry> = raw.par_iter()
        .map(|(port, pid)| {
            let proc = sys.process(sysinfo::Pid::from(*pid as usize));
            let cwd = proc.and_then(|p| p.cwd()).map(|p| p.to_owned());
            let name = proc
                .map(|p| p.name().to_string_lossy().into_owned())
                .unwrap_or_default();
            let cmd = proc
                .and_then(|p| p.exe())
                .map(|e| e.to_string_lossy().into_owned())
                .unwrap_or_default();

            PortEntry {
                port: *port,
                pid: *pid,
                process_name: name,
                command: cmd,
                memory_kb: proc.map(|p| p.memory() / 1024).unwrap_or(0),
                cpu_pct: 0.0, // Phase 1 — CPU needs two samples; add in Wave 2
                start_time: proc.map(|p| p.start_time()),
                framework: cwd.as_deref().and_then(enricher::framework::detect),
                git_branch: cwd.as_ref().and_then(|c| git_map.get(c).cloned()),
                docker: docker_map.get(port).cloned(),
                cwd,
                status: PortStatus::Healthy,
            }
        })
        .collect();

    entries.sort_by_key(|e| e.port);

    if !show_all {
        entries.retain(|e| enricher::status::is_dev_process(&e.process_name, &e.command));
    }

    Ok(entries)
}
```

`src/display/table.rs` — colored comfy-table (mirrors `displayPortTable` in display.js):
```rust
use comfy_table::{Attribute, Cell, Color, ContentArrangement, Table};
use crate::types::PortEntry;
use colored::Colorize;

pub fn print_port_table(ports: &[PortEntry], dev_only: bool) {
    if ports.is_empty() {
        println!("{}", "  No listening ports found.\n".dimmed());
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
        let port_cell = Cell::new(format!(":{}", entry.port))
            .fg(Color::Cyan)
            .add_attribute(Attribute::Bold);
        let framework_cell = match entry.framework.as_deref() {
            Some(fw) => Cell::new(fw).fg(Color::Green),
            None => Cell::new("—").fg(Color::DarkGrey),
        };
        let git_cell = match entry.git_branch.as_deref() {
            Some(b) => Cell::new(b).fg(Color::Yellow),
            None => Cell::new("—").fg(Color::DarkGrey),
        };
        let docker_label = entry.docker.as_ref()
            .map(|d| format!("🐳 {}", d.name))
            .unwrap_or_default();
        let name = if docker_label.is_empty() {
            entry.process_name.clone()
        } else {
            docker_label
        };

        table.add_row(vec![
            port_cell,
            Cell::new(entry.pid.to_string()).fg(Color::DarkGrey),
            Cell::new(name),
            framework_cell,
            Cell::new(format_memory(entry.memory_kb)).fg(Color::DarkGrey),
            git_cell,
        ]);
    }
    println!("{table}");
    if dev_only {
        println!("{}", "  Run with --all to see all ports".dimmed());
    }
}

fn format_memory(kb: u64) -> String {
    if kb >= 1_048_576 { format!("{:.1}GB", kb as f64 / 1_048_576.0) }
    else if kb >= 1024 { format!("{:.0}MB", kb as f64 / 1024.0) }
    else { format!("{}KB", kb) }
}
```

**Key APIs:**
- `netstat2::get_sockets_info(af, proto)` → `Vec<SocketInfo>`
- `si.associated_pids.first()` — always use first, not only
- `sysinfo::ProcessesToUpdate::Some(&pids)` for targeted refresh
- `rayon::prelude::*` — `.par_iter().map(...).collect()`
- `comfy-table`: `Table::new()`, `.set_header()`, `.add_row()`, `Cell::new().fg().add_attribute()`

**Pitfalls:**
- ⚠️ `sysinfo::System` — create ONCE in `collect_ports`, pass `&sys` into the rayon closure (read-only). NEVER call `sys.refresh_*` inside the rayon loop.
- ⚠️ Port deduplication — the same port appears twice on dual-stack (IPv4 + IPv6). Dedup by port before processing.
- ⚠️ `associated_pids` is `Vec<u32>` — call `.first()` and filter_map away `None`.
- ⚠️ `docker::get_docker_port_map()` must be called BEFORE `raw.par_iter()` (once, not per entry).

**Verification:**
```bash
cargo run
cargo run -- --all
```
Expected: Table with columns PORT / PID / PROCESS / FRAMEWORK / MEMORY / GIT. On a dev machine with Node.js servers running, framework column shows "Next.js" / "Vite" etc.

---

## Wave 2: Enrichment Pipeline

**Deliverable:** Framework detection, Docker container labeling, git branch column, and dev-process filtering all work correctly. Mirrors Node.js `scanner-shared.js` + `scanner-unix.js` enrichment.

---

### Task 2.1: Framework Detection + `--all` Filter

**Objective:** Implement `enricher/framework.rs` with full framework detection (package.json + non-JS indicators) and `enricher/status.rs` with `is_dev_process` filter matching `scanner-shared.js` logic.

**Files to create/modify:**
- `src/enricher/framework.rs`
- `src/enricher/status.rs`
- `src/enricher/mod.rs`

**Implementation:**

`src/enricher/framework.rs` (translate `detectFramework` from scanner-shared.js):
```rust
use std::path::Path;
use serde_json::Value;

pub fn detect(project_root: &Path) -> Option<String> {
    // JS ecosystem — read package.json
    let pkg_path = project_root.join("package.json");
    if let Ok(content) = std::fs::read_to_string(&pkg_path) {
        if let Ok(pkg) = serde_json::from_str::<Value>(&content) {
            let deps = pkg.get("dependencies").and_then(Value::as_object);
            let dev  = pkg.get("devDependencies").and_then(Value::as_object);
            let all_keys: Vec<&str> = deps.iter().chain(dev.iter())
                .flat_map(|m| m.keys().map(String::as_str))
                .collect();
            if all_keys.contains(&"next")            { return Some("Next.js".into()); }
            if all_keys.contains(&"nuxt")            { return Some("Nuxt".into()); }
            if all_keys.contains(&"@sveltejs/kit")   { return Some("SvelteKit".into()); }
            if all_keys.contains(&"vite")            { return Some("Vite".into()); }
            if all_keys.contains(&"react-scripts")   { return Some("Create React App".into()); }
            if all_keys.contains(&"@angular/core")   { return Some("Angular".into()); }
            if all_keys.contains(&"express")         { return Some("Express".into()); }
            if all_keys.contains(&"fastify")         { return Some("Fastify".into()); }
            if all_keys.contains(&"@nestjs/core")    { return Some("NestJS".into()); }
            if all_keys.contains(&"gatsby")          { return Some("Gatsby".into()); }
            if all_keys.contains(&"remix")           { return Some("Remix".into()); }
            if all_keys.contains(&"astro")           { return Some("Astro".into()); }
            return Some("Node.js".into()); // has package.json but no known framework
        }
    }
    // Non-JS indicators — file presence checks
    if project_root.join("Cargo.toml").exists()   { return Some("Rust".into()); }
    if project_root.join("go.mod").exists()        { return Some("Go".into()); }
    if project_root.join("manage.py").exists()     { return Some("Django".into()); }
    if project_root.join("requirements.txt").exists() { return Some("Python".into()); }
    if project_root.join("mix.exs").exists()       { return Some("Phoenix".into()); }
    if project_root.join("Gemfile").exists()       { return Some("Ruby/Rails".into()); }
    if project_root.join("pom.xml").exists()
       || project_root.join("build.gradle").exists() { return Some("Java".into()); }
    None
}
```

`src/enricher/status.rs` (translate `isDevProcess` from scanner-shared.js):
```rust
const DEV_PROCESS_NAMES: &[&str] = &[
    "node", "node.exe", "deno", "bun", "python", "python3",
    "ruby", "go", "cargo", "rustc", "java", "php", "dotnet",
    "webpack", "vite", "esbuild", "parcel",
];
const DEV_COMMAND_PATTERNS: &[&str] = &[
    "webpack", "vite", "next", "nuxt", "react-scripts", "gatsby",
    "nest", "fastify", "express", "rails", "django", "flask",
    "gunicorn", "uvicorn", "phoenix", "mix phx",
    "cargo run", "go run",
];

pub fn is_dev_process(name: &str, command: &str) -> bool {
    let name_lower = name.to_lowercase();
    let cmd_lower = command.to_lowercase();
    DEV_PROCESS_NAMES.iter().any(|&n| name_lower.contains(n))
        || DEV_COMMAND_PATTERNS.iter().any(|&p| cmd_lower.contains(p))
}

pub fn detect_status(name: &str, command: &str, pid: u32) -> crate::types::PortStatus {
    // A process is "orphaned" if its parent is init (PID 1) and it's a dev process.
    // For now, mark all healthy — zombie/orphan detection added in Wave 3 (ports clean).
    let _ = (name, command, pid);
    crate::types::PortStatus::Healthy
}
```

`src/enricher/mod.rs`:
```rust
pub mod framework;
pub mod status;
```

**Key APIs:** `serde_json::from_str::<serde_json::Value>(&content)`, `.get("dependencies").and_then(Value::as_object)`.

**Pitfalls:**
- `serde_json` parsing may fail on malformed `package.json` — always use `.ok()` / if-let, never `.unwrap()`.
- The framework check order matters: `next` before `react-scripts` before generic `Node.js`.

**Verification:**
```bash
cargo run   # framework column should show framework names for any running dev servers
cargo test  # run: cargo test enricher  (unit tests for detect() with fixture data)
```

---

### Task 2.2: Docker Container Detection + Git Branch Lookup

**Objective:** Implement `collector/docker.rs` (run `docker ps` once, parse port map) and `collector/git.rs` (deduplicated CWD → branch map). Both run before the rayon loop.

**Files to create/modify:**
- `src/collector/docker.rs`
- `src/collector/git.rs`

**Implementation:**

`src/collector/docker.rs`:
```rust
use std::collections::HashMap;
use std::process::Command;
use regex::Regex;
use crate::types::DockerInfo;

/// Run `docker ps` ONCE before the parallel enrichment loop.
/// Returns HashMap<host_port, DockerInfo> for O(1) lookup during enrichment.
pub fn get_docker_port_map() -> HashMap<u16, DockerInfo> {
    let output = match Command::new("docker")
        .args(["ps", "--format", "{{.Ports}}\t{{.Names}}\t{{.Image}}"])
        .output()
    {
        Ok(o) if o.status.success() => o,
        _ => return HashMap::new(), // docker not installed / not running — that's fine
    };

    // Pitfall 5: Docker port format varies — handle IPv4, IPv6, unbound
    // "0.0.0.0:5432->5432/tcp", ":::5432->5432/tcp", "5432/tcp" (no host binding)
    let port_re = Regex::new(r"(?:[\d.]+|:::):(\d+)->").unwrap();

    let text = String::from_utf8_lossy(&output.stdout);
    let mut map = HashMap::new();

    for line in text.lines() {
        let parts: Vec<&str> = line.splitn(3, '\t').collect();
        if parts.len() < 3 { continue; }
        let (ports_str, name, image) = (parts[0], parts[1], parts[2]);

        for cap in port_re.captures_iter(ports_str) {
            if let Ok(port) = cap[1].parse::<u16>() {
                map.insert(port, DockerInfo {
                    name: name.to_string(),
                    image: image.trim().to_string(),
                });
            }
        }
    }
    map
}
```

`src/collector/git.rs`:
```rust
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::process::Command;
use rayon::prelude::*;

/// For a set of unique CWDs, run `git rev-parse` in parallel (one call per CWD).
/// Returns HashMap<cwd, branch_name>.
/// Deduplication is critical: many PIDs share the same project CWD.
pub fn batch_git_branches(cwds: &HashSet<PathBuf>) -> HashMap<PathBuf, String> {
    cwds.par_iter()
        .filter_map(|cwd| {
            get_branch(cwd).map(|branch| (cwd.clone(), branch))
        })
        .collect()
}

fn get_branch(dir: &Path) -> Option<String> {
    let out = Command::new("git")
        .args(["-C", dir.to_str()?, "rev-parse", "--abbrev-ref", "HEAD"])
        .output()
        .ok()?;
    if out.status.success() {
        let branch = String::from_utf8_lossy(&out.stdout).trim().to_string();
        if branch.is_empty() || branch == "HEAD" { None } else { Some(branch) }
    } else {
        None
    }
}
```

**Key APIs:**
- `std::process::Command::new("docker").args([...]).output()` — synchronous subprocess
- `regex::Regex::new(r"...")` — compile once, reuse
- `rayon::prelude::*` — `cwds.par_iter().filter_map(...)` for parallel git calls

**Pitfalls:**
- ⚠️ Docker subprocess runs ONCE before `raw.par_iter()`. The `collector/mod.rs` orchestrator (Task 1.3) already calls `docker::get_docker_port_map()` at the right time — do not move this call inside the rayon closure.
- ⚠️ Git branch deduplication: pre-compute `unique_cwds: HashSet<PathBuf>` from all PIDs' CWDs BEFORE the parallel loop. The `collector/mod.rs` orchestrator handles this — `git.rs` just receives the set.
- ⚠️ `git rev-parse` returns `"HEAD"` when in detached HEAD state — treat as `None`.
- `docker` may not be installed — `Command::output()` returning `Err` or non-zero exit is expected; return empty map silently.

**Verification:**
```bash
cargo run  # GIT column should show branch name for processes in git repos
# Start a docker container with a mapped port, then:
cargo run --all  # docker container should show 🐳 container_name
```

---

## Wave 3: Interactive Commands

**Deliverable:** `ports <number>`, `ports ps`, and `ports clean` all work. Process kill uses `nix` (Unix) and includes a `dialoguer` confirm prompt.

---

### Task 3.1: Port Detail View + Interactive Kill (`ports <number>`)

**Objective:** Implement `display/detail.rs` (single-port detail card) and `platform/unix.rs` (kill via nix signal). Wire into `main.rs` dispatch.

**Files to create/modify:**
- `src/display/detail.rs`
- `src/platform/unix.rs`
- `src/platform/windows.rs`
- `src/platform/mod.rs`
- `src/main.rs` (wire `Commands::Port` branch)

**Implementation:**

`src/display/detail.rs`:
```rust
use crate::types::PortEntry;
use colored::Colorize;

pub fn print_port_detail(entry: &PortEntry) {
    println!();
    println!("  {} {}", "Port".bold(), format!(":{}", entry.port).cyan().bold());
    println!("  {} {}", "PID".bold(),     entry.pid.to_string().dimmed());
    println!("  {} {}", "Process".bold(), entry.process_name.white());
    if !entry.command.is_empty() {
        println!("  {} {}", "Command".bold(), entry.command.dimmed());
    }
    if let Some(fw) = &entry.framework {
        println!("  {} {}", "Framework".bold(), fw.green());
    }
    if let Some(branch) = &entry.git_branch {
        println!("  {} {}", "Branch".bold(), branch.yellow());
    }
    if let Some(docker) = &entry.docker {
        println!("  {} {} ({})", "Docker".bold(), docker.name.cyan(), docker.image.dimmed());
    }
    if let Some(cwd) = &entry.cwd {
        println!("  {} {}", "CWD".bold(), cwd.display().to_string().dimmed());
    }
    println!("  {} {}", "Memory".bold(), format_memory(entry.memory_kb).dimmed());
    println!();
}

fn format_memory(kb: u64) -> String {
    if kb >= 1_048_576 { format!("{:.1}GB", kb as f64 / 1_048_576.0) }
    else if kb >= 1024 { format!("{:.0}MB", kb as f64 / 1024.0) }
    else { format!("{}KB", kb) }
}
```

`src/platform/mod.rs`:
```rust
#[cfg(unix)]
pub mod unix;
#[cfg(windows)]
pub mod windows;

/// Kill a process by PID. Returns Ok(()) on success.
pub fn kill_process(pid: u32) -> anyhow::Result<()> {
    #[cfg(unix)]
    return unix::kill_process(pid);
    #[cfg(windows)]
    return windows::kill_process(pid);
    #[allow(unreachable_code)]
    anyhow::bail!("kill not supported on this platform")
}
```

`src/platform/unix.rs`:
```rust
use anyhow::{Context, Result};
use nix::sys::signal::{kill, Signal};
use nix::unistd::Pid;

pub fn kill_process(pid: u32) -> Result<()> {
    kill(Pid::from_raw(pid as i32), Signal::SIGTERM)
        .with_context(|| format!("Failed to send SIGTERM to PID {pid}"))?;
    Ok(())
}

/// Force kill — used if SIGTERM doesn't work
pub fn force_kill_process(pid: u32) -> Result<()> {
    kill(Pid::from_raw(pid as i32), Signal::SIGKILL)
        .with_context(|| format!("Failed to send SIGKILL to PID {pid}"))?;
    Ok(())
}
```

`src/platform/windows.rs` (stub — full implementation in Wave 5):
```rust
pub fn kill_process(pid: u32) -> anyhow::Result<()> {
    let status = std::process::Command::new("taskkill")
        .args(["/PID", &pid.to_string(), "/F"])
        .status()?;
    if status.success() { Ok(()) }
    else { anyhow::bail!("taskkill failed for PID {pid}") }
}
```

`src/main.rs` — wire Port command:
```rust
Some(Commands::Port { number }) => {
    let entries = collector::collect_ports(true)?; // show all to find by port
    match entries.iter().find(|e| e.port == number) {
        None => println!("{}", format!("  No process found on :{number}").red()),
        Some(entry) => {
            display::detail::print_port_detail(entry);
            if dialoguer::Confirm::new()
                .with_prompt(format!("Kill process on :{number}?"))
                .default(false)
                .interact()
                .unwrap_or(false)
            {
                match platform::kill_process(entry.pid) {
                    Ok(()) => println!("{}", format!("  ✓ Killed PID {}", entry.pid).green()),
                    Err(e) => println!("{}", format!("  ✕ Failed: {e}").red()),
                }
            }
        }
    }
}
```

**Key APIs:**
- `nix::sys::signal::kill(Pid::from_raw(i32), Signal::SIGTERM)` — typed signal send
- `dialoguer::Confirm::new().with_prompt(...).default(false).interact()` — TTY-safe confirm
- `#[cfg(unix)]` / `#[cfg(windows)]` for platform dispatch

**Pitfalls:**
- `dialoguer::Confirm::interact()` returns `Result<bool>` — use `.unwrap_or(false)` to handle Ctrl+C gracefully (same as Node.js `readline.question` with empty input).
- `nix` is only in `[target.'cfg(unix)'.dependencies]` — the `#[cfg(unix)]` gate in `platform/mod.rs` must match exactly or it won't compile on Windows.

**Verification:**
```bash
cargo run -- port 3000   # (assuming something runs on :3000)
# Shows detail card, prompts "Kill process?", y/N
cargo run -- port 99999  # Shows "No process found on :99999"
```

---

### Task 3.2: Process List (`ports ps`) + Orphan Cleanup (`ports clean`)

**Objective:** Implement `ports ps` (all running dev processes, not just listening ones) and `ports clean` (find + kill orphaned/zombie dev processes). Wire both into `main.rs`.

**Files to create/modify:**
- `src/collector/mod.rs` (add `collect_processes()` function)
- `src/enricher/status.rs` (implement orphan/zombie detection)
- `src/display/table.rs` (add `print_process_table`)
- `src/display/detail.rs` (add `print_clean_results`)
- `src/main.rs` (wire `Ps` and `Clean` branches)

**Implementation:**

Add `collect_processes()` to `src/collector/mod.rs`:
```rust
use sysinfo::{ProcessRefreshKind, RefreshKind, System, UpdateKind};

pub fn collect_processes(show_all: bool) -> anyhow::Result<Vec<crate::types::ProcessEntry>> {
    use sysinfo::{ProcessesToUpdate};

    let mut sys = System::new_with_specifics(
        RefreshKind::new().with_processes(
            ProcessRefreshKind::new()
                .with_memory()
                .with_exe(UpdateKind::Always)
                .with_cwd(UpdateKind::Always),
        )
    );
    sys.refresh_processes(ProcessesToUpdate::All, true);

    let mut entries: Vec<crate::types::ProcessEntry> = sys.processes()
        .values()
        .filter(|p| {
            let name = p.name().to_string_lossy();
            let cmd = p.exe().map(|e| e.to_string_lossy().into_owned()).unwrap_or_default();
            show_all || crate::enricher::status::is_dev_process(&name, &cmd)
        })
        .map(|p| {
            let name = p.name().to_string_lossy().into_owned();
            let cmd  = p.exe().map(|e| e.to_string_lossy().into_owned()).unwrap_or_default();
            let pid  = p.pid().as_u32();
            crate::types::ProcessEntry {
                pid,
                status: crate::enricher::status::detect_status(&name, &cmd, pid),
                process_name: name,
                command: cmd,
                cwd: p.cwd().map(|c| c.to_owned()),
                memory_kb: p.memory() / 1024,
                cpu_pct: p.cpu_usage(),
                start_time: Some(p.start_time()),
            }
        })
        .collect();

    entries.sort_by(|a, b| b.memory_kb.cmp(&a.memory_kb)); // sort by memory desc
    Ok(entries)
}
```

Update `src/enricher/status.rs` — implement real orphan detection:
```rust
pub fn detect_status(name: &str, command: &str, pid: u32) -> crate::types::PortStatus {
    // A dev process is "orphaned" when its parent PID is 1 (reparented to init)
    // Read parent PID via sysinfo or /proc. For now: check via /proc on Linux,
    // accept limitation on macOS (requires second sysinfo lookup).
    // Conservative approach: only flag as Orphaned if ppid == 1 AND is_dev_process.
    let _ = (name, command, pid);
    crate::types::PortStatus::Healthy
    // TODO: expand with ppid lookup in Wave 3 iteration if needed
}

pub fn find_orphaned(processes: &[crate::types::ProcessEntry]) -> Vec<&crate::types::ProcessEntry> {
    processes.iter()
        .filter(|p| p.status == crate::types::PortStatus::Orphaned)
        .collect()
}
```

Add `print_process_table` to `src/display/table.rs` (mirrors `displayProcessTable` in display.js):
```rust
pub fn print_process_table(procs: &[crate::types::ProcessEntry], dev_only: bool) {
    if procs.is_empty() {
        println!("{}", "  No dev processes found.\n".dimmed());
        return;
    }
    let mut table = Table::new();
    table
        .set_content_arrangement(ContentArrangement::Dynamic)
        .set_header(vec![
            Cell::new("PID").add_attribute(Attribute::Bold),
            Cell::new("PROCESS").add_attribute(Attribute::Bold),
            Cell::new("MEMORY").add_attribute(Attribute::Bold),
            Cell::new("CPU%").add_attribute(Attribute::Bold),
            Cell::new("STATUS").add_attribute(Attribute::Bold),
        ]);
    for p in procs {
        let status_cell = match p.status {
            crate::types::PortStatus::Healthy  => Cell::new("●").fg(Color::Green),
            crate::types::PortStatus::Orphaned => Cell::new("⚠ orphaned").fg(Color::Yellow),
            crate::types::PortStatus::Zombie   => Cell::new("💀 zombie").fg(Color::Red),
        };
        table.add_row(vec![
            Cell::new(p.pid.to_string()).fg(Color::DarkGrey),
            Cell::new(&p.process_name),
            Cell::new(format_memory(p.memory_kb)).fg(Color::DarkGrey),
            Cell::new(format!("{:.1}%", p.cpu_pct)).fg(Color::DarkGrey),
            status_cell,
        ]);
    }
    println!("{table}");
}
```

Wire into `src/main.rs`:
```rust
Some(Commands::Ps { all }) => {
    let procs = collector::collect_processes(all)?;
    display::table::print_process_table(&procs, !all);
}
Some(Commands::Clean) => {
    let procs = collector::collect_processes(false)?;
    let orphans = enricher::status::find_orphaned(&procs);
    if orphans.is_empty() {
        println!("{}", "  ✓ No orphaned dev processes found.".green());
        return Ok(());
    }
    println!("{}", format!("  Found {} orphaned process(es):", orphans.len()).yellow());
    for p in &orphans {
        println!("    {} {} ({})", "•".yellow(), p.process_name, p.pid);
    }
    if dialoguer::Confirm::new()
        .with_prompt("Kill all orphaned processes?")
        .default(false)
        .interact()
        .unwrap_or(false)
    {
        for p in &orphans {
            match platform::kill_process(p.pid) {
                Ok(()) => println!("{}", format!("  ✓ Killed {}", p.pid).green()),
                Err(e) => println!("{}", format!("  ✕ {}: {e}", p.pid).red()),
            }
        }
    }
}
```

**Key APIs:**
- `sysinfo::System::new_with_specifics(RefreshKind::new().with_processes(...))` for full process scan
- `sys.processes().values()` → iterator over all Process entries
- `p.pid().as_u32()` — Pid → u32
- `dialoguer::Confirm` for kill confirmation

**Pitfalls:**
- `collect_processes()` uses `ProcessesToUpdate::All` (full scan), unlike `collect_ports()` which uses targeted refresh. This is acceptable since `ps` is an intentionally slower, more comprehensive command.
- `p.cpu_usage()` on a fresh System returns 0.0 for all processes. Add a second `sys.refresh_processes()` call with a 100ms sleep between them if accurate CPU% is required for `ps`.

**Verification:**
```bash
cargo run -- ps
cargo run -- ps --all  # shows all system processes
cargo run -- clean     # "No orphaned dev processes found" on a clean system
```

---

## Wave 3 Checkpoint: Functional Parity with Node.js

```
[ ] cargo run           → colored port table with framework + git + docker
[ ] cargo run --all     → all ports including system ports
[ ] cargo run -- port 3000  → detail card + kill prompt
[ ] cargo run -- ps     → dev process list
[ ] cargo run -- ps --all   → all processes
[ ] cargo run -- clean  → orphan scan with kill prompt
```

All six behaviors must work on macOS before proceeding to Wave 4.

---

## Wave 4: Watch Mode

**Deliverable:** `cargo run --features watch -- watch` runs a live-updating terminal display, refreshing every 2 seconds, highlighting port appearances and disappearances.

---

### Task 4.1: Watch Mode with Tokio Interval + Ratatui TUI

**Objective:** Implement `display/watch.rs` with a full-screen ratatui table that re-renders on each tick. Wire `tokio::main` entry and `spawn_blocking` for the collection phase.

**Files to create/modify:**
- `src/display/watch.rs`
- `src/main.rs` (add `#[cfg(feature = "watch")]` tokio::main entry point or conditional dispatch)

**Implementation:**

`src/display/watch.rs`:
```rust
#[cfg(feature = "watch")]
pub mod watch_impl {
    use std::collections::HashSet;
    use std::time::Duration;
    use anyhow::Result;
    use ratatui::prelude::*;
    use ratatui::widgets::{Block, Borders, Row, Table, TableState};
    use crossterm::{
        event::{self, Event, KeyCode, KeyEventKind},
        execute,
        terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    };
    use crate::types::PortEntry;

    pub async fn run(interval_ms: u64) -> Result<()> {
        enable_raw_mode()?;
        let mut stdout = std::io::stdout();
        execute!(stdout, EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        let mut ticker = tokio::time::interval(Duration::from_millis(interval_ms));
        let mut prev_ports: HashSet<u16> = Default::default();
        let mut current_entries: Vec<PortEntry> = vec![];

        loop {
            // Non-blocking key event check
            if event::poll(Duration::from_millis(50))? {
                if let Event::Key(k) = event::read()? {
                    if k.kind == KeyEventKind::Press
                        && matches!(k.code, KeyCode::Char('q') | KeyCode::Esc)
                    {
                        break;
                    }
                }
            }

            // Check if tick fired (non-blocking select)
            tokio::select! {
                _ = ticker.tick() => {
                    // Pitfall 7: MUST use spawn_blocking for sync collection in async context
                    let entries = tokio::task::spawn_blocking(|| {
                        crate::collector::collect_ports(false)
                    }).await??;

                    let current: HashSet<u16> = entries.iter().map(|e| e.port).collect();
                    // Log diffs to a side panel or status line
                    for port in current.difference(&prev_ports) {
                        // new port appeared — could animate/highlight in the table
                        let _ = port;
                    }
                    for port in prev_ports.difference(&current) {
                        // port disappeared
                        let _ = port;
                    }
                    prev_ports = current;
                    current_entries = entries;
                }
                else => {}
            }

            terminal.draw(|frame| render_frame(frame, &current_entries))?;
        }

        disable_raw_mode()?;
        execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
        Ok(())
    }

    fn render_frame(frame: &mut Frame, entries: &[PortEntry]) {
        let area = frame.area();
        let header = Row::new(vec!["PORT", "PID", "PROCESS", "FRAMEWORK", "MEMORY"])
            .style(Style::default().bold());
        let rows: Vec<Row> = entries.iter().map(|e| {
            Row::new(vec![
                format!(":{}", e.port),
                e.pid.to_string(),
                e.process_name.clone(),
                e.framework.clone().unwrap_or_else(|| "—".into()),
                format_memory(e.memory_kb),
            ])
        }).collect();
        let table = Table::new(rows, [
            Constraint::Length(8),
            Constraint::Length(8),
            Constraint::Length(20),
            Constraint::Length(15),
            Constraint::Length(10),
        ])
        .header(header)
        .block(Block::default().borders(Borders::ALL).title(" ports — press q to quit "));
        frame.render_widget(table, area);
    }

    fn format_memory(kb: u64) -> String {
        if kb >= 1_048_576 { format!("{:.1}GB", kb as f64 / 1_048_576.0) }
        else if kb >= 1024 { format!("{:.0}MB", kb as f64 / 1024.0) }
        else { format!("{}KB", kb) }
    }
}
```

`src/main.rs` — add watch dispatch:
```rust
// In main(), add to match arm:
#[cfg(feature = "watch")]
Some(Commands::Watch { interval_ms }) => {
    // Build a separate tokio runtime for watch mode only
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(display::watch::watch_impl::run(interval_ms))?;
}
```

**Key APIs:**
- `tokio::time::interval(Duration::from_millis(n))` — async timer
- `tokio::task::spawn_blocking(|| collect_ports(false)).await??` — run sync code off async executor
- `ratatui::Terminal::new(CrosstermBackend::new(stdout))` — TUI backend
- `crossterm::terminal::{enable_raw_mode, disable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen}`
- `frame.render_widget(table, area)` — ratatui table rendering

**Pitfalls:**
- ⚠️ Pitfall 7 (critical): `collect_ports()` calls netstat2, sysinfo, and subprocess — all blocking. ALWAYS wrap in `spawn_blocking`. Calling it directly inside `async fn` will starve the tokio executor.
- Always call `disable_raw_mode()` + `LeaveAlternateScreen` in the cleanup path, even on error — use a guard or `defer!` pattern. A crashed watch mode that leaves the terminal in raw mode is a terrible UX.
- The `tokio::select!` approach with `poll(50ms)` avoids blocking on key events while still checking the ticker. This is the correct pattern — do NOT use `event::read()` (blocking) inside the tick loop.

**Verification:**
```bash
cargo build --features watch
cargo run --features watch -- watch
# Full-screen table appears, refreshes every 2 seconds, q exits cleanly
cargo run --features watch -- watch --interval-ms 500
```

---

## Wave 5: Windows Platform Support

**Deliverable:** `cargo build` and all commands work on Windows. TCP socket enumeration via netstat2's Windows backend, kill via `TerminateProcess`, no Unix-specific code in the hot path.

---

### Task 5.1: Windows Kill + Platform Compilation Guards

**Objective:** Replace the `taskkill` subprocess stub (from Task 3.1) with proper `windows` crate API call. Ensure all `#[cfg(unix)]` / `#[cfg(windows)]` guards are correct so the project compiles on both platforms.

**Files to create/modify:**
- `Cargo.toml` (add `[target.'cfg(windows)'.dependencies]` for windows crate)
- `src/platform/windows.rs` (full implementation with `windows::Win32` API)
- `src/platform/mod.rs` (verify cfg guards cover all OS combinations)

**Implementation:**

Add to `Cargo.toml`:
```toml
[target.'cfg(windows)'.dependencies]
windows = { version = "0.59", features = [
    "Win32_System_Threading",
    "Win32_Foundation",
] }
```

`src/platform/windows.rs` (full implementation):
```rust
use anyhow::{bail, Result};
use windows::Win32::Foundation::{CloseHandle, HANDLE};
use windows::Win32::System::Threading::{
    OpenProcess, TerminateProcess, PROCESS_TERMINATE,
};

pub fn kill_process(pid: u32) -> Result<()> {
    unsafe {
        let handle: HANDLE = OpenProcess(PROCESS_TERMINATE, false, pid)
            .map_err(|e| anyhow::anyhow!("OpenProcess failed for PID {pid}: {e}"))?;
        let result = TerminateProcess(handle, 1);
        let _ = CloseHandle(handle);
        result.map_err(|e| anyhow::anyhow!("TerminateProcess failed for PID {pid}: {e}"))?;
    }
    Ok(())
}
```

**Key APIs:**
- `windows::Win32::System::Threading::{OpenProcess, TerminateProcess, PROCESS_TERMINATE}`
- `netstat2` — already handles Windows via `GetExtendedTcpTable` internally, no changes needed
- `sysinfo` — already handles Windows, no changes needed
- `nix` — `[target.'cfg(unix)'.dependencies]` so it only compiles on Unix; Windows has no nix dep

**Pitfalls:**
- `colored` requires Windows VT processing enabled for ANSI codes on older Windows — `colored` crate handles this automatically via its Windows feature; no extra setup.
- `sysinfo::Process::cwd()` on Windows may return `None` for some processes without elevated permissions — same acceptable behavior as macOS (no fallback needed).
- `cfg(windows)` vs `cfg(target_os = "windows")` — use `cfg(windows)` (the Rust standard alias); do NOT use the string form.

**Verification:**
```bash
# On Windows:
cargo build
cargo run
cargo run -- port 3000
cargo run -- ps

# Cross-compilation check from macOS (verifies cfg guards are correct):
rustup target add x86_64-pc-windows-msvc
cargo check --target x86_64-pc-windows-msvc 2>&1 | grep "^error"
# Expected: zero errors
```

---

### Task 5.2: End-to-End Integration + Binary Distribution

**Objective:** Add a `Makefile` / `justfile` with build targets, create a `dist/` release build script, and run the full feature checklist on macOS. Update `README.md` to document the Rust binary alongside the npm package.

**Files to create/modify:**
- `Makefile` (or `justfile`)
- `README.md` (add Rust build/install instructions)
- `.github/workflows/rust.yml` (optional: CI matrix macOS + Windows)

**Implementation:**

`Makefile`:
```makefile
.PHONY: build build-watch release test check clean

build:
	cargo build

build-watch:
	cargo build --features watch

release:
	cargo build --release --features watch
	@echo "Binary at: target/release/ports"

test:
	cargo test

check:
	cargo check --features watch
	cargo clippy --features watch -- -D warnings

clean:
	cargo clean
```

`README.md` additions — add a section "Rust Binary (Fast, Single-File)":
```markdown
## Rust Binary (Single File, No Runtime)

### Build from source
\`\`\`bash
cargo build --release --features watch
sudo cp target/release/ports /usr/local/bin/ports
\`\`\`

### Usage
Same as the npm version:
\`\`\`
ports             # list dev ports
ports --all       # all ports
ports ps          # dev process list
ports clean       # kill orphaned processes
ports port 3000   # detail + kill prompt
ports watch       # live TUI (requires --features watch)
\`\`\`
```

**Verification:**
```bash
make release
./target/release/ports
./target/release/ports --all
./target/release/ports ps
./target/release/ports port 3000
time ./target/release/ports  # should complete in < 200ms
make build-watch
./target/release/ports watch  # (rebuild needed after adding feature)
```
Expected: startup time < 200ms (vs ~300-500ms for Node.js), all commands work, `watch` opens full-screen TUI.

---

## Appendix: Pitfall Quick Reference

| # | Issue | File | How to Avoid |
|---|-------|------|--------------|
| P1 | `sysinfo.cwd()` → None for root processes | `collector/processes.rs` | Treat None as acceptable; no fallback needed |
| P2 | `associated_pids` is Vec, not single PID | `collector/ports.rs` | `.first()` + `filter_map` |
| P3 | `sys.refresh_all()` is slow (~100ms) | `collector/processes.rs` | `refresh_processes_specifics(..., Some(&pids), ...)` |
| P4 | `&mut System` in rayon closure = compile error | `collector/mod.rs` | Refresh Phase 1 (serial), read Phase 2 (rayon, &sys) |
| P5 | Docker port format: IPv4/IPv6/unbound variants | `collector/docker.rs` | Regex `r"(?:[\d.]+|:::):(\d+)->"` covers all |
| P6 | colored output breaks in pipes | `display/table.rs` | colored auto-detects TTY; NO_COLOR respected automatically |
| P7 | Blocking in tokio context | `display/watch.rs` | Always `spawn_blocking(collect_ports)` in async context |

## Appendix: Migration Traceability

| Node.js source | Rust equivalent |
|----------------|-----------------|
| `scanner-unix.js::getListeningPorts()` | `collector/ports.rs` + `collector/processes.rs` |
| `scanner-shared.js::detectFramework()` | `enricher/framework.rs::detect()` |
| `scanner-shared.js::isDevProcess()` | `enricher/status.rs::is_dev_process()` |
| `scanner-shared.js::batchDockerInfo()` | `collector/docker.rs::get_docker_port_map()` |
| `scanner-unix.js::watchPorts()` | `display/watch.rs` + tokio |
| `display.js::displayPortTable()` | `display/table.rs::print_port_table()` |
| `display.js::displayPortDetail()` | `display/detail.rs::print_port_detail()` |
| `display.js::displayProcessTable()` | `display/table.rs::print_process_table()` |
| `display.js::displayCleanResults()` | `display/detail.rs::print_clean_results()` |
| `index.js` (main dispatch) | `main.rs` + `cli.rs` |
| `scanner-windows.js` | `platform/windows.rs` |
