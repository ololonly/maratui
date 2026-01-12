// src/ui.rs
use ratatui::{prelude::*, widgets::*};
use crate::telemetry::{AppEvent, MachineMode, Snapshot};
use std::time::Instant;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Dashboard,
    Details,
    History,
}

#[derive(Debug, Clone)]
pub struct UiState {
    pub screen: Screen,
    pub show_debug: bool,
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            screen: Screen::Dashboard,
            show_debug: false,
        }
    }
}

/// Main render function - routes to appropriate screen renderer
pub fn render_app(
    frame: &mut Frame, 
    ui: &UiState, 
    snap: Option<&Snapshot>, 
    cups: Option<u32>,
    recent_events: &[(Instant, AppEvent)],
) {
    match ui.screen {
        Screen::Dashboard => render_dashboard(frame, snap, cups, ui.show_debug, recent_events),
        Screen::Details => render_details(frame, snap, cups, ui.show_debug, recent_events),
        Screen::History => render_history(frame, snap, cups, ui.show_debug),
    }
}

/// Dashboard: main screen with big numbers and status chips
fn render_dashboard(
    frame: &mut Frame, 
    snap: Option<&Snapshot>, 
    cups: Option<u32>,
    show_debug: bool,
    recent_events: &[(Instant, AppEvent)],
) {
    let area = frame.area();

    // Outer border with title showing current screen
    let outer = Block::bordered()
        .title(" MaraTUI [Dashboard] ")
        .title_alignment(Alignment::Center);
    frame.render_widget(outer, area);

    // Inner area (with margin for border)
    let inner = area.inner(Margin {
        vertical: 1,
        horizontal: 1,
    });

    // Split into rows: temps, status chips, bottom bar
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(7), // Big temperature display
            Constraint::Length(5), // Status chips
            Constraint::Min(0),   // Spacer / debug info
        ])
        .split(inner);

    render_big_temps(frame, rows[0], snap);
    render_status_chips(frame, rows[1], snap);
    
    // Bottom bar with info and optional debug events
    render_bottom_bar(frame, rows[2], snap, cups, show_debug, recent_events);
}

/// Render large temperature displays (Boiler and HX)
fn render_big_temps(frame: &mut Frame, area: Rect, snap: Option<&Snapshot>) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(area);

    // Extract temperature data from snapshot
    let (boiler_now, boiler_target, hx_now) = if let Some(s) = snap {
        (
            Some(s.frame.boiler_now_c),
            s.frame.boiler_target_c,
            Some(s.frame.hx_now_c),
        )
    } else {
        (None, None, None)
    };

    // Left: Boiler temperature (big) + target
    let left_text = match (boiler_now, boiler_target) {
        (Some(now), Some(target)) => format!("{now:>3}°C\nTarget {target}"),
        (Some(now), None) => format!("{now:>3}°C\nTarget --"),
        _ => "--°C\nTarget --".to_string(),
    };

    let left_block = Paragraph::new(left_text)
        .alignment(Alignment::Center)
        .block(Block::bordered().title("Boiler"));
    frame.render_widget(left_block, cols[0]);

    // Right: Heat Exchanger temperature
    let right_text = match hx_now {
        Some(hx) => format!("HX\n{hx:>3}°C"),
        None => "HX\n--°C".to_string(),
    };

    let right_block = Paragraph::new(right_text)
        .alignment(Alignment::Center)
        .block(Block::bordered().title("Heat Exch"));
    frame.render_widget(right_block, cols[1]);
}

/// Render status chips: MODE, HEAT, PUMP, WATER
fn render_status_chips(frame: &mut Frame, area: Rect, snap: Option<&Snapshot>) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
        ])
        .split(area);

    // Extract status values from snapshot
    let (mode, heating, pump, no_water) = if let Some(s) = snap {
        (
            Some(s.frame.mode),
            Some(s.frame.heating_on),
            Some(s.frame.pump_on),
            s.frame.no_water_code,
        )
    } else {
        (None, None, None, None)
    };

    // Render status chips
    frame.render_widget(status_block("MODE", mode_label(mode)), cols[0]);
    frame.render_widget(bool_chip("HEAT", heating), cols[1]);
    frame.render_widget(bool_chip("PUMP", pump), cols[2]);

    // Water status chip (special handling for refill warning)
    let water_chip = if let Some(code) = no_water {
        Paragraph::new(format!("FILL\nL{code}"))
            .alignment(Alignment::Center)
            .block(Block::bordered().title("WATER"))
            .style(Style::default().fg(Color::Red))
    } else {
        Paragraph::new("OK")
            .alignment(Alignment::Center)
            .block(Block::bordered().title("WATER"))
    };

    frame.render_widget(water_chip, cols[3]);
}

