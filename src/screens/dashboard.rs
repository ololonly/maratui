use std::time::Duration;

use ratatui::Frame;
use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Padding, Paragraph, Widget};
use tui_widgets::big_text::{BigText, PixelSize};

use crate::screens::screen::Board;
use crate::state::GlobalAppState;
use crate::telemetry::{MachineMode, TelemetryFrame};

const STYLE_GREEN: Style = Style::new().fg(Color::Green);
const STYLE_CYAN: Style = Style::new().fg(Color::Cyan);
const STYLE_RED: Style = Style::new().fg(Color::Red);
const STYLE_GRAY: Style = Style::new().fg(Color::Gray);
const STYLE_DARK_GRAY: Style = Style::new().fg(Color::DarkGray);
const STYLE_WHITE: Style = Style::new().fg(Color::White);
const STYLE_YELLOW: Style = Style::new().fg(Color::Yellow);

const BOILER_SCALE_MAX: u16 = 160;
const SHOT_GAUGE_MAX_SECS: u64 = 30;
const HX_SCALE_MIN: f64 = 60.0;
const HX_SCALE_MAX: f64 = 110.0;
const HX_IDEAL_LOW: f64 = 90.0;
const HX_IDEAL_HIGH: f64 = 95.0;

#[derive(Default)]
pub struct Dashboard;

impl Board for Dashboard {
    fn render(state: &GlobalAppState, area: Rect, frame: &mut Frame) {
        let buf = frame.buffer_mut();

        let Some(t_frame) = state.machine_state.last_frame.as_ref() else {
            return;
        };

        // banner (1) | main (fill) | boiler (3)
        let [banner_area, content_area, boiler_area] = Layout::vertical([
            Constraint::Length(1),
            Constraint::Fill(1),
            Constraint::Length(3),
        ])
        .areas(area);

        let [mode_area, counter_area] =
            Layout::horizontal(Constraint::from_fills([1, 1])).areas(banner_area);

        render_mode_banner(t_frame, mode_area, buf);
        render_cup_counter(state.cup_counter, counter_area, buf);
        render_boiler_gauge(t_frame, boiler_area, buf);

        // rule of thirds: 1 | 3 | 1
        let [info_col, timer_col, gauge_col] = Layout::horizontal([
            Constraint::Fill(1),
            Constraint::Fill(3),
            Constraint::Fill(1),
        ])
        .areas(content_area);

        render_info_col(t_frame, info_col, buf);
        render_shot_gauge(state, gauge_col, buf);
        render_timer(state, timer_col, frame);
    }
}

fn render_mode_banner(t_frame: &TelemetryFrame, area: Rect, buf: &mut Buffer) {
    let (label, style) = match t_frame.mode {
        MachineMode::Coffee => ("═══ COFFEE MODE ═══", STYLE_GREEN),
        MachineMode::SteamS | MachineMode::SteamV | MachineMode::SteamC => {
            ("═══ STEAM MODE ═══", STYLE_CYAN)
        }
        MachineMode::Offline => ("═══  OFFLINE  ═══", STYLE_RED),
        MachineMode::Unknown(_) => ("═══  UNKNOWN  ═══", STYLE_GRAY),
    };

    Paragraph::new(Line::from(label))
        .centered()
        .style(style)
        .render(area, buf);
}

fn render_cup_counter(count: Option<u64>, area: Rect, buf: &mut Buffer) {
    let Some(cups) = count else {
        return;
    };

    Paragraph::new(Line::from(format!("Cups brewed: {cups}")))
        .centered()
        .style(STYLE_WHITE)
        .render(area, buf);
}

fn render_info_col(t_frame: &TelemetryFrame, area: Rect, buf: &mut Buffer) {
    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(STYLE_YELLOW);
    let inner = block.inner(area);
    block.render(area, buf);

    let [hx_label_area, hx_gauge_area, status_area] = Layout::vertical([
        Constraint::Length(2),
        Constraint::Fill(1),
        Constraint::Length(4),
    ])
    .areas(inner);

    Paragraph::new(vec![
        Line::styled("HX", STYLE_DARK_GRAY),
        Line::styled(format!("{}°", t_frame.hx_now_c), STYLE_CYAN),
    ])
    .centered()
    .render(hx_label_area, buf);

    let [_, hx_gauge_area, _] = Layout::horizontal([
        Constraint::Length(1),
        Constraint::Fill(1),
        Constraint::Length(1),
    ])
    .areas(hx_gauge_area);

    render_hx_vgauge(t_frame.hx_now_c, hx_gauge_area, buf);

    let heat_span = Span::styled(
        if t_frame.heating_on {
            "● HEAT"
        } else {
            "○ HEAT"
        },
        if t_frame.heating_on {
            STYLE_GREEN
        } else {
            STYLE_DARK_GRAY
        },
    );
    let pump_span = Span::styled(
        if t_frame.pump_on {
            "● PUMP"
        } else {
            "○ PUMP"
        },
        if t_frame.pump_on {
            STYLE_GREEN
        } else {
            STYLE_DARK_GRAY
        },
    );

    Paragraph::new(vec![
        Line::raw(""),
        Line::from(heat_span),
        Line::raw(""),
        Line::from(pump_span),
    ])
    .centered()
    .render(status_area, buf);
}

