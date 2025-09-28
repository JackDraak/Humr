use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{
        Block, Borders, Clear, Gauge, List, ListItem, ListState, Paragraph, Wrap,
    },
    Frame, Terminal,
};
use std::io;
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub struct DiscoveredPeer {
    pub name: String,
    pub connection_type: String,
    pub quality: u8, // 0-100
    pub latency_ms: u32,
    pub is_secure: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AppMode {
    MainMenu,
    HostMode,
    JoinMode,
    Connected,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ConnectionStatus {
    Disconnected,
    Discovering,
    Connecting,
    Connected,
    Failed(String),
}

pub struct TerminalApp {
    pub mode: AppMode,
    pub connection_status: ConnectionStatus,
    pub discovered_peers: Vec<DiscoveredPeer>,
    pub selected_peer: usize,
    pub room_code: Option<String>,
    pub qr_code: Option<String>,
    pub audio_level_in: f32,
    pub audio_level_out: f32,
    pub connection_quality: u8,
    pub connection_latency: u32,
    pub is_muted: bool,
    pub show_help: bool,
    pub last_update: Instant,
    peer_list_state: ListState,
}

impl Default for TerminalApp {
    fn default() -> Self {
        Self {
            mode: AppMode::MainMenu,
            connection_status: ConnectionStatus::Disconnected,
            discovered_peers: Vec::new(),
            selected_peer: 0,
            room_code: None,
            qr_code: None,
            audio_level_in: 0.0,
            audio_level_out: 0.0,
            connection_quality: 0,
            connection_latency: 0,
            is_muted: false,
            show_help: false,
            last_update: Instant::now(),
            peer_list_state: ListState::default(),
        }
    }
}

impl TerminalApp {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn generate_room_code(&mut self) {
        use uuid::Uuid;
        let id = Uuid::new_v4();
        self.room_code = Some(format!("humr-{}", &id.to_string()[..8]));
    }

    pub fn add_discovered_peer(&mut self, peer: DiscoveredPeer) {
        self.discovered_peers.push(peer);
    }

    pub fn start_hosting(&mut self) {
        self.mode = AppMode::HostMode;
        self.generate_room_code();
        self.qr_code = Some(format!("https://humr.chat/{}", self.room_code.as_ref().unwrap()));
    }

    pub fn start_joining(&mut self) {
        self.mode = AppMode::JoinMode;
        self.connection_status = ConnectionStatus::Discovering;
        // Simulate discovery
        self.add_discovered_peer(DiscoveredPeer {
            name: "Alice's Chat".to_string(),
            connection_type: "Local Network".to_string(),
            quality: 95,
            latency_ms: 2,
            is_secure: true,
        });
        self.add_discovered_peer(DiscoveredPeer {
            name: "Bob's Phone".to_string(),
            connection_type: "Bluetooth".to_string(),
            quality: 78,
            latency_ms: 8,
            is_secure: true,
        });
    }

    pub fn connect_to_peer(&mut self) {
        if self.selected_peer < self.discovered_peers.len() {
            self.connection_status = ConnectionStatus::Connecting;
            // Simulate connection process
        }
    }

    pub fn toggle_help(&mut self) {
        self.show_help = !self.show_help;
    }

    pub fn handle_key_event(&mut self, key: KeyCode) -> bool {
        match key {
            KeyCode::Char('q') | KeyCode::Esc => return false,
            KeyCode::Char('h') | KeyCode::F(1) => self.toggle_help(),
            KeyCode::Char('m') => self.is_muted = !self.is_muted,
            _ => {}
        }

        match self.mode {
            AppMode::MainMenu => match key {
                KeyCode::Char('1') => self.start_hosting(),
                KeyCode::Char('2') => self.start_joining(),
                _ => {}
            },
            AppMode::JoinMode => match key {
                KeyCode::Up => {
                    if self.selected_peer > 0 {
                        self.selected_peer -= 1;
                    }
                }
                KeyCode::Down => {
                    if self.selected_peer < self.discovered_peers.len().saturating_sub(1) {
                        self.selected_peer += 1;
                    }
                }
                KeyCode::Enter => self.connect_to_peer(),
                KeyCode::Backspace => self.mode = AppMode::MainMenu,
                _ => {}
            },
            AppMode::HostMode => match key {
                KeyCode::Backspace => self.mode = AppMode::MainMenu,
                _ => {}
            },
            AppMode::Connected => match key {
                KeyCode::Backspace => {
                    self.mode = AppMode::MainMenu;
                    self.connection_status = ConnectionStatus::Disconnected;
                }
                _ => {}
            },
        }
        true
    }

    pub fn update(&mut self) {
        // Simulate connection progression
        if matches!(self.connection_status, ConnectionStatus::Connecting) {
            if self.last_update.elapsed() > Duration::from_secs(2) {
                self.mode = AppMode::Connected;
                self.connection_status = ConnectionStatus::Connected;
                self.connection_quality = 92;
                self.connection_latency = 15;
            }
        }

        // Simulate audio levels
        if matches!(self.mode, AppMode::Connected) {
            self.audio_level_in = (self.last_update.elapsed().as_millis() as f32 * 0.1).sin().abs() * 0.8;
            self.audio_level_out = (self.last_update.elapsed().as_millis() as f32 * 0.07).sin().abs() * 0.6;
        }
    }
}

pub fn run_terminal_ui() -> Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app state
    let mut app = TerminalApp::new();
    let res = run_app(&mut terminal, &mut app);

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{err:?}");
    }

    Ok(())
}

