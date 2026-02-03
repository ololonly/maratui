use ratatui::layout::Rect;
use ratatui::text::Line;
use ratatui::widgets::Paragraph;
use ratatui::{Frame, widgets::Widget};

use crate::screens::screen::Board;
use crate::state::GlobalAppState;

/// Debug screen showing raw UART telemetry data
#[derive(Default)]
pub struct Debug;

impl Board for Debug {
    fn render(state: &GlobalAppState, area: Rect, frame: &mut Frame) {
        let buf = frame.buffer_mut();
        let text = match &state.last_telemetry {
            Some(telemetry) => format!("UART: {}", telemetry.raw_string),
            None => "UART: No data (waiting for connection...)".to_string(),
        };

        Paragraph::new(vec![Line::from(text)]).render(area, buf);
    }
}
