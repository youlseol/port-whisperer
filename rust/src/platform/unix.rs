pub fn kill_process(pid: u32) -> anyhow::Result<()> {
    #[cfg(unix)]
    {
        use nix::sys::signal::{kill, Signal};
        use nix::unistd::Pid;
        kill(Pid::from_raw(pid as i32), Signal::SIGTERM)
            .map_err(|e| anyhow::anyhow!("Failed to kill PID {}: {}", pid, e))
    }
    #[cfg(not(unix))]
    Err(anyhow::anyhow!("kill_process not supported on this platform"))
}
