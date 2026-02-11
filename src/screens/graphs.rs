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
        let hx_states: Vec<(f64, f64)> = state
            .machine_state
            .current_hx_data
            .iter()
            .enumerate()
            .map(|(i, &v)| (i as f64, v))
            .collect();

        let curr_boiler_states: Vec<(f64, f64)> = state
            .machine_state
            .current_boiler_data
            .iter()
            .enumerate()
            .map(|(i, &v)| (i as f64, v))
            .collect();

        let target_boiler_states: Vec<(f64, f64)> = state
            .machine_state
            .target_boiler_data
            .iter()
            .enumerate()
            .map(|(i, &v)| (i as f64, v))
            .collect();

        let datasets = vec![
            Dataset::default()
                .name("Current temperature")
                .marker(symbols::Marker::Braille)
                .style(Style::default().fg(Color::Yellow))
                .graph_type(GraphType::Line)
                .data(&curr_boiler_states),
            Dataset::default()
                .name("Target temperature")
                .marker(symbols::Marker::Braille)
                .style(Style::default().fg(Color::Red))
                .graph_type(GraphType::Line)
                .data(&target_boiler_states),
            Dataset::default()
                .name("HX temperature")
                .marker(symbols::Marker::Braille)
                .style(Style::default().fg(Color::Blue))
                .graph_type(GraphType::Line)
                .data(&hx_states),
        ];

        let chart = Chart::new(datasets)
            .block(Block::bordered().title(Line::from("Line chart").cyan().bold().centered()))
            .x_axis(
                Axis::default()
                    .style(Style::default().gray())
                    .labels(["-5 min", "Now"])
                    .bounds([0.0, 300.0]),
            )
            .y_axis(
                Axis::default()
                    .title("Temp")
                    .style(Style::default().gray())
                    .labels(["30°", "90°", "150°"])
                    .bounds([30.0, 150.0]),
            )
            .legend_position(Some(LegendPosition::BottomRight))
            .hidden_legend_constraints((Constraint::Min(0), Constraint::Ratio(1, 2)));

        frame.render_widget(chart, area);
    }
}
