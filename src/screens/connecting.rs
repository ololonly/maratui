use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Gauge, Paragraph, Widget};
use tui_widgets::big_text::BigText;

use crate::screens::screen::Board;
use crate::state::GlobalAppState;

#[derive(Default)]
pub struct Connecting;

impl Board for Connecting {
    fn render(state: &GlobalAppState, area: Rect, frame: &mut Frame) {
        Self::render_inner(state, area, frame);
    }
}

impl Connecting {
    fn render_inner(state: &GlobalAppState, area: Rect, frame: &mut Frame) {
        // Top 13 rows (≈182 px) left empty — rat barista image rendered directly via render_image()
        let [top, bottom] =
            Layout::vertical([Constraint::Length(13), Constraint::Fill(1)]).areas(area);

        // Version in the top-right corner (outside the image footprint which ends at ~col 27)
        let [_, label_render_area] =
            Layout::horizontal([Constraint::Percentage(60), Constraint::Fill(1)]).areas(top);

        let [ver_area, title_area] =
            Layout::vertical([Constraint::Length(1), Constraint::Fill(1)]).areas(label_render_area);

        Paragraph::new(concat!("v", env!("CARGO_PKG_VERSION")))
            .alignment(Alignment::Right)
            .style(Style::new().dark_gray())
            .render(ver_area, frame.buffer_mut());

        let [_, centered_title_area, _] = Layout::vertical([
            Constraint::Fill(1),
            Constraint::Length(8),
            Constraint::Fill(1),
        ])
        .areas(title_area);

        let title_big_text = BigText::builder()
            .alignment(ratatui::layout::HorizontalAlignment::Center)
            .lines(vec!["MARA".into(), "TUI".into()])
            .style(Style::new().yellow())
            .pixel_size(tui_widgets::big_text::PixelSize::Quadrant)
            .build();

        frame.render_widget(title_big_text, centered_title_area);

        // Resolve current message and progress from state
        let (message, progress) = state.loading_status.unwrap_or(("starting up...", 0));
        let ratio = f64::from(progress) / 100.0;

        // Loading block: rounded yellow border, message + gauge inside
        let block = Block::bordered()
            .border_type(BorderType::Double)
            .border_style(Style::new().black());
        let inner = block.inner(bottom);
        block.render(bottom, frame.buffer_mut());

        let [msg_row, gauge_row] =
            Layout::vertical([Constraint::Length(1), Constraint::Length(1)]).areas(inner);

        Paragraph::new(Line::styled(message, Style::new().dark_gray()))
            .centered()
            .render(msg_row, frame.buffer_mut());

        Gauge::default()
            .gauge_style(Style::default().fg(Color::Yellow).bg(Color::Gray))
            .label(Span::raw(""))
            .ratio(ratio)
            .render(gauge_row, frame.buffer_mut());
    }
}