fn run_app<B: Backend>(terminal: &mut Terminal<B>, app: &mut TerminalApp) -> io::Result<()> {
    loop {
        terminal.draw(|f| ui(f, app))?;

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    if !app.handle_key_event(key.code) {
                        return Ok(());
                    }
                }
            }
        }

        app.update();
    }
}

fn ui(f: &mut Frame, app: &mut TerminalApp) {
    let size = f.size();

    // Main layout
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Header
            Constraint::Min(0),     // Content
            Constraint::Length(3),  // Footer
        ])
        .split(size);

    // Header
    let header = Paragraph::new("🎤 Humr - Revolutionary Voice Communication")
        .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL).style(Style::default().fg(Color::White)));
    f.render_widget(header, chunks[0]);

    // Content based on mode
    match app.mode {
        AppMode::MainMenu => render_main_menu(f, chunks[1]),
        AppMode::HostMode => render_host_mode(f, chunks[1], app),
        AppMode::JoinMode => render_join_mode(f, chunks[1], app),
        AppMode::Connected => render_connected_mode(f, chunks[1], app),
    }

    // Footer with controls
    let footer_text = match app.mode {
        AppMode::MainMenu => "1: Start Voice Chat | 2: Join Voice Chat | q: Quit | h: Help",
        AppMode::HostMode => "Share room code with others | Backspace: Back | q: Quit",
        AppMode::JoinMode => "↑↓: Select | Enter: Connect | Backspace: Back | q: Quit",
        AppMode::Connected => "m: Mute/Unmute | Backspace: Disconnect | q: Quit",
    };

    let footer = Paragraph::new(footer_text)
        .style(Style::default().fg(Color::Green))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(footer, chunks[2]);

    // Help overlay
    if app.show_help {
        render_help_overlay(f);
    }
}

fn render_main_menu(f: &mut Frame, area: ratatui::layout::Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(100)])
        .margin(5)
        .split(area);

    let welcome_text = Text::from(vec![
        Line::from(""),
        Line::from("Welcome to Humr! Choose an option:"),
        Line::from(""),
        Line::from(Span::styled("1. Start Voice Chat", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))),
        Line::from("   • Become discoverable host"),
        Line::from("   • Share connection via QR code or magic link"),
        Line::from("   • Zero-configuration setup"),
        Line::from(""),
        Line::from(Span::styled("2. Join Voice Chat", Style::default().fg(Color::Blue).add_modifier(Modifier::BOLD))),
        Line::from("   • Discover nearby conversations"),
        Line::from("   • Scan QR codes or use magic links"),
        Line::from("   • Auto-connect to best available host"),
        Line::from(""),
        Line::from(Span::styled("Features:", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))),
        Line::from("• End-to-end encrypted voice communication"),
        Line::from("• Real-time noise suppression and echo cancellation"),
        Line::from("• Cross-platform peer-to-peer connections"),
        Line::from("• No servers required - direct connections only"),
    ]);

    let paragraph = Paragraph::new(welcome_text)
        .block(Block::default().title("🎯 Revolutionary Voice Communication").borders(Borders::ALL))
        .wrap(Wrap { trim: true });

    f.render_widget(paragraph, chunks[0]);
}

