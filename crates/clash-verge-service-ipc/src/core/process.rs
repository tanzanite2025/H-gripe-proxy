use tracing::warn;

pub(super) fn is_process_alive(pid: u32) -> bool {
    let filter = format!("PID eq {pid}");
    std::process::Command::new("tasklist")
        .args(["/FI", &filter, "/FO", "CSV", "/NH"])
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| String::from_utf8_lossy(&output.stdout).contains(&pid.to_string()))
        .unwrap_or(false)
}

pub(super) async fn terminate_process(pid: u32) {
    warn!("Terminating process {}", pid);
    let pid_arg = pid.to_string();
    let _ = std::process::Command::new("taskkill")
        .args(["/PID", pid_arg.as_str(), "/T", "/F"])
        .status();
}
