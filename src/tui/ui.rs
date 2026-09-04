use super::app::{App, AppMode};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph},
    Frame,
};

pub fn render(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Min(10),   // Main Split Pane
            Constraint::Length(3), // Footer
        ])
        .split(f.area());

    // 1. Header
    let status_style = if app.connected {
        Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
    };

    let status_text = if app.connected { "CONNECTED (UDS)" } else { "CONNECTING..." };

    let mut header_spans = vec![
        Span::styled("› SANTITY CORE CONTROL DECK ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::raw(" | Engine IPC: "),
        Span::styled(status_text, status_style),
        Span::raw(format!(" | Socket: {}", app.socket_path)),
    ];

    if let Some(ref note) = app.notification_msg {
        header_spans.push(Span::raw(" | "));
        header_spans.push(Span::styled(note, Style::default().fg(Color::LightYellow).add_modifier(Modifier::BOLD)));
    }

    let header = Paragraph::new(Line::from(header_spans))
        .block(Block::default().borders(Borders::ALL).title(" Host Engine Supervisor "));

    f.render_widget(header, chunks[0]);

    // 2. Split Pane Layout
    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(35), // Left: Loaded Plugins & System State
            Constraint::Percentage(65), // Right: Real-time Streaming Logs
        ])
        .split(chunks[1]);

    // Left Pane Split (System Stats top, Plugin List bottom)
    let left_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(6), // System Info
            Constraint::Min(6),    // Stateful Plugin List
        ])
        .split(main_chunks[0]);

    // System Info Box
    let info_lines = vec![
        Line::from(vec![
            Span::styled("● Storage: ", Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD)),
            Span::raw(&app.storage_db_path),
        ]),
        Line::from(vec![
            Span::styled("● Concurrency: ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::raw("100 permits / actor"),
        ]),
        Line::from(vec![
            Span::styled("● Slash Cmds: ", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
            Span::raw(format!("{} active routes", app.total_slash_commands)),
        ]),
    ];

    let info_block = Paragraph::new(info_lines)
        .block(Block::default().borders(Borders::ALL).title(" System State "));
    f.render_widget(info_block, left_chunks[0]);

    // Plugin Selection List Widget (k9s style navigation)
    let plugin_items: Vec<ListItem> = if app.active_plugins.is_empty() {
        vec![ListItem::new(Span::styled("  (No plugins active)", Style::default().fg(Color::DarkGray)))]
    } else {
        app.active_plugins
            .iter()
            .enumerate()
            .map(|(idx, plugin)| {
                let prefix = if idx == app.selected_plugin { " ► " } else { "   " };
                let style = if idx == app.selected_plugin {
                    Style::default().fg(Color::Black).bg(Color::Cyan).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                };

                ListItem::new(Line::from(vec![
                    Span::styled(prefix, Style::default().fg(Color::Cyan)),
                    Span::styled(plugin, style),
                    Span::styled(" [.wasm]", Style::default().fg(Color::DarkGray)),
                ]))
            })
            .collect()
    };

    let mut state = ListState::default();
    if !app.active_plugins.is_empty() {
        state.select(Some(app.selected_plugin));
    }

    let plugins_list = List::new(plugin_items)
        .block(Block::default().borders(Borders::ALL).title(" Active WASM Plugin Actors (j/k) "))
        .highlight_style(Style::default().bg(Color::Cyan).fg(Color::Black).add_modifier(Modifier::BOLD));

    f.render_stateful_widget(plugins_list, left_chunks[1], &mut state);

    // Right Pane: Log Stream Viewer
    let log_items: Vec<ListItem> = app
        .logs
        .iter()
        .map(|item| {
            let level_style = match item.level.as_str() {
                "ERROR" => Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                "WARN" => Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
                _ => Style::default().fg(Color::Green),
            };

            ListItem::new(Line::from(vec![
                Span::styled(format!("[{}] ", item.level), level_style),
                Span::styled(format!("[{}] ", item.target), Style::default().fg(Color::Cyan)),
                Span::styled(format!("({}) ", item.timestamp), Style::default().fg(Color::DarkGray)),
                Span::raw(&item.message),
            ]))
        })
        .collect();

    let logs_list = List::new(log_items)
        .block(Block::default().borders(Borders::ALL).title(" Live IPC Log Stream "));
    f.render_widget(logs_list, main_chunks[1]);

    // 3. Footer
    let footer_spans = match app.mode {
        AppMode::Normal => vec![
            Span::styled("a", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::raw(" Add Plugin  |  "),
            Span::styled("x / Del", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
            Span::raw(" Unmount Selected  |  "),
            Span::styled("j/k", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Span::raw(" Select  |  "),
            Span::styled("c", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::raw(" Clear Logs  |  "),
            Span::styled("q", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::raw(" Quit"),
        ],
        AppMode::AddInputModal | AppMode::CapabilityInputModal => vec![
            Span::styled("Enter", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
            Span::raw(" Confirm  |  "),
            Span::styled("Esc", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
            Span::raw(" Cancel"),
        ],
    };

    let footer = Paragraph::new(Line::from(footer_spans))
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(footer, chunks[2]);

    // 4. Modal Dialog Overlays
    if app.mode == AppMode::AddInputModal {
        let area = centered_rect(60, 20, f.area());
        f.render_widget(Clear, area);

        let input_text = app.input.value();
        let dialog = Paragraph::new(vec![
            Line::from("Enter URL or local file path to .component.wasm:"),
            Line::from(""),
            Line::from(Span::styled(input_text, Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))),
        ])
        .block(Block::default().borders(Borders::ALL).title(" // Add WASM Plugin Component "));

        f.render_widget(dialog, area);
    } else if app.mode == AppMode::CapabilityInputModal {
        let area = centered_rect(65, 25, f.area());
        f.render_widget(Clear, area);

        let input_text = app.input.value();
        let dialog = Paragraph::new(vec![
            Line::from("// Capability Boundary Whitelist Prompt"),
            Line::from("Enter allowed network domains (comma-separated, or leave blank):"),
            Line::from(""),
            Line::from(Span::styled(input_text, Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))),
        ])
        .block(Block::default().borders(Borders::ALL).title(" Egress Capability Grant "));

        f.render_widget(dialog, area);
    }
}

/// Helper function to create a centered Rect for modal overlays
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
