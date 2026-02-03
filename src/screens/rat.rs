use crate::qoi_widget::QoiImage;
use crate::screens::screen::Board;
use crate::state::GlobalAppState;
use ratatui::Frame;
use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::text::Line;
use ratatui::widgets::{Block, Padding, Paragraph, Widget};
use tinyqoi::Qoi;

/// Main screen with rat barista mascot and system status
#[derive(Default)]
pub struct Rat;

impl Board for Rat {
    fn render(_state: &GlobalAppState, area: Rect, frame: &mut Frame) {
        let buf = frame.buffer_mut();
        Self::render_main(area, buf);
    }
}

impl Rat {
    /// Render the main layout with rat image and status text
    fn render_main(area: Rect, buf: &mut Buffer) {
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
        Self::render_main_status(status_area, buf);
    }

    /// Render status information text
    fn render_main_status(area: Rect, buf: &mut Buffer) {
        let mut lines = Vec::new();

        // Status text - split by newlines to create multiple lines
        let status = "System ready\nfor coffee.";

        // Split by newlines and create a Line for each part
        for line_text in status.split('\n') {
            lines.push(Line::from(line_text));
        }

        // Mode and cups
        let mode_str = "COFFEE";
        lines.push(Line::from(""));
        lines.push(Line::from(format!("Mode: {mode_str}")));

        Paragraph::new(lines)
            .block(Block::default().padding(Padding::new(1, 0, 1, 0)))
            .render(area, buf);
    }
}
