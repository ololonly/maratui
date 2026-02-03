use std::time::Duration;

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::widgets::{Block, BorderType, Padding, Paragraph, Widget};
use tui_widgets::big_text::{BigText, PixelSize};

use crate::screens::screen::Board;
use crate::state::GlobalAppState;
use crate::telemetry::TelemetryFrame;

/// Dashboard screen showing machine status and extraction timer
#[derive(Default)]
pub struct Dashboard;

impl Board for Dashboard {
    fn render(state: &GlobalAppState, area: Rect, frame: &mut Frame) {
        let buf = frame.buffer_mut();

        // Get telemetry frame or use debug frame if not available
        let t_frame = match &state.last_telemetry {
            Some(frame) => frame.clone(),
            None => TelemetryFrame::debug_frame(),
        };

        let [col1, col2] = Layout::horizontal(Constraint::from_fills([1, 2])).areas(area);

        let col1_areas = Layout::vertical([Constraint::Fill(1); 3]).split(col1);
        let col2_areas = Layout::vertical(Constraint::from_fills([2, 1])).split(col2);
        let col2_row2_areas = Layout::horizontal([Constraint::Fill(1); 2]).split(col2_areas[1]);

        if let Some(tt) = t_frame.boiler_target_c {
            let boiler_block = Block::bordered()
                .title("Boiler")
                .border_type(BorderType::Rounded)
                .border_style(Style::new().yellow())
                .padding(Padding::left(1));

            Paragraph::new(vec![
                Line::from(format!("Target  {tt}°")),
                Line::raw(""),
                Line::from(format!("Current {}°", t_frame.boiler_now_c)),
            ])
            .left_aligned()
            .block(boiler_block)
            .render(col1_areas[0], buf);

            let hx_block = Block::bordered()
                .title("HX")
                .border_type(BorderType::Rounded)
                .border_style(Style::new().yellow())
                .padding(Padding::top(1));

            Paragraph::new(Line::from(format!("{}°", t_frame.hx_now_c)))
                .centered()
                .block(hx_block)
                .render(col1_areas[1], buf);

            let mode_block = Block::bordered()
                .title("Mode")
                .border_type(BorderType::Rounded)
                .border_style(Style::new().yellow())
                .padding(Padding::top(1));

            Paragraph::new(Line::from(t_frame.mode.to_string().to_uppercase()))
                .centered()
                .block(mode_block)
                .render(col1_areas[2], buf);
        }

        // Display extraction timer
        let extraction_time = if state.extraction_state.is_extracting() {
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
        };
        let big_text = BigText::builder()
            .pixel_size(PixelSize::Full)
            .centered()
            .lines(vec![Line::from(format!("{}", extraction_time))])
            .style(Style::new().green())
            .build();

        let heat_block = Block::bordered()
            .title("Heating")
            .border_type(BorderType::Rounded)
            .border_style(Style::new().yellow())
            .padding(Padding::top(1));

        Paragraph::new(Line::from(if t_frame.heating_on { "ON" } else { "OFF" }))
            .centered()
            .style(if t_frame.heating_on {
                Style::new().green()
            } else {
                Style::new().gray()
            })
            .block(heat_block)
            .render(col2_row2_areas[0], buf);

        let pump_block = Block::bordered()
            .title("Pump")
            .border_type(BorderType::Rounded)
            .border_style(Style::new().yellow())
            .padding(Padding::top(1));

        Paragraph::new(Line::from(if t_frame.pump_on { "ON" } else { "OFF" }))
            .centered()
            .style(if t_frame.pump_on {
                Style::new().green()
            } else {
                Style::new().gray()
            })
            .block(pump_block)
            .render(col2_row2_areas[1], buf);

        frame.render_widget(big_text, col2);
    }
}
