#[cfg(unix)]
pub mod unix;
#[cfg(windows)]
pub mod windows;

pub fn kill_process(pid: u32) -> anyhow::Result<()> {
    #[cfg(unix)]
    return unix::kill_process(pid);
    #[cfg(windows)]
    return windows::kill_process(pid);
    #[cfg(not(any(unix, windows)))]
    Err(anyhow::anyhow!("kill_process not supported on this platform"))
}
