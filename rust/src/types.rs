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
    pub start_time: Option<u64>,
    pub status: PortStatus,
    pub docker: Option<DockerInfo>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PortStatus {
    Healthy,
    Zombie,
    Orphaned,
}

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
