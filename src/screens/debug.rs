use ratatui::layout::Rect;
use ratatui::text::Line;
use ratatui::widgets::Paragraph;
use ratatui::{Frame, widgets::Widget};

use crate::{screens::screen::Board, telemetry::TelemetryFrame};

#[derive(Default)]
pub struct Debug {
    pub state: Option<TelemetryFrame>,
}

impl Board for Debug {
    fn render(&self, area: Rect, frame: &mut Frame) {
        let buf = frame.buffer_mut();
        let text = match &self.state {
            Some(telemetry) => format!("UART: {}", telemetry.raw_string),
            None => "UART: No data (waiting for connection...)".to_string(),
        };

        Paragraph::new(vec![Line::from(text)]).render(area, buf);
    }
}

impl Debug {
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