/// Full-width vertical gauge for HX temperature.
///
/// Scale 60–110°C. Ideal zone 90–95°C is marked with `▒` even when unfilled.
/// Colors: white (cold) → green (ideal 88–95°) → yellow (95–100°) → red (>100°).
fn render_hx_vgauge(temp: u16, area: Rect, buf: &mut Buffer) {
    let total = area.height as usize;
    if total == 0 || area.width == 0 {
        return;
    }

    let ratio = ((temp as f64 - HX_SCALE_MIN) / (HX_SCALE_MAX - HX_SCALE_MIN)).clamp(0.0, 1.0);
    let filled = (ratio * total as f64).round() as usize;

    // Convert temperature to row index (bar fills from bottom, so higher temp = lower row index)
    let temp_to_row = |t: f64| -> usize {
        let r = ((t - HX_SCALE_MIN) / (HX_SCALE_MAX - HX_SCALE_MIN)).clamp(0.0, 1.0);
        total.saturating_sub((r * total as f64).round() as usize)
    };
    let ideal_top_row = temp_to_row(HX_IDEAL_HIGH);
    let ideal_bot_row = temp_to_row(HX_IDEAL_LOW);

    let fill_style = match temp {
        0..=87 => STYLE_WHITE,
        88..=95 => STYLE_GREEN,
        96..=100 => STYLE_YELLOW,
        _ => STYLE_RED,
    };

    let w = area.width as usize;
    let fill = "█".repeat(w);
    let empty = "░".repeat(w);
    let ideal = "▒".repeat(w);

    for row in 0..total {
        let y = area.y + row as u16;
        let is_filled = row >= total.saturating_sub(filled);
        let in_ideal = row >= ideal_top_row && row < ideal_bot_row;

        let (s, style) = if is_filled {
            (fill.as_str(), fill_style)
        } else if in_ideal {
            (ideal.as_str(), STYLE_GREEN)
        } else {
            (empty.as_str(), STYLE_DARK_GRAY)
        };

        buf.set_string(area.x, y, s, style);
    }
}

/// Custom boiler bar with absolute temperature scale, color zones, and target marker.
///
/// Visual:  label | ████████████░░░│────────── |
///                  warm     ready  ^target
fn render_boiler_gauge(t_frame: &TelemetryFrame, area: Rect, buf: &mut Buffer) {
    let block = Block::bordered()
        .title("Boiler")
        .border_type(BorderType::Rounded)
        .border_style(STYLE_YELLOW);
    let inner = block.inner(area);
    block.render(area, buf);

    if inner.height == 0 || inner.width == 0 {
        return;
    }

    let Some(target) = t_frame.boiler_target_c else {
        Paragraph::new(Line::from("NO WATER"))
            .centered()
            .style(STYLE_RED)
            .render(inner, buf);
        return;
    };

    let now = t_frame.boiler_now_c;
    let at_temp = now as f64 / target as f64 >= 0.95;

    // Left label: "120°/138° "
    let label = format!("{:3}°/{:3}° ", now, target);
    let label_len = label.len() as u16; // all ASCII + °(1 display width each)
    let label_style = if at_temp { STYLE_GREEN } else { STYLE_YELLOW };
    buf.set_string(inner.x, inner.y, &label, label_style);

    let bar_x = inner.x + label_len;
    let bar_w = inner.width.saturating_sub(label_len) as usize;
    if bar_w == 0 {
        return;
    }

    // Map temperatures to bar positions on 0..BOILER_SCALE_MAX scale
    let pos_of = |temp: u16| -> usize {
        ((temp as f64 / BOILER_SCALE_MAX as f64) * bar_w as f64).round() as usize
    };
    let current_pos = pos_of(now).min(bar_w);
    let target_pos = pos_of(target).min(bar_w.saturating_sub(1));
    // Ready zone starts at 90% of target temperature
    let ready_pos = pos_of((target as f64 * 0.9) as u16);

    for i in 0..bar_w {
        let x = bar_x + i as u16;
        let (ch, style) = if i < current_pos {
            // Filled portion — two-tone: warming → ready
            if i >= ready_pos {
                ("█", STYLE_GREEN)
            } else {
                ("█", STYLE_YELLOW)
            }
        } else if i == target_pos {
            // Target marker (current hasn't reached it yet)
            ("│", STYLE_WHITE)
        } else {
            ("─", STYLE_DARK_GRAY)
        };
        buf.set_string(x, inner.y, ch, style);
    }
}

