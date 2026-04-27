use std::time::Duration;

use ratatui::Frame;
use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Gauge, Padding, Paragraph, Widget};
use tui_widgets::big_text::{BigText, PixelSize};

use crate::screens::screen::Board;
use crate::state::GlobalAppState;
use crate::telemetry::{MachineMode, TelemetryFrame};

const SHOT_GAUGE_MAX_SECS: u64 = 30;

#[derive(Default)]
pub struct Dashboard;

impl Board for Dashboard {
    fn render(state: &GlobalAppState, area: Rect, frame: &mut Frame) {
        let buf = frame.buffer_mut();

        let t_frame = match &state.machine_state.last_frame {
            Some(f) => f.clone(),
            None => TelemetryFrame::debug_frame(),
        };

        // banner (1) | main content (fill) | boiler gauge (3)
        let [banner_area, content_area, boiler_area] = Layout::vertical([
            Constraint::Length(1),
            Constraint::Fill(1),
            Constraint::Length(3),
        ])
        .areas(area);

        render_mode_banner(&t_frame, banner_area, buf);
        render_boiler_gauge(&t_frame, boiler_area, buf);

        // info (fill 1) | timer (fill 3) | shot gauge (5 wide)
        let [info_col, timer_col, gauge_col] = Layout::horizontal([
            Constraint::Fill(1),
            Constraint::Fill(3),
            Constraint::Length(5),
        ])
        .areas(content_area);

        render_info_col(&t_frame, info_col, buf);
        render_shot_gauge(state, gauge_col, buf);
        render_timer(state, timer_col, frame);
    }
}

fn render_mode_banner(t_frame: &TelemetryFrame, area: Rect, buf: &mut Buffer) {
    let (label, style) = match t_frame.mode {
        MachineMode::Coffee => ("═══ COFFEE MODE ═══", Style::new().green()),
        MachineMode::SteamS | MachineMode::SteamV | MachineMode::SteamC => {
            ("═══ STEAM MODE ═══", Style::new().cyan())
        }
        MachineMode::Offline => ("═══  OFFLINE  ═══", Style::new().red()),
        MachineMode::Unknown(_) => ("═══  UNKNOWN  ═══", Style::new().gray()),
    };

    Paragraph::new(Line::from(label))
        .centered()
        .style(style)
        .render(area, buf);
}

fn render_info_col(t_frame: &TelemetryFrame, area: Rect, buf: &mut Buffer) {
    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(Style::new().yellow());
    let inner = block.inner(area);
    block.render(area, buf);

    let [hx_area, status_area] =
        Layout::vertical([Constraint::Fill(1), Constraint::Length(4)]).areas(inner);

    Paragraph::new(vec![
        Line::styled("HX", Style::new().dark_gray()),
        Line::raw(""),
        Line::styled(format!("{}°", t_frame.hx_now_c), Style::new().cyan()),
    ])
    .centered()
    .render(hx_area, buf);

    let heat_span = Span::styled(
        if t_frame.heating_on {
            "● HEAT"
        } else {
            "○ HEAT"
        },
        if t_frame.heating_on {
            Style::new().green()
        } else {
            Style::new().dark_gray()
        },
    );
    let pump_span = Span::styled(
        if t_frame.pump_on {
            "● PUMP"
        } else {
            "○ PUMP"
        },
        if t_frame.pump_on {
            Style::new().green()
        } else {
            Style::new().dark_gray()
        },
    );

    Paragraph::new(vec![
        Line::from(heat_span),
        Line::raw(""),
        Line::from(pump_span),
    ])
    .centered()
    .render(status_area, buf);
}

fn render_boiler_gauge(t_frame: &TelemetryFrame, area: Rect, buf: &mut Buffer) {
    let block = Block::bordered()
        .title("Boiler")
        .border_type(BorderType::Rounded)
        .border_style(Style::new().yellow());

    if let Some(target) = t_frame.boiler_target_c {
        let now = t_frame.boiler_now_c;
        let ratio = (now as f64 / target as f64).min(1.0);
        let gauge_style = if ratio >= 0.95 {
            Style::new().on_green()
        } else {
            Style::new().yellow()
        };

        Gauge::default()
            .block(block)
            .gauge_style(gauge_style)
            .ratio(ratio)
            .label(Span::styled(
                format!("{}° / {}°", now, target),
                Style::new().black(),
            ))
            .render(area, buf);
    } else {
        Paragraph::new(Line::from("NO WATER"))
            .centered()
            .style(Style::new().red())
            .block(block)
            .render(area, buf);
    }
}

fn render_shot_gauge(state: &GlobalAppState, area: Rect, buf: &mut Buffer) {
    let extraction_secs = current_extraction_secs(state);
    let has_data = state.extraction_state.is_extracting()
        || state.extraction_state.last_extraction_duration().is_some();

    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .title_bottom("%")
        .title_alignment(ratatui::layout::HorizontalAlignment::Center)
        .border_style(Style::new().yellow());
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
            buf.set_string(inner.x, y, &empty, Style::new().dark_gray());
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
        Style::new().dark_gray()
    };

    let timer_block = Block::bordered()
        .title("Extraction")
        .title_alignment(ratatui::layout::HorizontalAlignment::Center)
        .border_type(BorderType::Rounded)
        .border_style(Style::new().yellow())
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

    // Post-shot quality label below the number
    if !state.extraction_state.is_extracting()
        && state.extraction_state.last_extraction_duration().is_some()
    {
        let buf = frame.buffer_mut();
        let label_area = Rect {
            x: area.x,
            y: area.y + area.height.saturating_sub(2),
            width: area.width,
            height: 1,
        };
        let label = shot_quality_label(extraction_secs);
        Paragraph::new(Line::from(label))
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
        // Friendly: neutral until the sweet spot, then indicate
        match secs {
            0..=19 => Style::new().white(),
            20..=30 => Style::new().green(),
            _ => Style::new().yellow(),
        }
    } else {
        // Post-shot: semantic verdict
        match secs {
            0..=14 => Style::new().red(),
            15..=19 => Style::new().yellow(),
            20..=30 => Style::new().green(),
            _ => Style::new().yellow(),
        }
    }
}

fn shot_quality_label(secs: u64) -> &'static str {
    match secs {
        0..=14 => "TOO SHORT",
        15..=19 => "UNDEREXTRACTED",
        20..=30 => "PERFECT",
        31..=39 => "OVEREXTRACTED",
        _ => "TOO LONG",
    }
}
