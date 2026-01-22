use maratui::button::{Button, ButtonPressType};
use maratui::qoi_widget::QoiImage;
use maratui::setup::MaraUiApp;
use maratui::telemetry::TelemetryFrame;
use maratui::telemetry::telemetry::parse_uart_line;
use mousefood::prelude::*;
use mousefood::ratatui::layout::{Direction, Layout};
use mousefood::ratatui::widgets::{
    Axis, Block, Chart, Dataset, GraphType, LegendPosition, Paragraph, Tabs, Wrap,
};
use strum::IntoEnumIterator;
use strum_macros::{Display, EnumIter, FromRepr};
use tinyqoi::Qoi;
use tui_big_text::{BigText, PixelSize};

#[derive(Clone, Copy, Default, Display, FromRepr, PartialEq, EnumIter)]
enum Screen {
    #[default]
    #[strum(to_string = "Main")]
    Main,
    #[strum(to_string = "Dashboard")]
    Dashboard,
    #[strum(to_string = "Graphs")]
    Graphs,
    #[strum(to_string = "Debug")]
    Debug,
}

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
    ///
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
            Screen::Main => self.render_main(area, frame.buffer_mut()),
            Screen::Dashboard => self.render_dashboard(area, frame),
            Screen::Graphs => self.render_graphs(area, frame),
            Screen::Debug => self.render_debug(area, frame.buffer_mut()),
        }
    }

    fn render_main(&self, area: Rect, buf: &mut Buffer) {
        // Layout: pixel rat on left, text info on right
        let layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
            .split(area);

        // Left area: render QOI image as Ratatui widget
        let rat_area = layout[0];
        let data = include_bytes!("../assets/rat_chef.qoi");
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
            .block(
                Block::default()
                    .padding(mousefood::ratatui::widgets::block::Padding::new(1, 0, 1, 0)),
            )
            .render(area, buf);
    }

    fn render_dashboard(&self, area: Rect, frame: &mut Frame) {
        let buf = frame.buffer_mut();
        let t_frame = parse_uart_line("C1.10,037,138,035,0000,1,0").expect("Not parsed!");
        let t_frame_low_water = parse_uart_line("C1.10,037,L65,035,0000,0,0").expect("Not parsed!");

        let [col1, col2, col3] = Layout::horizontal([Constraint::Fill(1); 3]).areas(area);

        let col1_areas = Layout::vertical([Constraint::Fill(1); 3]).split(col1);
        let col3_areas = Layout::horizontal([Constraint::Fill(1); 3]).split(col3);

        if let Some(tt) = t_frame.boiler_target_c {
            Paragraph::new(Line::from(format!("Target {tt}")))
                .wrap(Wrap { trim: true })
                .centered()
                .block(Block::default())
                .render(col1_areas[0], buf);
            Paragraph::new(Line::from(format!("Current {}", t_frame.boiler_now_c)))
                .wrap(Wrap { trim: true })
                .centered()
                .block(Block::default())
                .render(col1_areas[1], buf);
            Paragraph::new(Line::from(format!("Current HX {}", t_frame.hx_now_c)))
                .wrap(Wrap { trim: true })
                .centered()
                .block(Block::default())
                .render(col1_areas[2], buf);
        }

        let big_text = BigText::builder()
            .pixel_size(PixelSize::HalfWidth)
            .centered()
            .style(Style::default().yellow())
            .lines(vec!["138".into()])
            .build();

        Paragraph::new(Line::from(format!("Target {}", t_frame.mode.to_string())))
            .wrap(Wrap { trim: true })
            .centered()
            .block(Block::default())
            .render(col1_areas[0], buf);
        Paragraph::new(Line::from(format!("Heating {}", t_frame.heating_on)))
            .wrap(Wrap { trim: true })
            .centered()
            .block(Block::default())
            .render(col1_areas[1], buf);
        Paragraph::new(Line::from(format!("Pump {}", t_frame.pump_on)))
            .wrap(Wrap { trim: true })
            .centered()
            .block(Block::default())
            .render(col1_areas[2], buf);

        frame.render_widget(big_text, col2);
    }

    fn demo_dataset() -> Vec<(f64, f64)> {
        let mut data = Vec::new();

        for i in 0..360 {
            let x = i as f64;
            let y = (i / 2) as f64; // псевдо-температура
            data.push((x, y));
        }

        data
    }

    fn demo_dataset2() -> Vec<(f64, f64)> {
        let mut data = Vec::new();

        for i in 0..360 {
            let x = i as f64;
            let y = 128 as f64; // псевдо-температура
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
        let uart_line = "C1.10,037,L65,035,0000,0,0";

        Paragraph::new(vec![Line::from(format!("UART: {}", uart_line))]).render(area, buf);
    }
}

impl Screen {
    fn previous(self) -> Self {
        let current_index: usize = self as usize;
        let total = Screen::iter().count();
        let previous_index = if current_index == 0 {
            total - 1 // Wrap to last
        } else {
            current_index - 1
        };
        Self::from_repr(previous_index).unwrap_or(self)
    }

    fn next(self) -> Self {
        let current_index = self as usize;
        let total = Screen::iter().count();
        let next_index = (current_index + 1) % total; // Wrap to 0 after last
        Self::from_repr(next_index).unwrap_or(self)
    }
}

fn main() {
    MaraUi::default().run()
}
