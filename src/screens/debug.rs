use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::text::{Line, Text};
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

        let [net_line, uart_line, log_area] = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(1),
        ])
        .areas(area);

        let net_text = format!(
            "NET: Wi-Fi={:?} MQTT={:?}",
            state.wifi_status, state.mqtt_status
        );

        let text = match &state.machine_state.last_frame {
            Some(telemetry) => format!("UART: {}", telemetry.raw_string),
            None => "UART: No data (waiting for connection...)".to_string(),
        };

        let lines = state.events_log.iter().map(|s| Line::from(s.to_string()));

        Paragraph::new(vec![Line::from(net_text)]).render(net_line, buf);
        Paragraph::new(vec![Line::from(text)]).render(uart_line, buf);
        Paragraph::new(Text::from_iter(lines)).render(log_area, buf);
    }
}
