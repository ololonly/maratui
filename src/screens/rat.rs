use crate::qoi_widget::QoiImage;
use mousefood::prelude::*;
use mousefood::ratatui::layout::{Direction, Layout};
use mousefood::ratatui::widgets::{Block, Padding, Paragraph};
use tinyqoi::Qoi;

use crate::{screens::screen::Board, telemetry::TelemetryFrame};

#[derive(Default)]
pub struct Rat {
    //pub state: Option<TelemetryFrame>,
}

impl Board for Rat {
    fn render(&self, area: Rect, frame: &mut Frame) {
        let buf = frame.buffer_mut();
        self.render_main(area, buf);
    }
}

impl Rat {
    fn render_main(&self, area: Rect, buf: &mut Buffer) {
        // Layout: pixel rat on left, text info on right
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
        self.render_main_status(status_area, buf);
    }

    fn render_main_status(
        &self,
        area: Rect,
        buf: &mut Buffer,
        // snapshot: Option<&Snapshot>,
        // cups: Option<u32>,
    ) {
        let mut lines = Vec::new();

        // Status text - split by newlines to create multiple lines
        let status = "System ready\nfor coffee.";

        // Split by newlines and create a Line for each part
        for line_text in status.split('\n') {
            lines.push(mousefood::ratatui::text::Line::from(line_text));
        }

        // Mode and cups
        let mode_str = "COFFEE";
        lines.push(mousefood::ratatui::text::Line::from(""));
        lines.push(mousefood::ratatui::text::Line::from(format!(
            "Mode: {mode_str}"
        )));

        Paragraph::new(lines)
            .block(Block::default().padding(Padding::new(1, 0, 1, 0)))
            .render(area, buf);
    }
}
