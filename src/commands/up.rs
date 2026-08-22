use anyhow::{Context, Result};
use inquire::{Password, Text};
use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn get_config_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("~/.config"))
        .join("santity")
}

fn get_config_path() -> PathBuf {
    get_config_dir().join("config.toml")
}

fn get_systemd_service_path() -> Option<PathBuf> {
    dirs::config_dir().map(|dir| dir.join("systemd").join("user").join("santity.service"))
}

pub async fn execute(force_configure: bool) -> Result<()> {
    let config_dir = get_config_dir();
    let config_path = get_config_path();

    fs::create_dir_all(&config_dir).context("Failed to create configuration directory")?;

    // Step 1: Interactive Wizard if config is missing or force_configure requested
    if force_configure || !config_path.exists() {
        println!("🚀 Welcome to Santity Core Interactive Setup Wizard!");
        println!("Configuring runtime environment at {:?}\n", config_path);

        let token = Password::new("Enter your Discord Bot Token:")
            .with_display_mode(inquire::PasswordDisplayMode::Masked)
            .prompt()?;

        let app_id = Text::new("Enter your Discord Application ID (Optional):")
            .with_placeholder("e.g. 1483822128296890520")
            .prompt_skippable()?;

        let dev_guild = Text::new("Enter your Dev Guild ID for instant Slash Command sync (Optional):")
            .with_placeholder("e.g. 1395086150493802656")
            .prompt_skippable()?;

        let db_path = config_dir.join("santity.db").to_string_lossy().to_string();

        let mut toml_content = format!(
            "[bot]\ntoken = \"{}\"\nintents = 33281\n",
            token.trim()
        );

        if let Some(id) = app_id {
            if !id.trim().is_empty() {
                toml_content.push_str(&format!("application_id = \"{}\"\n", id.trim()));
            }
        }
        if let Some(guild) = dev_guild {
            if !guild.trim().is_empty() {
                toml_content.push_str(&format!("dev_guild_id = \"{}\"\n", guild.trim()));
            }
        }

        toml_content.push_str(&format!(
            "\n[storage]\ndb_path = \"{}\"\n\n[[plugins]]\nname = \"ping_pong\"\npath = \"{}\"\nallowed_domains = [\"api.github.com\"]\n",
            db_path,
            config_dir.join("plugins").join("ping_pong_plugin.component.wasm").display()
        ));

        fs::write(&config_path, toml_content)?;
        println!("✅ Configuration file saved at {:?}\n", config_path);
    }

    let plugins_dir = config_dir.join("plugins");
    fs::create_dir_all(&plugins_dir)?;

    // Ensure santity-core engine binary is installed and located
    let core_bin = crate::commands::core::ensure_core_installed().await?;

    // Step 2: Systemd User Service Generation & Activation
    if let Some(service_path) = get_systemd_service_path() {
        if let Some(parent) = service_path.parent() {
            let _ = fs::create_dir_all(parent);
        }

        let service_content = format!(
            r#"[Unit]
Description=Santity Discord Wasm Host Engine Daemon
After=network.target

[Service]
Type=simple
ExecStart={}
Environment=SANTITY_CONFIG={}
Environment=SANTITY_PLUGINS_DIR={}
Restart=on-failure
RestartSec=3s

[Install]
WantedBy=default.target
"#,
            core_bin.display(),
            config_path.display(),
            plugins_dir.display()
        );

        fs::write(&service_path, service_content)?;
        println!("⚙️  Generated systemd user service at {:?}", service_path);

        // Try activating via systemctl
        let reload_res = Command::new("systemctl")
            .args(["--user", "daemon-reload"])
            .status();

        if reload_res.map(|s| s.success()).unwrap_or(false) {
            let start_res = Command::new("systemctl")
                .args(["--user", "enable", "--now", "santity.service"])
                .status();

            if start_res.map(|s| s.success()).unwrap_or(false) {
                println!("🎉 Santity Core daemon activated natively via systemd user service!");
                println!("  • Service: santity.service");
                println!("  • Config:  {:?}", config_path);
                println!("  • Plugins: {:?}", plugins_dir);
                println!("  • Socket:  /tmp/santity.sock\n");
                println!("Run `santity ui` to open the real-time Ratatui dashboard!");
                return Ok(());
            }
        }
    }

    // Fallback: spawn detached background process if systemd systemctl fails
    println!("⚡ Spawning Santity Core daemon as a background process...");
    let child = Command::new(&core_bin)
        .env("SANTITY_CONFIG", &config_path)
        .env("SANTITY_PLUGINS_DIR", &plugins_dir)
        .spawn()
        .context("Failed to spawn santity-core engine process")?;

    let pid_path = config_dir.join("santity.pid");
    fs::write(&pid_path, child.id().to_string())?;

    println!("🎉 Santity Core engine running in background (PID {})", child.id());
    println!("Run `santity ui` to open the real-time Ratatui dashboard!");

    Ok(())
}

