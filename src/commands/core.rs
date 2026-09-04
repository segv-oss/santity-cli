use anyhow::{Context, Result};
use inquire::Confirm;
use std::path::PathBuf;
use std::process::Command;

pub fn find_core_binary() -> Option<PathBuf> {
    // 1. Check system PATH
    if let Ok(path) = which::which("santity-core") {
        return Some(path);
    }

    // 2. Check ~/.santity/bin/santity-core
    if let Some(home) = dirs::home_dir() {
        let santity_bin = home.join(".santity").join("bin").join("santity-core");
        if santity_bin.exists() {
            return Some(santity_bin);
        }

        // 3. Check ~/.cargo/bin/santity-core
        let cargo_bin = home.join(".cargo").join("bin").join("santity-core");
        if cargo_bin.exists() {
            return Some(cargo_bin);
        }
    }

    None
}

pub async fn ensure_core_installed() -> Result<PathBuf> {
    if let Some(path) = find_core_binary() {
        return Ok(path);
    }

    println!("[WARN] Santity Core runtime engine (`santity-core`) was not found in PATH or ~/.santity/bin/");
    let prompt_msg = "Would you like santity-cli to install `santity-core` via `cargo install santity-core` now?";
    let should_install = Confirm::new(prompt_msg)
        .with_default(true)
        .prompt()?;

    if !should_install {
        anyhow::bail!("Cannot start daemon without `santity-core` binary installed. Please run `cargo install santity-core` manually.");
    }

    install_core().await?;

    find_core_binary().ok_or_else(|| {
        anyhow::anyhow!("`santity-core` installation finished, but binary was not found in PATH or ~/.santity/bin/")
    })
}

pub async fn install_core() -> Result<()> {
    println!("› Installing `santity-core` via cargo...");

    let santity_home = dirs::home_dir()
        .context("Could not determine home directory")?
        .join(".santity");

    let status = Command::new("cargo")
        .args(["install", "santity-core", "--root", santity_home.to_str().unwrap()])
        .status()
        .context("Failed to execute `cargo install santity-core`")?;

    if status.success() {
        println!("[OK] Successfully installed `santity-core` to {:?}", santity_home.join("bin"));
        Ok(())
    } else {
        anyhow::bail!("`cargo install santity-core` exited with non-zero status.");
    }
}

pub async fn status() -> Result<()> {
    println!("// Checking Santity Core Installation Status...");

    if let Some(path) = find_core_binary() {
        println!("  • Binary Location: {:?}", path);

        let output = Command::new(&path)
            .arg("--version")
            .output();

        if let Ok(out) = output {
            let ver = String::from_utf8_lossy(&out.stdout);
            println!("  • Binary Version:  {}", ver.trim());
        } else {
            println!("  • Binary Version:  Unknown");
        }
    } else {
        println!("  • Binary Location: NOT FOUND in PATH or ~/.santity/bin/");
        println!("  • Run `santity core install` or `cargo install santity-core` to install it.");
    }

    let socket_path = crate::commands::default_socket_path();
    if socket_path.exists() {
        println!("  • IPC Socket:     [ACTIVE] {:?}", socket_path);
    } else {
        println!("  • IPC Socket:     [INACTIVE] ({:?} not found)", socket_path);
    }

    Ok(())
}
