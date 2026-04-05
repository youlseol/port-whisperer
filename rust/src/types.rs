use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct PortEntry {
    pub port: u16,
    pub pid: u32,
    pub process_name: String,
    pub command: String,
    pub cwd: Option<PathBuf>,
    pub project_name: Option<String>,
    pub framework: Option<String>,
    pub git_branch: Option<String>,
    pub memory_kb: u64,
    pub start_time: Option<u64>,
    pub status: PortStatus,
    pub docker: Option<DockerInfo>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
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
    pub description: String,
    pub cwd: Option<PathBuf>,
    pub project_name: Option<String>,
    pub framework: Option<String>,
    pub memory_kb: u64,
    pub cpu_pct: f32,
    pub start_time: Option<u64>,
    pub status: PortStatus,
}

#[derive(Debug, Clone)]
pub struct ProcessTreeNode {
    pub pid: u32,
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct CleanResult {
    pub entry: PortEntry,
    pub killed: bool,
    pub error: Option<String>,
}
