use serde::Deserialize;
use tui_input::Input;

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "kind", content = "payload")]
pub enum IpcMessage {
    Log {
        level: String,
        target: String,
        message: String,
        timestamp: String,
    },
    Status {
        active_plugins: Vec<String>,
        total_slash_commands: usize,
        storage_db_path: String,
    },
}

#[derive(Debug, Clone)]
pub enum Action {
    AddPlugin {
        source: String,
        allowed_domains: Vec<String>,
    },
    UnmountPlugin {
        plugin_name: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppMode {
    Normal,
    AddInputModal,
    CapabilityInputModal,
}

#[derive(Debug, Clone)]
pub struct LogItem {
    pub level: String,
    pub target: String,
    pub message: String,
    pub timestamp: String,
}

pub struct App {
    pub connected: bool,
    pub socket_path: String,
    pub mode: AppMode,
    pub selected_plugin: usize,
    pub active_plugins: Vec<String>,
    pub total_slash_commands: usize,
    pub storage_db_path: String,
    pub logs: Vec<LogItem>,
    pub input: Input,
    pub pending_source: Option<String>,
    pub notification_msg: Option<String>,
}

impl App {
    pub fn new(socket_path: String) -> Self {
        Self {
            connected: false,
            socket_path,
            mode: AppMode::Normal,
            selected_plugin: 0,
            active_plugins: vec!["ping_pong".to_string()],
            total_slash_commands: 1,
            storage_db_path: "~/.config/santity/santity.db".to_string(),
            logs: vec![
                LogItem {
                    level: "INFO".to_string(),
                    target: "santity".to_string(),
                    message: "Santity Core Control Deck initialized.".to_string(),
                    timestamp: "NOW".to_string(),
                },
            ],
            input: Input::default(),
            pending_source: None,
            notification_msg: None,
        }
    }

    pub fn select_next(&mut self) {
        if !self.active_plugins.is_empty() {
            self.selected_plugin = (self.selected_plugin + 1) % self.active_plugins.len();
        }
    }

    pub fn select_prev(&mut self) {
        if !self.active_plugins.is_empty() {
            if self.selected_plugin == 0 {
                self.selected_plugin = self.active_plugins.len() - 1;
            } else {
                self.selected_plugin -= 1;
            }
        }
    }

    pub fn selected_plugin_name(&self) -> Option<String> {
        self.active_plugins.get(self.selected_plugin).cloned()
    }
}
