use anyhow::Result;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn get_pid_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("~/.config"))
        .join("santity")
        .join("santity.pid")
}

pub async fn execute() -> Result<()> {
    println!("🛑 Stopping Santity Core daemon...");

    // Try systemd user service stop first
    let systemctl_res = Command::new("systemctl")
        .args(["--user", "stop", "santity.service"])
        .status();

    if systemctl_res.map(|s| s.success()).unwrap_or(false) {
        println!("✅ Stopped systemd user service santity.service");
    }

    // Fallback PID file check
    let pid_path = get_pid_path();
    if pid_path.exists() {
        if let Ok(pid_str) = fs::read_to_string(&pid_path) {
            if let Ok(pid) = pid_str.trim().parse::<i32>() {
                #[cfg(unix)]
                unsafe {
                    libc::kill(pid, libc::SIGTERM);
                }
            }
        }
        let _ = fs::remove_file(&pid_path);
    }

    let _ = fs::remove_file("/tmp/santity.sock");
    println!("✅ Santity Core daemon stopped successfully.");
    Ok(())
}
