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

fn is_santity_process(pid: i32) -> bool {
    #[cfg(target_os = "linux")]
    {
        if let Ok(cmdline) = fs::read_to_string(format!("/proc/{}/cmdline", pid)) {
            if cmdline.contains("santity-core") || cmdline.contains("santity") {
                return true;
            }
        }
        if let Ok(comm) = fs::read_to_string(format!("/proc/{}/comm", pid)) {
            if comm.contains("santity-core") || comm.contains("santity") {
                return true;
            }
        }
    }

    if let Ok(out) = Command::new("ps")
        .args(["-p", &pid.to_string(), "-o", "command="])
        .output()
    {
        if out.status.success() {
            let cmd = String::from_utf8_lossy(&out.stdout);
            return cmd.contains("santity-core") || cmd.contains("santity");
        }
    }

    false
}

pub async fn execute() -> Result<()> {
    println!("› Stopping Santity Core daemon...");

    // Stop launchd agent first (macOS)
    if cfg!(target_os = "macos") {
        let uid = unsafe { libc::getuid() };
        let bootout_res = Command::new("launchctl")
            .args(["bootout", &format!("gui/{uid}/com.santity.core")])
            .status();

        if bootout_res.map(|s| s.success()).unwrap_or(false) {
            println!("[OK] Stopped launchd agent com.santity.core");
        }
    }

    // Try systemd user service stop (Linux)
    let systemctl_res = Command::new("systemctl")
        .args(["--user", "stop", "santity.service"])
        .status();

    if systemctl_res.map(|s| s.success()).unwrap_or(false) {
        println!("[OK] Stopped systemd user service santity.service");
    }

    // Fallback PID file check with process identity verification
    let pid_path = get_pid_path();
    if pid_path.exists() {
        if let Ok(pid_str) = fs::read_to_string(&pid_path) {
            if let Ok(pid) = pid_str.trim().parse::<i32>() {
                if is_santity_process(pid) {
                    #[cfg(unix)]
                    unsafe {
                        libc::kill(pid, libc::SIGTERM);
                    }
                    println!("[OK] Sent SIGTERM to santity-core process (PID: {})", pid);
                } else {
                    println!(
                        "[WARN] Process {} is not santity-core (PID was likely recycled). Skipping SIGTERM.",
                        pid
                    );
                }
            }
        }
        let _ = fs::remove_file(&pid_path);
    }

    let socket_path = crate::commands::default_socket_path();
    let _ = fs::remove_file(&socket_path);
    println!("[OK] Santity Core daemon stopped successfully.");
    Ok(())
}
