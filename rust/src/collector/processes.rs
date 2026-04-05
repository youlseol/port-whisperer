use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System, UpdateKind};

/// Create and refresh a System for the given PIDs only (batch — not per-PID).
/// Caller must NOT call refresh again after this; pass &System read-only.
pub fn batch_refresh_processes(pids: &[u32]) -> System {
    let mut sys = System::new();
    let sysinfo_pids: Vec<Pid> = pids.iter().map(|&p| Pid::from(p as usize)).collect();

    sys.refresh_processes_specifics(
        ProcessesToUpdate::Some(&sysinfo_pids),
        true,
        ProcessRefreshKind::nothing()
            .with_memory()
            .with_exe(UpdateKind::Always)
            .with_cwd(UpdateKind::Always),
    );
    sys
}