fn render_host_mode(f: &mut Frame, area: ratatui::layout::Rect, app: &TerminalApp) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    // Room information
    let generating_text = "generating...".to_string();
    let room_code = app.room_code.as_ref().unwrap_or(&generating_text);
    let room_info = Text::from(vec![
        Line::from(""),
        Line::from(Span::styled("🏠 Voice Chat Room Ready!", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))),
        Line::from(""),
        Line::from(vec![
            Span::raw("Room Code: "),
            Span::styled(room_code, Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(""),
        Line::from("Share with others:"),
        Line::from("• Show this screen to scan QR code"),
        Line::from("• Send the room code via any messaging app"),
        Line::from("• Share the magic link: humr://your-room-code"),
        Line::from(""),
        Line::from(Span::styled("🔒 Security:", Style::default().fg(Color::Yellow))),
        Line::from("• End-to-end encrypted communication"),
        Line::from("• Forward secrecy with key rotation"),
        Line::from("• No data stored on external servers"),
    ]);

    let room_block = Paragraph::new(room_info)
        .block(Block::default().title("📡 Broadcasting").borders(Borders::ALL))
        .wrap(Wrap { trim: true });

    f.render_widget(room_block, chunks[0]);

    // QR Code placeholder (in a real implementation, this would be an actual QR code)
    let qr_placeholder = Text::from(vec![
        Line::from(""),
        Line::from("█▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀█"),
        Line::from("█ ▄▄▄▄▄ █▀█ █▄▀▄▄▄█ ▄▄▄▄▄ █"),
        Line::from("█ █   █ █▀▀▀█▄▄▄▄▄█ █   █ █"),
        Line::from("█ █▄▄▄█ ██▄█▀ ▀█▀▄█ █▄▄▄█ █"),
        Line::from("█▄▄▄▄▄▄▄█▄▀ ▀▄█ █▄█▄▄▄▄▄▄▄█"),
        Line::from("█  ▄▄▀▄▄▀▄█▄▄ ▄▀▀▄  ▄▀▀▄▄▄█"),
        Line::from("█▀▀▀▄ ▄▄ ▄ █▄ ▄▄▀▄▄▄ ▄▄ ▀▄█"),
        Line::from("█▀▄ ▄▀▄▄▀▀██▀▄▄▄▀▀▄▄▄▀▀▀▀▄█"),
        Line::from("█▄██▄█▄▄ ▄▄▄▄▄ ▀█▄█ ▄▄▄▄▄ █"),
        Line::from("█ ▄▄▄▄▄ █▄█ ▄▄ ▄██  █   █ █"),
        Line::from("█ █   █ █▀▀▀▀▄ ▄▄▀▄ █▄▄▄█ █"),
        Line::from("█ █▄▄▄█ █▀ ▄▄▄▄▀▄██  ▄▄▄▄▄█"),
        Line::from("█▄▄▄▄▄▄▄█▄█▄██▄█▄██▄█▄▄▄▄▄█"),
        Line::from(""),
        Line::from("    Scan to join voice chat"),
    ]);

    let qr_block = Paragraph::new(qr_placeholder)
        .style(Style::default().fg(Color::White))
        .alignment(Alignment::Center)
        .block(Block::default().title("📱 QR Code").borders(Borders::ALL));

    f.render_widget(qr_block, chunks[1]);
}

fn render_join_mode(f: &mut Frame, area: ratatui::layout::Rect, app: &mut TerminalApp) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(area);

    // Status
    let status_text = match app.connection_status {
        ConnectionStatus::Discovering => "🔍 Discovering voice chats...",
        ConnectionStatus::Connecting => "🔄 Connecting...",
        ConnectionStatus::Failed(ref err) => &format!("❌ Connection failed: {}", err),
        _ => "Ready to connect",
    };

    let status = Paragraph::new(status_text)
        .style(Style::default().fg(Color::Yellow))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(status, chunks[0]);

    // Discovered peers list
    let peer_items: Vec<ListItem> = app
        .discovered_peers
        .iter()
        .enumerate()
        .map(|(i, peer)| {
            let quality_bar = "█".repeat((peer.quality / 10) as usize);
            let security_icon = if peer.is_secure { "🔒" } else { "🔓" };

            let content = Line::from(vec![
                Span::styled(
                    format!("{} {}", security_icon, peer.name),
                    Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
                ),
                Span::raw("  "),
                Span::styled(
                    &peer.connection_type,
                    Style::default().fg(Color::Blue),
                ),
                Span::raw("  "),
                Span::styled(
                    format!("{}ms", peer.latency_ms),
                    Style::default().fg(Color::Green),
                ),
                Span::raw("  "),
                Span::styled(
                    format!("{}%", peer.quality),
                    Style::default().fg(Color::Cyan),
                ),
                Span::raw("  "),
                Span::styled(
                    quality_bar,
                    Style::default().fg(Color::Green),
                ),
            ]);

            let style = if i == app.selected_peer {
                Style::default().bg(Color::Blue).fg(Color::White)
            } else {
                Style::default()
            };

            ListItem::new(content).style(style)
        })
        .collect();

    let peer_list = List::new(peer_items)
        .block(Block::default().title("📡 Available Voice Chats").borders(Borders::ALL))
        .highlight_style(Style::default().bg(Color::Blue));

    f.render_widget(peer_list, chunks[1]);
}

