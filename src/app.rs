use crate::brew_timer::BrewTimer;
use crate::button::{Button, ButtonPressType};
use crate::run_app;
use crate::screens::{Board, Dashboard, Debug, Graphs, Rat, Screen};
use crate::telemetry::TelemetryFrame;
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::Style;
use ratatui::symbols;
use ratatui::widgets::Tabs;
use ratatui::widgets::Widget;
use strum::IntoEnumIterator;

/// Application trait to be implemented by the user.
pub trait MaraUiApp {
    /// Draw the UI frame.
    fn draw(&self, frame: &mut Frame);

    /// Handle button press events.
    fn handle_press(&mut self, button: Button);

    fn next_tab(&mut self);
    fn previous_tab(&mut self);

    fn update_telemetry(&mut self, telemetry: TelemetryFrame);

    /// Run the application.
    ///
    /// Default implementation provided. Do not override unless necessary.
    fn run(self)
    where
        Self: Sized,
    {
        run_app(self);
    }
}

#[derive(Default, Clone)]
pub struct AppState {
    screen: Screen,
    brew_timer: BrewTimer,
    telemetry: Option<TelemetryFrame>,
}

#[derive(Default, Clone)]
pub struct MaraUi {
    pub state: AppState,
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
        if let Some(prev) = &self.state.telemetry {
            if !prev.pump_on && telemetry.pump_on {
                self.state.brew_timer.start();
            }

            if prev.pump_on && !telemetry.pump_on {
                self.state.brew_timer.stop();
            }
        }

        self.state.telemetry = Some(telemetry);
    }
}

impl MaraUi {
    fn render_tab_content(&self, area: Rect, frame: &mut Frame) {
        match self.state.screen {
            Screen::Main => Rat::new().render(area, frame),
            Screen::Dashboard => {
                #[cfg(not(feature = "simulator"))]
                Dashboard::new(&self.state.telemetry, self.state.brew_timer).render(area, frame);
                #[cfg(feature = "simulator")]
                Dashboard::default().render(area, frame);
            }
            Screen::Graphs => Graphs::default().render(area, frame),
            Screen::Debug => Debug::new(&self.state.telemetry).render(area, frame),
        }
    }
}
