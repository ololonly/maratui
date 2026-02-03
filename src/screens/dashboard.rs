use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::widgets::{Block, BorderType, Padding, Paragraph, Widget, Wrap};
use tui_widgets::big_text::{BigText, PixelSize};

use crate::brew_timer::BrewTimer;
use crate::{screens::screen::Board, telemetry::TelemetryFrame};

#[derive(Default)]
pub struct Dashboard {
    pub state: Option<TelemetryFrame>,
    pub brew_timer: BrewTimer,
}

impl Board for Dashboard {
    fn render(&self, area: Rect, frame: &mut Frame) {
        let buf = frame.buffer_mut();

        let t_frame = self.state.as_ref().unwrap();

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
            .wrap(Wrap { trim: true })
            .left_aligned()
            .block(boiler_block)
            .render(col1_areas[0], buf);

            let hx_block = Block::bordered()
                .title("HX")
                .border_type(BorderType::Rounded)
                .border_style(Style::new().yellow())
                .padding(Padding::new(0, 0, 1, 0));

            Paragraph::new(Line::from(format!("{}°", t_frame.hx_now_c)))
                .wrap(Wrap { trim: true })
                .centered()
                .block(hx_block)
                .render(col1_areas[1], buf);

            let mode_block = Block::bordered()
                .title("Mode")
                .border_type(BorderType::Rounded)
                .border_style(Style::new().yellow())
                .padding(Padding::new(1, 0, 1, 0));

            Paragraph::new(Line::from(t_frame.mode.to_string().to_uppercase()))
                .wrap(Wrap { trim: true })
                .centered()
                .block(mode_block)
                .render(col1_areas[2], buf);
        }

        let time = self.brew_timer.elapsed_secs();

        let big_text = BigText::builder()
            .pixel_size(PixelSize::Full)
            .centered()
            .lines(vec![Line::from(format!("{}", time))])
            .style(Style::new().magenta())
            .build();

        let heat_block = Block::bordered()
            .title("Heating")
            .border_type(BorderType::Rounded)
            .border_style(Style::new().yellow())
            .padding(Padding::new(1, 0, 1, 0));

        let pump_block = Block::bordered()
            .title("Pump")
            .border_type(BorderType::Rounded)
            .border_style(Style::new().yellow())
            .padding(Padding::new(1, 0, 1, 0));

        Paragraph::new(Line::from(format!("Heating {}", t_frame.heating_on)))
            .wrap(Wrap { trim: true })
            .centered()
            .block(heat_block)
            .render(col2_row2_areas[0], buf);
        Paragraph::new(Line::from(format!("Pump {}", t_frame.pump_on)))
            .wrap(Wrap { trim: true })
            .centered()
            .block(pump_block)
            .render(col2_row2_areas[1], buf);

        frame.render_widget(big_text, col2);
    }
}

impl Dashboard {
    pub fn new(state: &Option<TelemetryFrame>, brew_timer: BrewTimer) -> Self {
        Self {
            state: state.clone(),
            brew_timer: brew_timer,
        }
    }

    pub fn default() -> Self {
        let bt = BrewTimer::new();
        Self {
            state: Some(TelemetryFrame::debug_frame()),
            brew_timer: bt,
        }
    }
}
