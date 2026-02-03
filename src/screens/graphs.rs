use ratatui::layout::{Constraint, Rect};
use ratatui::style::{Color, Style, Stylize};
use ratatui::text::Line;
use ratatui::widgets::{Axis, Block, Chart, Dataset, GraphType, LegendPosition};
use ratatui::{Frame, symbols};

use crate::screens::screen::Board;
use crate::state::GlobalAppState;

/// Graphs screen showing temperature trends
#[derive(Default)]
pub struct Graphs;

impl Board for Graphs {
    fn render(_state: &GlobalAppState, area: Rect, frame: &mut Frame) {
        let points = Self::demo_dataset();
        let points2 = Self::demo_dataset2();

        let datasets = vec![
            Dataset::default()
                .name("Current temperature")
                .marker(symbols::Marker::Braille)
                .style(Style::default().fg(Color::Yellow))
                .graph_type(GraphType::Line)
                .data(&points),
            Dataset::default()
                .name("Target temperature")
                .marker(symbols::Marker::Braille)
                .style(Style::default().fg(Color::Red))
                .graph_type(GraphType::Line)
                .data(&points2),
        ];

        let chart = Chart::new(datasets)
            .block(Block::bordered().title(Line::from("Line chart").cyan().bold().centered()))
            .x_axis(
                Axis::default()
                    .title("Time")
                    .style(Style::default().gray())
                    .bounds([0.0, 360.0]),
            )
            .y_axis(
                Axis::default()
                    .title("Temp")
                    .style(Style::default().gray())
                    .bounds([0.0, 140.0]),
            )
            .legend_position(Some(LegendPosition::TopLeft))
            .hidden_legend_constraints((Constraint::Ratio(1, 2), Constraint::Ratio(1, 2)));

        frame.render_widget(chart, area);
    }
}

impl Graphs {
    /// Generate demo dataset for current temperature
    fn demo_dataset() -> Vec<(f64, f64)> {
        let mut data = Vec::new();

        for i in 0..360 {
            let x = i as f64;
            let y = (i / 2) as f64;
            data.push((x, y));
        }

        data
    }

    /// Generate demo dataset for target temperature
    fn demo_dataset2() -> Vec<(f64, f64)> {
        let mut data = Vec::new();

        for i in 0..360 {
            let x = i as f64;
            let y = 128 as f64;
            data.push((x, y));
        }

        data
    }
}