fn render_shot_gauge(state: &GlobalAppState, area: Rect, buf: &mut Buffer) {
    let extraction_secs = current_extraction_secs(state);
    let has_data = state.extraction_state.is_extracting()
        || state.extraction_state.last_extraction_duration().is_some();

    let block = Block::bordered()
        .title_bottom("%")
        .title_alignment(ratatui::layout::HorizontalAlignment::Center)
        .border_type(BorderType::Rounded)
        .border_style(STYLE_YELLOW);
    let inner = block.inner(area);
    block.render(area, buf);

    if !has_data {
        return;
    }

    let total = inner.height as usize;
    let ratio = (extraction_secs as f64 / SHOT_GAUGE_MAX_SECS as f64).min(1.0);
    let filled = (ratio * total as f64).round() as usize;
    let fill_style = shot_style(extraction_secs, state.extraction_state.is_extracting());
    let fill: String = "█".repeat(inner.width as usize);
    let empty: String = "░".repeat(inner.width as usize);

    for row in 0..total {
        let y = inner.y + row as u16;
        let is_filled = row >= total.saturating_sub(filled);
        if is_filled {
            buf.set_string(inner.x, y, &fill, fill_style);
        } else {
            buf.set_string(inner.x, y, &empty, STYLE_DARK_GRAY);
        }
    }
}

fn render_timer(state: &GlobalAppState, area: Rect, frame: &mut Frame) {
    let buf = frame.buffer_mut();
    let extraction_secs = current_extraction_secs(state);

    let timer_style = if state.extraction_state.is_extracting() {
        shot_style(extraction_secs, true)
    } else if state.extraction_state.last_extraction_duration().is_some() {
        shot_style(extraction_secs, false)
    } else {
        STYLE_DARK_GRAY
    };

    let timer_block = Block::bordered()
        .title_bottom("Extraction")
        .title_alignment(ratatui::layout::HorizontalAlignment::Center)
        .border_type(BorderType::Rounded)
        .border_style(STYLE_YELLOW)
        .padding(Padding::top(1));

    let timer_inner = timer_block.inner(area);
    timer_block.render(area, buf);

    let big_text = BigText::builder()
        .pixel_size(PixelSize::Full)
        .centered()
        .lines(vec![extraction_secs.to_string().into()])
        .style(timer_style)
        .build();

    frame.render_widget(big_text, timer_inner);

    // Post-shot assessment label pinned to the bottom of the block
    if !state.extraction_state.is_extracting()
        && state.extraction_state.last_extraction_duration().is_some()
    {
        let buf = frame.buffer_mut();
        let label_area = Rect {
            x: timer_inner.x,
            y: area.y + area.height.saturating_sub(2),
            width: timer_inner.width,
            height: 1,
        };
        Paragraph::new(Line::from(shot_quality_label(extraction_secs)))
            .centered()
            .style(timer_style)
            .render(label_area, buf);
    }
}

fn current_extraction_secs(state: &GlobalAppState) -> u64 {
    if state.extraction_state.is_extracting() {
        state
            .extraction_state
            .elapsed()
            .unwrap_or(Duration::ZERO)
            .as_secs()
    } else {
        state
            .extraction_state
            .last_extraction_duration()
            .unwrap_or(Duration::ZERO)
            .as_secs()
    }
}

fn shot_style(secs: u64, is_live: bool) -> Style {
    if is_live {
        match secs {
            0..=19 => STYLE_WHITE,
            20..=27 => STYLE_GREEN,
            _ => STYLE_YELLOW,
        }
    } else {
        match secs {
            0..=14 => STYLE_RED,
            15..=19 => STYLE_YELLOW,
            20..=30 => STYLE_GREEN,
            _ => STYLE_YELLOW,
        }
    }
}

fn shot_quality_label(secs: u64) -> &'static str {
    match secs {
        15..=19 => "UNDEREXTRACTED",
        20..=23 => "GOOD",
        24..=27 => "PERFECT",
        28..=31 => "LONG SHOT",
        32..=39 => "BLONDING",
        _ => "RIP SHOT",
    }
}
