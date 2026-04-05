use crate::types::{PortStatus, ProcessEntry, ProcessTreeNode};
use std::path::{Path, PathBuf};
use std::thread::sleep;
use std::time::Duration;
use sysinfo::{
    Pid, Process, ProcessRefreshKind, ProcessStatus, ProcessesToUpdate, System, UpdateKind,
};

const PROJECT_MARKERS: &[&str] = &[
    "package.json",
    "Cargo.toml",
    "go.mod",
    "pyproject.toml",
    "Gemfile",
    "pom.xml",
    "build.gradle",
];

pub fn refresh_processes(pids: &[u32], with_cpu: bool) -> System {
    let mut sys = System::new();
    let sysinfo_pids: Vec<Pid> = pids.iter().map(|&pid| Pid::from_u32(pid)).collect();
    let refresh_kind = refresh_kind(with_cpu);

    sys.refresh_processes_specifics(
        ProcessesToUpdate::Some(&sysinfo_pids),
        true,
        refresh_kind,
    );
    if with_cpu {
        sleep(Duration::from_millis(150));
        sys.refresh_processes_specifics(
            ProcessesToUpdate::Some(&sysinfo_pids),
            true,
            refresh_kind,
        );
    }

    sys
}

pub fn refresh_all_processes(with_cpu: bool) -> System {
    let mut sys = System::new();
    let refresh_kind = refresh_kind(with_cpu);

    sys.refresh_processes_specifics(ProcessesToUpdate::All, true, refresh_kind);
    if with_cpu {
        sleep(Duration::from_millis(150));
        sys.refresh_processes_specifics(ProcessesToUpdate::All, true, refresh_kind);
    }

    sys
}

pub fn command_line(process: &Process) -> String {
    let cmd = join_os(process.cmd());
    if !cmd.is_empty() {
        return cmd;
    }
    if let Some(exe) = process.exe() {
        return exe.to_string_lossy().into_owned();
    }
    process.name().to_string_lossy().into_owned()
}

pub fn process_name(process: &Process) -> String {
    process.name().to_string_lossy().into_owned()
}

pub fn project_root_from_cwd(cwd: Option<&Path>) -> Option<PathBuf> {
    let mut current = cwd?.to_path_buf();

    for _ in 0..15 {
        if PROJECT_MARKERS
            .iter()
            .any(|marker| current.join(marker).exists())
        {
            return Some(current);
        }
        if !current.pop() {
            break;
        }
    }

    None
}

pub fn summarize_command(command: &str, process_name: &str) -> String {
    let mut meaningful = Vec::new();
    for (index, token) in command.split_whitespace().enumerate() {
        if index == 0 || token.starts_with('-') {
            continue;
        }
        let candidate = token.trim_matches('"');
        if candidate.contains('/') || candidate.contains('\\') {
            if let Some(name) = Path::new(candidate).file_name() {
                meaningful.push(name.to_string_lossy().into_owned());
            }
        } else {
            meaningful.push(candidate.to_string());
        }
        if meaningful.len() >= 3 {
            break;
        }
    }

    if meaningful.is_empty() {
        process_name.to_string()
    } else {
        meaningful.join(" ")
    }
}

pub fn detect_status(process: &Process, is_dev_process: bool) -> PortStatus {
    if matches!(process.status(), ProcessStatus::Zombie) {
        return PortStatus::Zombie;
    }

    if is_dev_process {
        if let Some(parent) = process.parent() {
            if parent.as_u32() <= 1 {
                return PortStatus::Orphaned;
            }
        }
    }

    PortStatus::Healthy
}

pub fn process_tree(pid: u32, sys: &System) -> Vec<ProcessTreeNode> {
    let mut tree = Vec::new();
    let mut current = Some(Pid::from_u32(pid));
    let mut depth = 0;

    while let Some(current_pid) = current {
        if depth >= 8 {
            break;
        }
        let Some(process) = sys.process(current_pid) else {
            break;
        };
        tree.push(ProcessTreeNode {
            pid: current_pid.as_u32(),
            name: process_name(process),
        });
        current = process.parent();
        depth += 1;
    }

    tree
}

pub fn build_process_entry(
    process: &Process,
    status: PortStatus,
    project_name: Option<String>,
    framework: Option<String>,
) -> ProcessEntry {
    let process_name = process_name(process);
    let command = command_line(process);

    ProcessEntry {
        pid: process.pid().as_u32(),
        process_name: process_name.clone(),
        description: summarize_command(&command, &process_name),
        cwd: project_root_from_cwd(process.cwd()),
        project_name,
        framework,
        memory_kb: process.memory() / 1024,
        cpu_pct: process.cpu_usage(),
        start_time: Some(process.start_time()),
        status,
    }
}

fn join_os(parts: &[std::ffi::OsString]) -> String {
    parts
        .iter()
        .map(|part| part.to_string_lossy())
        .collect::<Vec<_>>()
        .join(" ")
}

fn refresh_kind(with_cpu: bool) -> ProcessRefreshKind {
    let mut kind = ProcessRefreshKind::nothing()
        .with_memory()
        .with_exe(UpdateKind::Always)
        .with_cwd(UpdateKind::Always)
        .with_cmd(UpdateKind::Always);
    if with_cpu {
        kind = kind.with_cpu();
    }
    kind
}

#[cfg(test)]
mod tests {
    use super::project_root_from_cwd;
    use std::fs;
    use std::path::PathBuf;

    #[test]
    fn project_root_requires_marker() {
        let temp = std::env::temp_dir().join(format!(
            "port-whisperer-test-{}",
            std::process::id()
        ));
        let nested = temp.join("runtime/bin");
        fs::create_dir_all(&nested).unwrap();
        assert_eq!(project_root_from_cwd(Some(&nested)), None);
        fs::create_dir_all(temp.join("app/src")).unwrap();
        fs::write(temp.join("app/Cargo.toml"), "[package]\nname='x'\nversion='0.1.0'\n").unwrap();
        let project = temp.join("app/src");
        assert_eq!(project_root_from_cwd(Some(&project)), Some(PathBuf::from(temp.join("app"))));
        let _ = fs::remove_dir_all(temp);
    }
}
