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
    fn render(state: &GlobalAppState, area: Rect, frame: &mut Frame) {
        let datasets = vec![
            Dataset::default()
                .name("Current")
                .marker(symbols::Marker::Braille)
                .style(Style::default().fg(Color::Yellow))
                .graph_type(GraphType::Line)
                .data(&state.machine_state.graph_boiler_current),
            Dataset::default()
                .name("Target")
                .marker(symbols::Marker::Braille)
                .style(Style::default().fg(Color::Red))
                .graph_type(GraphType::Line)
                .data(&state.machine_state.graph_boiler_target),
            Dataset::default()
                .name("HX")
                .marker(symbols::Marker::Braille)
                .style(Style::default().fg(Color::Blue))
                .graph_type(GraphType::Line)
                .data(&state.machine_state.graph_hx),
        ];

        let chart = Chart::new(datasets)
            .block(Block::bordered().title(Line::from("Temperatures").cyan().bold().centered()))
            .x_axis(
                Axis::default()
                    .style(Style::default().gray())
                    .labels(["-5 min", "Now"])
                    .bounds([0.0, 300.0]),
            )
            .y_axis(
                Axis::default()
                    .style(Style::default().gray())
                    .labels(["30°", "90°", "150°"])
                    .bounds([30.0, 150.0]),
            )
            .legend_position(Some(LegendPosition::BottomRight))
            .hidden_legend_constraints((Constraint::Min(0), Constraint::Ratio(1, 2)));

        frame.render_widget(chart, area);
    }
}
