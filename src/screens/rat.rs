use crate::qoi_widget::QoiImage;
use crate::screens::screen::Board;
use crate::state::GlobalAppState;
use ratatui::Frame;
use ratatui::buffer::Buffer;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Paragraph, Widget, Wrap};
use tinyqoi::Qoi;

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

        // Left area: render QOI image as Ratatui widget
        let rat_area = layout[0];
        let data = include_bytes!("../../assets/rat_barista.qoi");
        let qoi = Qoi::new(data).unwrap();
        let image_widget = QoiImage::new(&qoi);

        image_widget.render(rat_area, buf);

        // Render status text on the right
        let status_area = layout[1];
        Self::render_main_status(state, status_area, buf);
    }

    /// Render status information text
    fn render_main_status(state: &GlobalAppState, area: Rect, buf: &mut Buffer) {
        // Paragraph::from(vec!["Hello!", "Let's make some coffee!"])
        //     .block(Block::default().padding(Padding::new(1, 0, 1, 0)))
        //     .render(area, buf);
        let lines = Line::from(vec![
            Span::raw("Hello!"),
            Span::styled("Let's make some coffee!", Style::new().white().italic()),
        ]);
        Paragraph::new(lines)
            .block(Block::bordered().border_type(BorderType::Rounded))
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true })
            .render(area, buf);
    }
}
