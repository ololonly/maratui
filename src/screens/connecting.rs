use ratatui::Frame;
use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::widgets::{Block, BorderType, Paragraph, Widget};

use crate::screens::screen::Board;
use crate::state::GlobalAppState;

/// Shown on device boot until the first UART telemetry frame arrives.
#[derive(Default)]
pub struct Connecting;

impl Board for Connecting {
    fn render(state: &GlobalAppState, area: Rect, frame: &mut Frame) {
        let buf = frame.buffer_mut();
        Self::render_inner(state, area, buf);
    }
}

impl Connecting {
    fn render_inner(_state: &GlobalAppState, area: Rect, buf: &mut Buffer) {
        let block = Block::bordered()
            .border_type(BorderType::Rounded)
            .border_style(Style::new().yellow());
        let inner = block.inner(area);
        block.render(area, buf);

        let [_, center_area, _] = Layout::vertical([
            Constraint::Fill(1),
            Constraint::Length(3),
            Constraint::Fill(1),
        ])
        .areas(inner);

        Paragraph::new(vec![
            Line::styled("═══ MARATUI ═══", Style::new().yellow()),
            Line::raw(""),
            Line::styled("waiting for machine...", Style::new().dark_gray()),
        ])
        .centered()
        .render(center_area, buf);
    }
}