/// Render bottom bar with info and optional debug events
fn render_bottom_bar(
    frame: &mut Frame, 
    area: Rect, 
    snap: Option<&Snapshot>, 
    cups: Option<u32>,
    show_debug: bool,
    recent_events: &[(Instant, AppEvent)],
) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(3)])
        .split(area);

    // Debug events area (if enabled)
    if show_debug && !recent_events.is_empty() {
        let mut event_lines = Vec::new();
        event_lines.push(Line::from("Recent Events:").style(Style::default().add_modifier(Modifier::BOLD)));
        
        // Show last 3 events
        for (_, event) in recent_events.iter().rev().take(3) {
            let event_str = match event {
                AppEvent::ShotStarted => "ShotStarted",
                AppEvent::ShotEnded { duration } => &format!("ShotEnded({:.1}s)", duration.as_secs_f32()),
                AppEvent::WaterRefillNeeded { code } => &format!("WaterRefill(L{code})"),
                AppEvent::WaterRefillCleared => "WaterRefillCleared",
                AppEvent::ModeChanged { from, to } => &format!("Mode: {:?}->{:?}", from, to),
            };
            event_lines.push(Line::from(format!("  • {}", event_str)));
        }
        
        let events_para = Paragraph::new(event_lines)
            .wrap(Wrap { trim: true })
            .style(Style::default().fg(Color::Yellow));
        frame.render_widget(events_para, rows[0]);
    } else {
        // Spacer when debug is off
        frame.render_widget(Paragraph::new(""), rows[0]);
    }

    // Bottom info bar
    let (shot_s, mode, sw) = if let Some(s) = snap {
        (
            Some(s.pump_on_seconds),
            Some(s.frame.mode),
            Some(s.frame.sw_version.as_str()),
        )
    } else {
        (None, None, None)
    };

    let cups_str = cups
        .map(|v| v.to_string())
        .unwrap_or_else(|| "--".to_string());
    let shot_str = shot_s
        .map(|v| format!("{v}s"))
        .unwrap_or_else(|| "--".to_string());
    let mode_str = mode_label(mode);
    let sw_str = sw.unwrap_or("--");

    let text = format!("Cups: {cups_str}  |  Shot: {shot_str}  |  {mode_str}  |  v{sw_str}");
    let bar = Paragraph::new(text)
        .block(Block::bordered())
        .alignment(Alignment::Center);
    frame.render_widget(bar, rows[1]);
}

