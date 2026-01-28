use maratui::button::{Button, ButtonPressType};
use maratui::qoi_widget::QoiImage;
use maratui::screens::{Board, Dashboard, Rat, Screen};
use maratui::setup::MaraUiApp;
use maratui::telemetry::TelemetryFrame;
use ratatui::Frame;
use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Style, Stylize};
use ratatui::symbols;
use ratatui::text::Line;
use ratatui::widgets::{
    Axis, Block, Chart, Dataset, GraphType, LegendPosition, Paragraph, Tabs, Wrap,
};
use ratatui::widgets::{Padding, Widget};
use strum::IntoEnumIterator;
use tinyqoi::Qoi;
use tui_widgets::big_text::{BigText, PixelSize};

/// Application state.
///
/// Here you can store any state you need for your application.
#[derive(Default, Clone)]
pub struct AppState {
    /// Tracks the last button that was pressed.
    button_pressed: Option<Button>,
    screen: Screen,
    telemetry: Option<TelemetryFrame>,
}

#[derive(Default, Clone)]
pub struct UiState {
    screen: Screen,
    telemetry: Option<TelemetryFrame>,
}

#[derive(Default, Clone)]
pub struct MaraUi {
    pub state: UiState,
}

/// The main application trait that you need to implement.
impl MaraUiApp for MaraUi {
    /// Draw the UI frame.
    /// This is being called in the main loop to render the UI.
    fn draw(&self, frame: &mut Frame) {
        let titles = Screen::iter().map(|s| s.to_string());
        let selected_tab_index = self.state.screen as usize;

        let layout = Layout::default()
            .direction(Direction::Vertical)
            .spacing(1)
            .constraints([Constraint::Length(1), Constraint::Fill(1)])
            .split(frame.area());

        Tabs::new(titles)
            .style(Style::default().cyan().underlined())
            .highlight_style(Style::default().yellow())
            .select(selected_tab_index)
            .divider(symbols::DOT)
            .padding(" ", " ")
            .render(layout[0], frame.buffer_mut());

        self.render_tab_content(layout[1], frame);
    }

    /// Handle button press events.
    fn handle_press(&mut self, button: Button) {
        match button {
            Button::Button1(ButtonPressType::Short) => self.next_tab(),
            Button::Button2(ButtonPressType::Short) => self.previous_tab(),
            _ => {}
        }
    }

    fn next_tab(&mut self) {
        self.state.screen = self.state.screen.next();
    }

    fn previous_tab(&mut self) {
        self.state.screen = self.state.screen.previous();
    }

    fn update_telemetry(&mut self, telemetry: TelemetryFrame) {
        self.state.telemetry = Some(telemetry);
    }
}

impl MaraUi {
    fn render_tab_content(&self, area: Rect, frame: &mut Frame) {
        match self.state.screen {
            Screen::Main => Rat::new().render(area, frame),
            Screen::Dashboard => Dashboard::default().render(area, frame),
            // Screen::Graphs => self.render_graphs(area, frame),
            // Screen::Debug => self.render_debug(area, frame.buffer_mut()),
            _ => {}
        }
    }

    fn demo_dataset() -> Vec<(f64, f64)> {
        let mut data = Vec::new();

        for i in 0..360 {
            let x = i as f64;
            let y = (i / 2) as f64;
            data.push((x, y));
        }

        data
    }

    fn demo_dataset2() -> Vec<(f64, f64)> {
        let mut data = Vec::new();

        for i in 0..360 {
            let x = i as f64;
            let y = 128 as f64;
            data.push((x, y));
        }

        data
    }

    fn render_graphs(&self, area: Rect, frame: &mut Frame) {
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
    fn render_debug(&self, area: Rect, buf: &mut Buffer) {
        let text = match &self.state.telemetry {
            Some(telemetry) => format!("UART: {}", telemetry.raw_string),
            None => "UART: No data (waiting for connection...)".to_string(),
        };

        Paragraph::new(vec![Line::from(text)]).render(area, buf);
    }
}

fn main() {
    MaraUi::default().run()
}
