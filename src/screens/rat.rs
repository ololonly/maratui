use crate::screens::screen::Board;
use crate::state::{AppError, GlobalAppState};
use ratatui::Frame;
use ratatui::buffer::Buffer;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::widgets::{Block, BorderType, Paragraph, Widget, Wrap};

/// Main screen with rat barista mascot and system status
#[derive(Default)]
pub struct Rat;

impl Board for Rat {
    fn render(_state: &GlobalAppState, area: Rect, frame: &mut Frame) {
        let buf = frame.buffer_mut();
        Self::render_main(_state, area, buf);
    }
}

impl Rat {
    /// Render the main layout with rat image and status text
    fn render_main(state: &GlobalAppState, area: Rect, buf: &mut Buffer) {
        let layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
            .split(area);

        let status_area = layout[1];
        Self::render_main_status(state, status_area, buf);
    }

    /// Render status information text
    fn render_main_status(state: &GlobalAppState, area: Rect, buf: &mut Buffer) {
        let [state_area, message_area, _icon_area] = Layout::vertical([
            Constraint::Fill(1),
            Constraint::Fill(1),
            Constraint::Fill(1),
        ])
        .areas(area);

        let status_text = match state.error {
            Some(AppError::WaterRefillNeeded { .. }) => "I am thirsty!",
            _ => "Let's make some coffee!",
        };

        let lines = vec![
            Line::raw("Hello!"),
            Line::styled("I am Rat Barista", Style::new().white().italic()),
        ];
        Paragraph::new(lines)
            .block(Block::bordered().border_type(BorderType::Rounded))
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true })
            .render(state_area, buf);

        Paragraph::new(Line::from(status_text))
            .block(Block::bordered().border_type(BorderType::Rounded))
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true })
            .render(message_area, buf);
    }
}
