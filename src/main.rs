mod commands;
mod tui;

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "santity", author, version, about = "Native control plane, package manager, and Ratatui TUI dashboard for Santity Wasm runtime", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Manage the Santity Core runtime engine binary
    Core {
        #[command(subcommand)]
        action: CoreCommands,
    },
    /// Boot Santity Core engine background daemon (triggers interactive setup wizard if unconfigured)
    Up {
        /// Force re-running interactive config setup wizard
        #[arg(short, long)]
        configure: bool,
    },
    /// Stop the running Santity Core daemon process
    Down,
    /// Plugin package management (add, list, remove)
    Plugin {
        #[command(subcommand)]
        action: PluginCommands,
    },
    /// Launch interactive split-pane Ratatui TUI dashboard
    Ui {
        /// Path to Unix Domain Socket
        #[arg(short, long)]
        socket: Option<String>,
    },
    /// Scaffold a new Santity WASM plugin project from template
    New {
        /// Name of the new plugin project directory
        name: String,

        /// Path to local santity-pdk crate directory for local dev
        #[arg(long)]
        local_pdk: Option<String>,
    },
    /// Build guest Rust plugin into a WebAssembly Component Model binary
    Build {
        /// Build in release mode
        #[arg(short, long)]
        release: bool,
    },
}

#[derive(Subcommand)]
pub enum CoreCommands {
    /// Install santity-core runtime binary via cargo
    Install,
    /// Display santity-core binary location, version, and IPC socket status
    Status,
}

#[derive(Subcommand)]
pub enum PluginCommands {
    /// Download/install a .component.wasm plugin with interactive capability domain whitelisting
    Add {
        /// Local file path or HTTP URL to .component.wasm binary
        source: String,
    },
    /// List installed plugins in config.toml
    List,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Core { action } => match action {
            CoreCommands::Install => commands::core::install_core().await?,
            CoreCommands::Status => commands::core::status().await?,
        },
        Commands::Up { configure } => commands::up::execute(configure).await?,
        Commands::Down => commands::down::execute().await?,
        Commands::Plugin { action } => match action {
            PluginCommands::Add { source } => commands::plugin::add(&source).await?,
            PluginCommands::List => commands::plugin::list().await?,
        },
        Commands::Ui { socket } => {
            let socket_path = socket.unwrap_or_else(|| {
                commands::default_socket_path()
                    .to_string_lossy()
                    .to_string()
            });
            commands::ui::execute(&socket_path).await?
        }
        Commands::New { name, local_pdk } => commands::new::execute(&name, local_pdk.as_deref()).await?,
        Commands::Build { release } => commands::build::execute(release).await?,
    }

    Ok(())
}

