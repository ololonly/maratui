use crate::button::Button;
use crate::run_app;
use crate::screens::{Board, Dashboard, Debug, Graphs, Rat, Screen};
use crate::state::{AppStateMachine, GlobalAppState};
use crate::telemetry::TelemetryFrame;
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::Style;
use ratatui::symbols;
use ratatui::widgets::Tabs;
use ratatui::widgets::Widget;
use std::time::Instant;
use strum::IntoEnumIterator;

/// Application trait to be implemented by the user
pub trait MaraUiApp {
    /// Draw the UI frame
    fn draw(&self, frame: &mut Frame);

    /// Handle button press events
    fn handle_press(&mut self, button: Button);

    /// Update application state with telemetry data
    fn update_telemetry(&mut self, telemetry: TelemetryFrame);

    /// Run the application
    ///
    /// Default implementation provided. Do not override unless necessary.
    fn run(self)
    where
        Self: Sized,
    {
        run_app(self);
    }
}

/// Main application structure
#[derive(Default)]
pub struct MaraUi {
    /// Global application state
    pub state: GlobalAppState,
}

/// The main application implementation
impl MaraUiApp for MaraUi {
    /// Draw the UI frame
    /// This is being called in the main loop to render the UI
    fn draw(&self, frame: &mut Frame) {
        let titles = Screen::iter().map(|s| s.to_string());
        let selected_tab_index = self.state.current_screen as usize;

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

    /// Handle button press events
    fn handle_press(&mut self, button: Button) {
        AppStateMachine::handle_button_press(&mut self.state, button);
    }

    /// Update application state with telemetry data
    fn update_telemetry(&mut self, telemetry: TelemetryFrame) {
        let now = Instant::now();
        AppStateMachine::handle_telemetry_frame(&mut self.state, telemetry, now);
    }
}

impl MaraUi {
    /// Render the content of the current screen
    fn render_tab_content(&self, area: Rect, frame: &mut Frame) {
        match self.state.current_screen {
            Screen::Main => Rat::render(&self.state, area, frame),
            Screen::Dashboard => Dashboard::render(&self.state, area, frame),
            Screen::Graphs => Graphs::render(&self.state, area, frame),
            Screen::Debug => Debug::render(&self.state, area, frame),
        }
    }
}
