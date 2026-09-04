pub mod build;
pub mod core;
pub mod down;
pub mod new;
pub mod plugin;
pub mod ui;
pub mod up;

use std::path::PathBuf;

pub const DEFAULT_SOCKET_PATH: &str = "/tmp/santity.sock";

/// Returns the secure default Unix domain socket path:
/// 1. `SANTITY_SOCKET_PATH` env var if set
/// 2. `$XDG_RUNTIME_DIR/santity.sock` (e.g. `/run/user/<UID>/santity.sock`)
/// 3. `$XDG_CONFIG_HOME/santity/santity.sock` (e.g. `~/.config/santity/santity.sock`)
/// 4. `/tmp/santity.sock` fallback
pub fn default_socket_path() -> PathBuf {
    if let Ok(env_path) = std::env::var("SANTITY_SOCKET_PATH") {
        if !env_path.trim().is_empty() {
            return PathBuf::from(env_path);
        }
    }

    if let Some(runtime_dir) = dirs::runtime_dir() {
        return runtime_dir.join("santity.sock");
    }

    if let Some(config_dir) = dirs::config_dir() {
        return config_dir.join("santity").join("santity.sock");
    }

    PathBuf::from(DEFAULT_SOCKET_PATH)
}

