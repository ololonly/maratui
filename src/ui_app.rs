use crate::telemetry::{AppEvent, Snapshot};
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    style::{Style, Stylize},
    symbols,
    widgets::{Block, Padding, Paragraph, Tabs, Widget},
};
use std::time::Instant;
use strum::IntoEnumIterator;
use strum_macros::{Display, EnumIter, FromRepr};

pub struct UiApp {
    pub state: AppState,
}

#[derive(Default)]
pub struct AppState {
    pub screen: Screen,
    pub snapshot: Option<Snapshot>,
    pub cups: Option<u32>,
    pub recent_events: Vec<(Instant, AppEvent)>,
    pub show_debug: bool,
}

#[derive(Default, Clone, Copy, Display, FromRepr, EnumIter, PartialEq)]
pub enum Screen {
    #[default]
    #[strum(to_string = "Main")]
    Main,
    #[strum(to_string = "Dashboard")]
    Dashboard,
    #[strum(to_string = "Graphs")]
    Graphs,
    #[strum(to_string = "Debug")]
    Debug,
}

impl UiApp {
    pub fn new() -> Self {
        Self {
            state: AppState::default(),
        }
    }

    pub fn next_tab(&mut self) {
        self.state.screen = self.state.screen.next();
    }

    pub fn previous_tab(&mut self) {
        self.state.screen = self.state.screen.previous();
    }

    pub fn update_telemetry(&mut self, snapshot: Option<Snapshot>) {
        self.state.snapshot = snapshot;
    }

    pub fn update_cups(&mut self, cups: Option<u32>) {
        self.state.cups = cups;
    }

    pub fn add_event(&mut self, event: AppEvent, timestamp: Instant) {
        self.state.recent_events.push((timestamp, event));
        // Keep only last 10 events
        if self.state.recent_events.len() > 10 {
            self.state.recent_events.remove(0);
        }
    }

    pub fn toggle_debug(&mut self) {
        self.state.show_debug = !self.state.show_debug;
    }

    fn render_tabs(&self, area: Rect, buf: &mut Buffer) {
        let titles: Vec<&str> = Screen::iter().map(|s| s.as_str()).collect();
        let selected_tab_index = self.state.screen as usize;
        Tabs::new(titles)
            .style(Style::default().cyan().underlined())
            .highlight_style(Style::default().yellow())
            .select(selected_tab_index)
            .divider(symbols::DOT)
            .padding(" ", " ")
            .render(area, buf);
    }
}

impl Widget for &UiApp {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let layout = Layout::default()
            .direction(ratatui::layout::Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Percentage(100)])
            .margin(0)
            .split(area);

        // Left area: pixel rat is drawn via embedded-graphics in flush_callback
        // Just leave it empty or show a subtle indicator
        let tabs_area = layout[0];
        // Paragraph::new("")
        //     .block(Block::bordered().title(" 🐀 Chef Rat "))
        //     .render(rat_area, buf);

        // Render status text on the right
        let content_area = layout[1];

        self.render_tabs(tabs_area, buf);
        // Render content with border first (on full area)
        // This draws the complete border frame
        self.state.screen.render_tab_content(
            content_area,
            buf,
            self.state.snapshot.as_ref(),
            self.state.cups,
            self.state.show_debug,
            &self.state.recent_events,
        );
    }
}

impl Screen {
    fn previous(self) -> Self {
        let current_index: usize = self as usize;
        let total = Screen::iter().count();
        let previous_index = if current_index == 0 {
            total - 1 // Wrap to last
        } else {
            current_index - 1
        };
        Self::from_repr(previous_index).unwrap_or(self)
    }

    fn next(self) -> Self {
        let current_index = self as usize;
        let total = Screen::iter().count();
        let next_index = (current_index + 1) % total; // Wrap to 0 after last
        Self::from_repr(next_index).unwrap_or(self)
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Main => "Main",
            Self::Dashboard => "Dashboard",
            Self::Graphs => "Graphs",
            Self::Debug => "Debug",
        }
    }

    fn render_tab_content(
        self,
        area: Rect,
        buf: &mut Buffer,
        snapshot: Option<&Snapshot>,
        cups: Option<u32>,
        _show_debug: bool,
        _recent_events: &[(Instant, AppEvent)],
    ) {
        match self {
            Self::Main => {
                self.render_main_screen(area, buf, snapshot, cups);
            }
            Self::Dashboard => {
                Paragraph::new("Dashboard - coming soon")
                    .block(self.block())
                    .render(area, buf);
            }
            Self::Graphs => {
                Paragraph::new("Heat charts - coming soon")
                    .block(self.block())
                    .render(area, buf);
            }
            Self::Debug => {
                Paragraph::new("Debug screen - coming soon")
                    .block(self.block())
                    .render(area, buf);
            }
        }
    }

    /// A block surrounding the tab's content
    fn block(self) -> Block<'static> {
        Block::bordered()
    }

    fn render_main_screen(
        self,
        area: Rect,
        buf: &mut Buffer,
        snapshot: Option<&Snapshot>,
        cups: Option<u32>,
    ) {
        // Layout: pixel rat on left (drawn via embedded-graphics), text info on right
        // No inner margins - border should pass through tabs
        let layout = Layout::default()
            .direction(ratatui::layout::Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(area);

        // Left area: pixel rat is drawn via embedded-graphics in flush_callback
        // Just leave it empty or show a subtle indicator
        let _rat_area = layout[0];
        // Paragraph::new("")
        //     .block(Block::bordered().title(" 🐀 Chef Rat "))
        //     .render(rat_area, buf);

        // Render status text on the right
        let status_area = layout[1];
        self.render_main_status(status_area, buf, snapshot, cups);
    }

    fn render_main_status(
        self,
        area: Rect,
        buf: &mut Buffer,
        snapshot: Option<&Snapshot>,
        cups: Option<u32>,
    ) {
        let mut lines = Vec::new();

        // Status text - split by newlines to create multiple lines
        let status = if let Some(s) = snapshot {
            if matches!(s.frame.mode, crate::telemetry::MachineMode::Offline) {
                "Loading system...\nConnecting to HA..."
            } else {
                "System ready\nfor coffee."
            }
        } else {
            "Waiting for\ntelemetry..."
        };

        // Split by newlines and create a Line for each part
        for line_text in status.split('\n') {
            lines.push(ratatui::text::Line::from(line_text));
        }

        // Mode and cups
        if let Some(s) = snapshot {
            let mode_str = self.mode_label(Some(s.frame.mode));
            lines.push(ratatui::text::Line::from(""));
            lines.push(ratatui::text::Line::from(format!("Mode: {mode_str}")));
        }

        if let Some(c) = cups {
            lines.push(ratatui::text::Line::from(format!("Cups: {c}")));
        }

        Paragraph::new(lines)
            .block(Block::default().padding(Padding::new(0, 0, 1, 0)))
            .render(area, buf);
    }

    fn mode_label(self, mode: Option<crate::telemetry::MachineMode>) -> String {
        match mode {
            Some(crate::telemetry::MachineMode::Coffee) => "COFFEE".to_string(),
            Some(crate::telemetry::MachineMode::SteamS) => "STEAM(S)".to_string(),
            Some(crate::telemetry::MachineMode::SteamV) => "STEAM(V)".to_string(),
            Some(crate::telemetry::MachineMode::SteamC) => "STEAM(C)".to_string(),
            Some(crate::telemetry::MachineMode::Offline) => "OFFLINE".to_string(),
            Some(crate::telemetry::MachineMode::Unknown(c)) => format!("UNK({c})"),
            None => "--".to_string(),
        }
    }
}
