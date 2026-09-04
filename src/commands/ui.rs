use crate::commands::plugin;
use crate::tui::app::{Action, App, AppMode, IpcMessage, LogItem};
use crate::tui::ui::render;
use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::net::UnixStream;
use tokio::sync::mpsc;
use tui_input::backend::crossterm::EventHandler;

pub async fn execute(socket_path: &str) -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new(socket_path.to_string());

    // Channels for IPC messages from socket reader and background Action worker
    let (ipc_tx, mut ipc_rx) = mpsc::channel::<IpcMessage>(100);
    let (action_tx, mut action_rx) = mpsc::channel::<Action>(32);
    let (notification_tx, mut notification_rx) = mpsc::channel::<String>(32);

    // Task 1: UDS Socket Reader Task
    let sock_p = socket_path.to_string();
    tokio::spawn(async move {
        let path = Path::new(&sock_p);
        if path.exists() {
            if let Ok(stream) = UnixStream::connect(path).await {
                let reader = BufReader::new(stream);
                let mut lines = reader.lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    if let Ok(msg) = serde_json::from_str::<IpcMessage>(&line) {
                        let _ = ipc_tx.send(msg).await;
                    }
                }
            }
        }
    });

    // Task 2: Non-Blocking Background Action Worker (Executes heavy downloads, atomic swaps, and plugin deletions off the UI thread!)
    let n_tx = notification_tx.clone();
    tokio::spawn(async move {
        while let Some(action) = action_rx.recv().await {
            match action {
                Action::AddPlugin {
                    source,
                    allowed_domains,
                } => {
                    let _ = n_tx.send(format!("Downloading & installing plugin from {}...", source)).await;
                    match plugin::add_with_capabilities(&source, allowed_domains).await {
                        Ok(_) => {
                            let _ = n_tx.send("Plugin installed and hot-loaded successfully!".to_string()).await;
                        }
                        Err(e) => {
                            let _ = n_tx.send(format!("Error adding plugin: {:?}", e)).await;
                        }
                    }
                }
                Action::UnmountPlugin { plugin_name } => {
                    let plugins_dir = dirs::config_dir()
                        .unwrap_or_else(|| PathBuf::from("~/.config"))
                        .join("santity")
                        .join("plugins");

                    let mut removed = false;
                    for ext in ["component.wasm", "wasm"] {
                        let target = plugins_dir.join(format!("{}.{}", plugin_name, ext));
                        if target.exists() {
                            let _ = fs::remove_file(&target);
                            removed = true;
                            break;
                        }
                    }

                    if removed {
                        let _ = n_tx.send(format!("Unmounted and deleted plugin binary '{}'", plugin_name)).await;
                    } else {
                        let _ = n_tx.send(format!("Plugin binary for '{}' not found", plugin_name)).await;
                    }
                }
            }
        }
    });

    let res = run_loop(&mut terminal, &mut app, &mut ipc_rx, &mut notification_rx, action_tx).await;

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    res
}

async fn run_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
    ipc_rx: &mut mpsc::Receiver<IpcMessage>,
    notification_rx: &mut mpsc::Receiver<String>,
    action_tx: mpsc::Sender<Action>,
) -> Result<()> {
    loop {
        // Drain incoming IPC messages
        while let Ok(msg) = ipc_rx.try_recv() {
            app.connected = true;
            match msg {
                IpcMessage::Log {
                    level,
                    target,
                    message,
                    timestamp,
                } => {
                    app.logs.push(LogItem {
                        level,
                        target,
                        message,
                        timestamp,
                    });
                }
                IpcMessage::Status {
                    active_plugins,
                    total_slash_commands,
                    storage_db_path,
                } => {
                    app.active_plugins = active_plugins;
                    app.total_slash_commands = total_slash_commands;
                    app.storage_db_path = storage_db_path;
                }
            }
        }

        // Drain notifications from background worker
        while let Ok(note) = notification_rx.try_recv() {
            app.notification_msg = Some(note.clone());
            app.logs.push(LogItem {
                level: "INFO".to_string(),
                target: "worker".to_string(),
                message: note,
                timestamp: "NOW".to_string(),
            });
        }

        terminal.draw(|f| render(f, app))?;

        if event::poll(Duration::from_millis(30))? {
            if let Event::Key(key) = event::read()? {
                match app.mode {
                    AppMode::Normal => match key.code {
                        KeyCode::Char('q') | KeyCode::Char('Q') => break,
                        KeyCode::Char('c') | KeyCode::Char('C') => {
                            app.logs.clear();
                        }
                        KeyCode::Char('j') | KeyCode::Down => {
                            app.select_next();
                        }
                        KeyCode::Char('k') | KeyCode::Up => {
                            app.select_prev();
                        }
                        KeyCode::Char('a') | KeyCode::Char('A') => {
                            app.mode = AppMode::AddInputModal;
                            app.input.reset();
                        }
                        KeyCode::Char('x') | KeyCode::Delete => {
                            if let Some(plugin_name) = app.selected_plugin_name() {
                                let _ = action_tx.send(Action::UnmountPlugin { plugin_name }).await;
                            }
                        }
                        _ => {}
                    },
                    AppMode::AddInputModal => match key.code {
                        KeyCode::Esc => {
                            app.mode = AppMode::Normal;
                        }
                        KeyCode::Enter => {
                            let source = app.input.value().trim().to_string();
                            if !source.is_empty() {
                                app.pending_source = Some(source);
                                app.mode = AppMode::CapabilityInputModal;
                                app.input.reset();
                            } else {
                                app.mode = AppMode::Normal;
                            }
                        }

                        _ => {
                            app.input.handle_event(&Event::Key(key));
                        }
                    },
                    AppMode::CapabilityInputModal => match key.code {
                        KeyCode::Esc => {
                            app.mode = AppMode::Normal;
                        }
                        KeyCode::Enter => {
                            let domains_str = app.input.value().to_string();
                            let allowed_domains: Vec<String> = domains_str
                                .split(',')
                                .map(|s| s.trim().to_string())
                                .filter(|s| !s.is_empty())
                                .collect();

                            if let Some(source) = app.pending_source.take() {
                                let _ = action_tx
                                    .send(Action::AddPlugin {
                                        source,
                                        allowed_domains,
                                    })
                                    .await;
                            }
                            app.mode = AppMode::Normal;
                        }
                        _ => {
                            app.input.handle_event(&Event::Key(key));
                        }
                    },
                }
            }
        }
    }

    Ok(())
}
