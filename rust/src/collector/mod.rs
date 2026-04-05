pub mod docker;
pub mod git;
pub mod ports;
pub mod processes;

use crate::enricher;
use crate::types::{PortEntry, ProcessEntry, ProcessTreeNode};
use anyhow::Result;
use rayon::prelude::*;
use std::collections::HashSet;
use sysinfo::Pid;

pub fn collect_ports(show_all: bool) -> Result<Vec<PortEntry>> {
    let raw = ports::get_listening_sockets()?;
    if raw.is_empty() {
        return Ok(vec![]);
    }

    let pids: Vec<u32> = raw.iter().map(|(_, pid)| *pid).collect();
    let sys = processes::refresh_processes(&pids, false);
    let docker_map = docker::get_docker_port_map();

    let unique_cwds: HashSet<std::path::PathBuf> = pids
        .iter()
        .filter_map(|&pid| sys.process(Pid::from(pid as usize)))
        .filter_map(|p| processes::project_root_from_cwd(p.cwd()))
        .collect();
    let git_map = git::batch_git_branches(&unique_cwds);

    let mut entries: Vec<PortEntry> = raw
        .par_iter()
        .map(|(port, pid)| {
            let proc = sys.process(Pid::from(*pid as usize));
            let cwd = proc.and_then(|p| processes::project_root_from_cwd(p.cwd()));
            let process_name = proc.map(processes::process_name).unwrap_or_default();
            let command = proc.map(processes::command_line).unwrap_or_default();
            let is_dev = enricher::status::is_dev_process(&process_name, &command);
            let status = proc
                .map(|p| processes::detect_status(p, is_dev))
                .unwrap_or(crate::types::PortStatus::Healthy);
            let framework = cwd
                .as_deref()
                .and_then(enricher::framework::detect)
                .or_else(|| {
                    docker_map
                        .get(port)
                        .map(|d| infer_docker_framework(&d.image))
                });
            let project_name = cwd
                .as_ref()
                .and_then(|root| {
                    root.file_name()
                        .map(|name| name.to_string_lossy().into_owned())
                })
                .or_else(|| docker_map.get(port).map(|d| d.name.clone()));

            PortEntry {
                port: *port,
                pid: *pid,
                process_name: process_name.clone(),
                command: command.clone(),
                memory_kb: proc.map(|p| p.memory() / 1024).unwrap_or(0),
                start_time: proc.map(|p| p.start_time()),
                project_name,
                framework,
                git_branch: cwd.as_ref().and_then(|c| git_map.get(c.as_path()).cloned()),
                docker: docker_map.get(port).cloned(),
                status,
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

pub fn collect_processes(show_all: bool) -> Vec<ProcessEntry> {
    let sys = processes::refresh_all_processes(true);
    let mut processes_list: Vec<ProcessEntry> = sys
        .processes()
        .values()
        .filter(|process| {
            process.pid().as_u32() > 1 && process.pid().as_u32() != std::process::id()
        })
        .filter_map(|process| {
            let process_name = processes::process_name(process);
            let command = processes::command_line(process);
            let is_dev = enricher::status::is_dev_process(&process_name, &command);
            if !show_all && !is_dev {
                return None;
            }

            let cwd = processes::project_root_from_cwd(process.cwd());
            let project_name = cwd.as_ref().and_then(|root| {
                root.file_name()
                    .map(|name| name.to_string_lossy().into_owned())
            });
            let framework = cwd
                .as_deref()
                .and_then(enricher::framework::detect)
                .or_else(|| infer_framework_from_process_name(&process_name));
            let mut entry = processes::build_process_entry(
                process,
                processes::detect_status(process, is_dev),
                project_name,
                framework,
            );
            entry.cwd = cwd;
            Some(entry)
        })
        .collect();

    processes_list.sort_by(|left, right| {
        right
            .cpu_pct
            .partial_cmp(&left.cpu_pct)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    processes_list
}

pub fn get_port_detail(
    port: u16,
) -> Result<Option<(PortEntry, Vec<crate::types::ProcessTreeNode>)>> {
    let entries = collect_ports(true)?;
    let Some(entry) = entries.into_iter().find(|entry| entry.port == port) else {
        return Ok(None);
    };

    let tree = get_process_tree(entry.pid);
    Ok(Some((entry, tree)))
}

pub fn get_process_tree(pid: u32) -> Vec<ProcessTreeNode> {
    let sys = processes::refresh_all_processes(false);
    processes::process_tree(pid, &sys)
}

fn infer_docker_framework(image: &str) -> String {
    let image = image.to_lowercase();
    if image.contains("postgres") {
        "PostgreSQL".into()
    } else if image.contains("redis") {
        "Redis".into()
    } else if image.contains("mysql") || image.contains("mariadb") {
        "MySQL".into()
    } else if image.contains("mongo") {
        "MongoDB".into()
    } else if image.contains("localstack") {
        "LocalStack".into()
    } else if image.contains("nginx") {
        "nginx".into()
    } else {
        "Docker".into()
    }
}

fn infer_framework_from_process_name(process_name: &str) -> Option<String> {
    match process_name.to_lowercase().as_str() {
        "node" | "node.exe" => Some("Node.js".into()),
        "bun" | "bun.exe" => Some("Bun".into()),
        "python" | "python.exe" | "python3" | "python3.exe" => Some("Python".into()),
        "ruby" | "ruby.exe" => Some("Ruby".into()),
        "go" | "go.exe" => Some("Go".into()),
        _ => None,
    }
}
