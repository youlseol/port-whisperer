pub mod docker;
pub mod git;
pub mod ports;
pub mod processes;

use crate::enricher;
use crate::types::{PortEntry, PortStatus};
use anyhow::Result;
use rayon::prelude::*;
use std::collections::HashSet;
use sysinfo::Pid;

pub fn collect_ports(show_all: bool) -> Result<Vec<PortEntry>> {
    // Phase 1a: Get all listening TCP sockets
    let raw = ports::get_listening_sockets()?;
    if raw.is_empty() {
        return Ok(vec![]);
    }

    // Phase 1b: Batch process refresh — ONE System, all PIDs at once
    let pids: Vec<u32> = raw.iter().map(|(_, pid)| *pid).collect();
    let sys = processes::batch_refresh_processes(&pids);

    // Pre-compute: docker port map (once, before parallel loop)
    let docker_map = docker::get_docker_port_map();

    // Pre-compute: unique CWDs for deduplicated git branch lookup
    let unique_cwds: HashSet<std::path::PathBuf> = pids
        .iter()
        .filter_map(|&pid| sys.process(Pid::from(pid as usize)))
        .filter_map(|p| p.cwd().map(|c| c.to_owned()))
        .collect();
    let git_map = git::batch_git_branches(&unique_cwds);

    // Phase 2: Parallel enrichment — &sys is read-only here (no refresh calls)
    let mut entries: Vec<PortEntry> = raw
        .par_iter()
        .map(|(port, pid)| {
            let proc = sys.process(Pid::from(*pid as usize));
            let cwd = proc.and_then(|p| p.cwd()).map(|p| p.to_owned());
            let process_name = proc
                .map(|p| p.name().to_string_lossy().into_owned())
                .unwrap_or_default();
            let command = proc
                .and_then(|p| p.exe())
                .map(|e| e.to_string_lossy().into_owned())
                .unwrap_or_default();

            PortEntry {
                port: *port,
                pid: *pid,
                process_name: process_name.clone(),
                command: command.clone(),
                memory_kb: proc.map(|p| p.memory() / 1024).unwrap_or(0),
                cpu_pct: 0.0, // requires two sysinfo samples; add in Wave 2
                start_time: proc.map(|p| p.start_time()),
                framework: cwd.as_deref().and_then(enricher::framework::detect),
                git_branch: cwd.as_ref().and_then(|c| git_map.get(c.as_path()).cloned()),
                docker: docker_map.get(port).cloned(),
                status: PortStatus::Healthy,
                cwd,
            }
        })
        .collect();

    entries.sort_by_key(|e| e.port);

    if !show_all {
        entries.retain(|e| enricher::status::is_dev_process(&e.process_name, &e.command));
    }

    Ok(entries)
}
