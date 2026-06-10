use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Axis, Block, Chart, Dataset, GraphType, Paragraph};
use ratatui::{Frame, symbols};

use crate::screens::screen::Board;
use crate::state::GlobalAppState;

// Shared with the Dashboard screen's color coding: HX is cyan and
// "current" values are yellow. Target uses magenta so it stays visually
// distinct from HX (cyan and white were too close together).
const COLOR_CURRENT: Color = Color::Yellow;
const COLOR_TARGET: Color = Color::Magenta;
const COLOR_HX: Color = Color::Cyan;

const Y_AXIS_FALLBACK: [f64; 2] = [30.0, 150.0];

/// Graphs screen showing temperature trends
#[derive(Default)]
pub struct Graphs;

impl Board for Graphs {
    fn render(state: &GlobalAppState, area: Rect, frame: &mut Frame) {
        let [chart_area, legend_area] =
            Layout::vertical([Constraint::Fill(1), Constraint::Length(1)]).areas(area);

        let datasets = vec![
            Dataset::default()
                .marker(symbols::Marker::Braille)
                .style(Style::default().fg(COLOR_CURRENT))
                .graph_type(GraphType::Line)
                .data(&state.machine_state.graph_boiler_current),
            Dataset::default()
                .marker(symbols::Marker::Braille)
                .style(Style::default().fg(COLOR_TARGET))
                .graph_type(GraphType::Line)
                .data(&state.machine_state.graph_boiler_target),
            Dataset::default()
                .marker(symbols::Marker::Braille)
                .style(Style::default().fg(COLOR_HX))
                .graph_type(GraphType::Line)
                .data(&state.machine_state.graph_hx),
        ];

        let series = [
            state.machine_state.graph_boiler_current.as_slice(),
            state.machine_state.graph_boiler_target.as_slice(),
            state.machine_state.graph_hx.as_slice(),
        ];
        let [y_min, y_max] = y_bounds(&series);
        let y_mid = (y_min + y_max) / 2.0;

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
                    .labels([
                        format!("{y_min:.0}°"),
                        format!("{y_mid:.0}°"),
                        format!("{y_max:.0}°"),
                    ])
                    .bounds([y_min, y_max]),
            );

        frame.render_widget(chart, chart_area);
        render_legend(legend_area, frame);
    }
}

/// Computes Y-axis bounds from all visible series, padded by 5° and
/// rounded to multiples of 5 so the axis labels stay tidy. The lower
/// bound is clamped to 0° (temperatures here never go negative) — it's
/// the bound that matters most, since it tracks how far current readings
/// have climbed from room temperature toward operating temperature. The
/// upper bound stays close to the boiler target (~115-140°) regardless.
/// Falls back to a fixed range while no telemetry has been collected yet.
fn y_bounds(series: &[&[(f64, f64)]]) -> [f64; 2] {
    let mut min = f64::MAX;
    let mut max = f64::MIN;
    for s in series {
        for &(_, y) in *s {
            min = min.min(y);
            max = max.max(y);
        }
    }

    if min > max {
        return Y_AXIS_FALLBACK;
    }

    let lower = (((min - 5.0) / 5.0).floor() * 5.0).max(0.0);
    let upper = ((max + 5.0) / 5.0).ceil() * 5.0;
    [lower, upper]
}

fn render_legend(area: Rect, frame: &mut Frame) {
    let line = Line::from(vec![
        "● ".fg(COLOR_CURRENT),
        Span::raw("Current   "),
        "● ".fg(COLOR_TARGET),
        Span::raw("Target   "),
        "● ".fg(COLOR_HX),
        Span::raw("HX"),
    ])
    .centered();

    frame.render_widget(Paragraph::new(line), area);
}
