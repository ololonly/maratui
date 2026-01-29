use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::text::Line;
use ratatui::widgets::{Block, Paragraph, Widget, Wrap};
use tui_widgets::big_text::{BigText, PixelSize};

use crate::{screens::screen::Board, telemetry::TelemetryFrame};

#[derive(Default)]
pub struct Dashboard {
    pub state: Option<TelemetryFrame>,
}

impl Board for Dashboard {
    fn render(&self, area: Rect, frame: &mut Frame) {
        let buf = frame.buffer_mut();

        let t_frame = self.state.as_ref().unwrap();

        let [col1, col2, col3] = Layout::horizontal([Constraint::Fill(1); 3]).areas(area);

        let col1_areas = Layout::vertical([Constraint::Fill(1); 3]).split(col1);
        let col3_areas = Layout::vertical([Constraint::Fill(1); 3]).split(col3);

        if let Some(tt) = t_frame.boiler_target_c {
            Paragraph::new(Line::from(format!("Target {tt}")))
                .wrap(Wrap { trim: true })
                .centered()
                .block(Block::default())
                .render(col1_areas[0], buf);
            Paragraph::new(Line::from(format!("Current {}", t_frame.boiler_now_c)))
                .wrap(Wrap { trim: true })
                .centered()
                .block(Block::default())
                .render(col1_areas[1], buf);
            Paragraph::new(Line::from(format!("Current HX {}", t_frame.hx_now_c)))
                .wrap(Wrap { trim: true })
                .centered()
                .block(Block::default())
                .render(col1_areas[2], buf);
        }

        let big_text = BigText::builder()
            .pixel_size(PixelSize::HalfWidth)
            .centered()
            .lines(vec!["138".into()])
            .build();

        Paragraph::new(Line::from(format!("Mode {}", t_frame.mode.to_string())))
            .wrap(Wrap { trim: true })
            .centered()
            .block(Block::default())
            .render(col3_areas[0], buf);
        Paragraph::new(Line::from(format!("Heating {}", t_frame.heating_on)))
            .wrap(Wrap { trim: true })
            .centered()
            .block(Block::default())
            .render(col3_areas[1], buf);
        Paragraph::new(Line::from(format!("Pump {}", t_frame.pump_on)))
            .wrap(Wrap { trim: true })
            .centered()
            .block(Block::default())
            .render(col3_areas[2], buf);

        frame.render_widget(big_text, col2);
    }
}

impl Dashboard {
    pub fn new(state: &Option<TelemetryFrame>) -> Self {
        Self {
            state: state.clone(),
        }
    }

    pub fn default() -> Self {
        Self {
            state: Some(TelemetryFrame::debug_frame()),
        }
    }
}