fn render_connected_mode(f: &mut Frame, area: ratatui::layout::Rect, app: &TerminalApp) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(6),  // Connection info
            Constraint::Length(4),  // Audio levels
            Constraint::Min(0),     // Chat area
        ])
        .split(area);

    // Connection info
    let connection_info = Text::from(vec![
        Line::from(vec![
            Span::styled("🟢 Connected", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
            Span::raw(" to "),
            Span::styled("Alice's Chat", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::raw("Quality: "),
            Span::styled(format!("{}%", app.connection_quality), Style::default().fg(Color::Green)),
            Span::raw("  Latency: "),
            Span::styled(format!("{}ms", app.connection_latency), Style::default().fg(Color::Green)),
            Span::raw("  Security: "),
            Span::styled("🔒 Encrypted", Style::default().fg(Color::Green)),
        ]),
        Line::from(""),
        Line::from(if app.is_muted {
            vec![Span::styled("🔇 MUTED", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD))]
        } else {
            vec![Span::styled("🎤 Live", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))]
        }),
    ]);

    let info_block = Paragraph::new(connection_info)
        .block(Block::default().title("🎤 Voice Connection").borders(Borders::ALL));
    f.render_widget(info_block, chunks[0]);

    // Audio levels
    let audio_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(chunks[1]);

    let input_gauge = Gauge::default()
        .block(Block::default().title("🎤 Input").borders(Borders::ALL))
        .gauge_style(Style::default().fg(Color::Green))
        .ratio(app.audio_level_in as f64);
    f.render_widget(input_gauge, audio_chunks[0]);

    let output_gauge = Gauge::default()
        .block(Block::default().title("🔊 Output").borders(Borders::ALL))
        .gauge_style(Style::default().fg(Color::Blue))
        .ratio(app.audio_level_out as f64);
    f.render_widget(output_gauge, audio_chunks[1]);

    // Chat area placeholder
    let chat_text = Text::from(vec![
        Line::from("🎉 Voice chat active!"),
        Line::from(""),
        Line::from("Features active:"),
        Line::from("• Real-time audio processing"),
        Line::from("• Noise suppression"),
        Line::from("• Echo cancellation"),
        Line::from("• End-to-end encryption"),
        Line::from(""),
        Line::from("Audio quality is automatically optimized based on"),
        Line::from("your connection. Press 'm' to mute/unmute."),
    ]);

    let chat_block = Paragraph::new(chat_text)
        .block(Block::default().title("💬 Communication Status").borders(Borders::ALL))
        .wrap(Wrap { trim: true });
    f.render_widget(chat_block, chunks[2]);
}

fn render_help_overlay(f: &mut Frame) {
    let popup_area = centered_rect(80, 80, f.size());
    f.render_widget(Clear, popup_area);

    let help_text = Text::from(vec![
        Line::from(Span::styled("🎤 Humr Help", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))),
        Line::from(""),
        Line::from(Span::styled("Revolutionary P2P Voice Communication", Style::default().fg(Color::Green))),
        Line::from(""),
        Line::from(Span::styled("Main Menu:", Style::default().add_modifier(Modifier::BOLD))),
        Line::from("  1 - Start hosting a voice chat"),
        Line::from("  2 - Join an existing voice chat"),
        Line::from(""),
        Line::from(Span::styled("Host Mode:", Style::default().add_modifier(Modifier::BOLD))),
        Line::from("  • Share room code or QR code with others"),
        Line::from("  • Zero-configuration setup with UPnP"),
        Line::from("  • Local network auto-discovery via mDNS"),
        Line::from(""),
        Line::from(Span::styled("Join Mode:", Style::default().add_modifier(Modifier::BOLD))),
        Line::from("  ↑↓ - Navigate discovered hosts"),
        Line::from("  Enter - Connect to selected host"),
        Line::from("  • Automatic discovery of nearby voice chats"),
        Line::from(""),
        Line::from(Span::styled("Connected Mode:", Style::default().add_modifier(Modifier::BOLD))),
        Line::from("  m - Toggle mute/unmute"),
        Line::from("  • Real-time audio quality monitoring"),
        Line::from("  • Automatic noise suppression"),
        Line::from(""),
        Line::from(Span::styled("Global Controls:", Style::default().add_modifier(Modifier::BOLD))),
        Line::from("  h/F1 - Toggle this help"),
        Line::from("  Backspace - Go back/disconnect"),
        Line::from("  q/Esc - Quit application"),
        Line::from(""),
        Line::from(Span::styled("Security:", Style::default().fg(Color::Yellow))),
        Line::from("• All communication is end-to-end encrypted"),
        Line::from("• Forward secrecy with automatic key rotation"),
        Line::from("• No data stored on external servers"),
        Line::from("• Peer-to-peer connections only"),
    ]);

    let help_block = Paragraph::new(help_text)
        .block(Block::default().title("Help").borders(Borders::ALL).border_style(Style::default().fg(Color::White)))
        .wrap(Wrap { trim: true });

    f.render_widget(help_block, popup_area);
}

fn centered_rect(percent_x: u16, percent_y: u16, r: ratatui::layout::Rect) -> ratatui::layout::Rect {
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