/// Details screen: shows all telemetry fields and debug info
fn render_details(
    frame: &mut Frame, 
    snap: Option<&Snapshot>, 
    cups: Option<u32>, 
    show_debug: bool,
    recent_events: &[(Instant, AppEvent)],
) {
    let area = frame.area();
    frame.render_widget(
        Block::bordered()
            .title(" Details ")
            .title_alignment(Alignment::Center),
        area,
    );

    let inner = area.inner(Margin {
        vertical: 1,
        horizontal: 1,
    });

    let mut lines: Vec<Line> = Vec::new();

    if let Some(s) = snap {
        // Telemetry fields
        lines.push(kv("Mode", mode_label(Some(s.frame.mode))));
        lines.push(kv("SW", &s.frame.sw_version));
        lines.push(kv("Boiler now", &format!("{} °C", s.frame.boiler_now_c)));
        lines.push(kv(
            "Boiler target",
            &match s.frame.boiler_target_c {
                Some(v) => format!("{v} °C"),
                None => "--".to_string(),
            },
        ));
        lines.push(kv("HX now", &format!("{} °C", s.frame.hx_now_c)));
        lines.push(kv("Boost", &format!("{} s", s.frame.boost_countdown_s)));
        lines.push(kv("Heating", &format!("{}", s.frame.heating_on)));
        lines.push(kv("Pump", &format!("{}", s.frame.pump_on)));
        lines.push(kv("Shot seconds", &format!("{}", s.pump_on_seconds)));

        if let Some(code) = s.frame.no_water_code {
            lines.push(kv("Water", &format!("NEED REFILL (L{code})")));
        } else {
            lines.push(kv("Water", "OK"));
        }

        lines.push(kv(
            "Cups (HA)",
            &cups
                .map(|v| v.to_string())
                .unwrap_or_else(|| "--".to_string()),
        ));

        // Debug section
        if show_debug {
            lines.push(Line::from(""));
            lines.push(Line::from("Debug:").style(Style::default().add_modifier(Modifier::BOLD)));
            lines.push(kv("Extracting", &format!("{}", s.is_extracting)));
            
            // Show recent events
            if !recent_events.is_empty() {
                lines.push(Line::from(""));
                lines.push(Line::from("Events:").style(Style::default().add_modifier(Modifier::BOLD)));
                for (_, event) in recent_events.iter().rev().take(5) {
                    let event_str = match event {
                        AppEvent::ShotStarted => "ShotStarted".to_string(),
                        AppEvent::ShotEnded { duration } => format!("ShotEnded({:.1}s)", duration.as_secs_f32()),
                        AppEvent::WaterRefillNeeded { code } => format!("WaterRefill(L{code})"),
                        AppEvent::WaterRefillCleared => "WaterRefillCleared".to_string(),
                        AppEvent::ModeChanged { from, to } => format!("Mode: {:?}->{:?}", from, to),
                    };
                    lines.push(kv("", &event_str));
                }
            }
        }
    } else {
        lines.push(Line::from("No telemetry yet..."));
    }

    let p = Paragraph::new(lines)
        .wrap(Wrap { trim: true })
        .block(Block::bordered().title("Fields"));
    frame.render_widget(p, inner);
}

/// History screen: placeholder for future chart
fn render_history(
    frame: &mut Frame, 
    snap: Option<&Snapshot>, 
    cups: Option<u32>,
    show_debug: bool,
) {
    let area = frame.area();
    frame.render_widget(
        Block::bordered()
            .title(" History ")
            .title_alignment(Alignment::Center),
        area,
    );

    let inner = area.inner(Margin {
        vertical: 1,
        horizontal: 1,
    });

    let mode = snap
        .map(|s| mode_label(Some(s.frame.mode)))
        .unwrap_or_else(|| "--".to_string());
    let cups_str = cups
        .map(|v| v.to_string())
        .unwrap_or_else(|| "--".to_string());

    let mut text = format!(
        "Coming soon: mini chart.\n\nMode: {mode}\nCups: {cups_str}\n\nTip: we'll render a sparkline from last N samples."
    );
    
    if show_debug {
        text.push_str("\n\n[Debug mode ON]");
    }

    frame.render_widget(Paragraph::new(text).wrap(Wrap { trim: true }), inner);
}

/// Helper: create a key-value line for details screen
fn kv(key: &str, value: impl Into<String>) -> Line<'static> {
    Line::from(vec![
        Span::styled(
            format!("{key:<14} "),
            Style::default().add_modifier(Modifier::BOLD),
        ),
        Span::raw(value.into()),
    ])
}

/// Helper: create a status block widget
fn status_block(title: &str, text: String) -> Paragraph<'_> {
    Paragraph::new(text)
        .alignment(Alignment::Center)
        .block(Block::bordered().title(title))
}

/// Helper: create a boolean chip widget
fn bool_chip(title: &str, value: Option<bool>) -> Paragraph<'_> {
    let text = match value {
        Some(true) => "ON",
        Some(false) => "OFF",
        None => "--",
    };
    Paragraph::new(text)
        .alignment(Alignment::Center)
        .block(Block::bordered().title(title))
}

/// Helper: convert machine mode to display string
fn mode_label(mode: Option<MachineMode>) -> String {
    match mode {
        Some(MachineMode::Coffee) => "COFFEE".to_string(),
        Some(MachineMode::SteamS) => "STEAM(S)".to_string(),
        Some(MachineMode::SteamV) => "STEAM(V)".to_string(),
        Some(MachineMode::SteamC) => "STEAM(C)".to_string(),
        Some(MachineMode::Offline) => "OFFLINE".to_string(),
        Some(MachineMode::Unknown(c)) => format!("UNK({c})"),
        None => "--".to_string(),
    }
}
