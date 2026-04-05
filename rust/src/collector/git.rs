use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::process::Command;

pub fn get_git_branch(dir: &Path) -> Option<String> {
    let out = Command::new("git")
        .args(["-C", dir.to_str()?, "rev-parse", "--abbrev-ref", "HEAD"])
        .output()
        .ok()?;
    if out.status.success() {
        let branch = String::from_utf8_lossy(&out.stdout).trim().to_string();
        if branch == "HEAD" {
            None
        } else {
            Some(branch)
        }
    } else {
        None
    }
}

/// Batch git branch lookup for a set of unique CWDs.
/// Deduplicates — same CWD shared by multiple processes is only queried once.
pub fn batch_git_branches(cwds: &HashSet<PathBuf>) -> HashMap<PathBuf, String> {
    cwds.iter()
        .filter_map(|cwd| get_git_branch(cwd).map(|branch| (cwd.clone(), branch)))
        .collect()
}
