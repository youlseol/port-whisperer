pub fn kill_process(pid: u32) -> anyhow::Result<()> {
    let status = std::process::Command::new("taskkill")
        .args(["/F", "/PID", &pid.to_string()])
        .status()?;
    if status.success() {
        Ok(())
    } else {
        Err(anyhow::anyhow!("taskkill failed for PID {}", pid))
    }
}
