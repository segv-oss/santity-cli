use anyhow::{Context, Result};
use inquire::{MultiSelect, Password, Text};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Baseline unprivileged intents: GUILDS | GUILD_MESSAGES
const BASELINE_INTENTS: u64 = (1 << 0) | (1 << 9);

/// Privileged gateway intents surfaced as wizard opt-ins.
/// Each requires manual activation in the Discord Developer Portal
/// (Bot -> Privileged Gateway Intents) or the gateway IDENTIFY will fail.
const PRIVILEGED_INTENT_CHOICES: [(&str, u64); 3] = [
    (
        "Server Members — member join/leave/update events (moderation, welcome, role plugins)",
        1 << 1,
    ),
    (
        "Presence — user status & activity tracking (vanity role, leveling plugins)",
        1 << 8,
    ),
    (
        "Message Content — read full message text (auto-responder, anti-spam, snipe plugins)",
        1 << 15,
    ),
];

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

fn get_launchd_plist_path() -> Option<PathBuf> {
    dirs::home_dir().map(|home| home.join("Library/LaunchAgents/com.santity.core.plist"))
}

pub async fn execute(force_configure: bool) -> Result<()> {
    let config_dir = get_config_dir();
    let config_path = get_config_path();

    fs::create_dir_all(&config_dir).context("Failed to create configuration directory")?;

    // Step 1: Interactive Wizard if config is missing or force_configure requested
    if force_configure || !config_path.exists() {
        println!("// Welcome to Santity Core Interactive Setup Wizard");
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

        // Gateway intent selection: baseline always on; privileged intents are
        // explicit opt-ins because they must also be enabled in the Discord
        // Developer Portal and are gated by Discord verification requirements.
        println!("[INFO] Privileged intents you enable here must ALSO be toggled in the");
        println!("       Discord Developer Portal -> your app -> Bot -> Privileged Gateway Intents.\n");
        let intent_labels: Vec<&str> = PRIVILEGED_INTENT_CHOICES.iter().map(|(l, _)| *l).collect();
        let selected = MultiSelect::new("Enable privileged gateway intents?", intent_labels)
            .with_help_message("Plugins declare required events themselves; core aggregates them automatically. Only enable what your plugins need.")
            .with_default(&[2])
            .prompt()?;
        let mut intents = BASELINE_INTENTS;
        for label in selected {
            if let Some((_, mask)) = PRIVILEGED_INTENT_CHOICES.iter().find(|(l, _)| *l == label) {
                intents |= mask;
            }
        }

        let mut toml_content = format!(
            "[bot]\ntoken = \"{}\"\nintents = {}\n",
            token.trim(),
            intents
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
        println!("[OK] Configuration file saved at {:?}\n", config_path);
    }

    let plugins_dir = config_dir.join("plugins");
    fs::create_dir_all(&plugins_dir)?;

    // Ensure santity-core engine binary is installed and located
    let core_bin = crate::commands::core::ensure_core_installed().await?;

    // Step 2: Native daemon supervision (launchd on macOS, systemd user service on Linux)
    let supervised = if cfg!(target_os = "macos") {
        start_launchd_agent(&core_bin, &config_path, &plugins_dir)?
    } else {
        start_systemd_service(&core_bin, &config_path, &plugins_dir)?
    };

    if supervised {
        println!("  • Config:  {:?}", config_path);
        println!("  • Plugins: {:?}", plugins_dir);
        println!("  • Socket:  {:?}\n", crate::commands::default_socket_path());
        println!("Run `santity ui` to open the real-time Ratatui dashboard.");
        return Ok(());
    }

    // Fallback: spawn detached background process if no native supervisor is available
    println!("› Spawning Santity Core daemon as a background process...");
    let child = Command::new(&core_bin)
        .env("SANTITY_CONFIG", &config_path)
        .env("SANTITY_PLUGINS_DIR", &plugins_dir)
        .spawn()
        .context("Failed to spawn santity-core engine process")?;

    let pid_path = config_dir.join("santity.pid");
    fs::write(&pid_path, child.id().to_string())?;

    println!("[OK] Santity Core engine running in background (PID {})", child.id());
    println!("Run `santity ui` to open the real-time Ratatui dashboard.");

    Ok(())
}

/// Install and bootstrap a macOS launchd agent (KeepAlive + RunAtLoad) supervising santity-core.
fn start_launchd_agent(core_bin: &Path, config_path: &Path, plugins_dir: &Path) -> Result<bool> {
    let Some(plist_path) = get_launchd_plist_path() else {
        return Ok(false);
    };

    if let Some(parent) = plist_path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    let uid = unsafe { libc::getuid() };
    let stdout_log = dirs::home_dir()
        .map(|h| h.join("Library/Logs/santity-core.out.log"))
        .unwrap_or_else(|| PathBuf::from("/tmp/santity-core.out.log"));
    let stderr_log = dirs::home_dir()
        .map(|h| h.join("Library/Logs/santity-core.err.log"))
        .unwrap_or_else(|| PathBuf::from("/tmp/santity-core.err.log"));

    let plist_content = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.santity.core</string>

    <key>ProgramArguments</key>
    <array>
        <string>{}</string>
    </array>

    <key>EnvironmentVariables</key>
    <dict>
        <key>SANTITY_CONFIG</key>
        <string>{}</string>
        <key>SANTITY_PLUGINS_DIR</key>
        <string>{}</string>
    </dict>

    <key>RunAtLoad</key>
    <true/>

    <key>KeepAlive</key>
    <true/>

    <key>StandardOutPath</key>
    <string>{}</string>

    <key>StandardErrorPath</key>
    <string>{}</string>
</dict>
</plist>
"#,
        core_bin.display(),
        config_path.display(),
        plugins_dir.display(),
        stdout_log.display(),
        stderr_log.display()
    );

    fs::write(&plist_path, plist_content)?;
    println!("› Generated launchd agent at {:?}", plist_path);

    // Replace any previously-loaded instance of the agent (ignore failures: not loaded yet)
    let _ = Command::new("launchctl")
        .args(["bootout", &format!("gui/{uid}/com.santity.core")])
        .status();

    let bootstrapped = Command::new("launchctl")
        .args([
            "bootstrap",
            &format!("gui/{uid}"),
            &plist_path.to_string_lossy(),
        ])
        .status();

    if bootstrapped.map(|s| s.success()).unwrap_or(false) {
        println!("[OK] Santity Core daemon activated via launchd agent (auto-restart + login persistence).");
        println!("  • Agent: com.santity.core");
        return Ok(true);
    }

    // Older macOS fallback: legacy load subcommand
    let loaded = Command::new("launchctl").arg("load").arg(&plist_path).status();
    if loaded.map(|s| s.success()).unwrap_or(false) {
        println!("[OK] Santity Core daemon activated via launchd (legacy load).");
        return Ok(true);
    }

    warn_launchd_failure();
    Ok(false)
}

fn warn_launchd_failure() {
    println!("[WARN] Could not register launchd agent; falling back to background process.");
}

/// Generate and activate a systemd user service supervising santity-core (Linux).
fn start_systemd_service(core_bin: &Path, config_path: &Path, plugins_dir: &Path) -> Result<bool> {
    let Some(service_path) = get_systemd_service_path() else {
        return Ok(false);
    };

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
    println!("› Generated systemd user service at {:?}", service_path);

    let reload_res = Command::new("systemctl")
        .args(["--user", "daemon-reload"])
        .status();

    if reload_res.map(|s| s.success()).unwrap_or(false) {
        let start_res = Command::new("systemctl")
            .args(["--user", "enable", "--now", "santity.service"])
            .status();

        if start_res.map(|s| s.success()).unwrap_or(false) {
            println!("[OK] Santity Core daemon activated natively via systemd user service.");
            println!("  • Service: santity.service");
            return Ok(true);
        }
    }

    Ok(false)
}

