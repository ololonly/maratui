use mousefood::prelude::*;
use mousefood::ratatui::layout::Layout;
use mousefood::ratatui::widgets::{Block, Paragraph, Tabs};
use ratatui_mousefood_template::button::{Button, ButtonPressType};
use ratatui_mousefood_template::qoi_widget::QoiImage;
use ratatui_mousefood_template::setup::MaraUiApp;
use strum::IntoEnumIterator;
use strum_macros::{Display, EnumIter, FromRepr};
use tinyqoi::Qoi;

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
#[derive(Default, Clone, Copy)]
pub struct AppState {
    /// Tracks the last button that was pressed.
    button_pressed: Option<Button>,
    screen: Screen,
}

#[derive(Default, Clone, Copy)]
pub struct UiState {
    screen: Screen,
    // telemetry: Some,
}

#[derive(Default, Clone, Copy)]
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
            .direction(mousefood::ratatui::layout::Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Fill(1)])
            .split(frame.area());

        let tabs = Tabs::new(titles)
            .style(Style::default().cyan().underlined())
            .highlight_style(Style::default().yellow())
            .select(selected_tab_index)
            .divider(symbols::DOT)
            .padding(" ", " ")
            .render(layout[0], frame.buffer_mut());

        self.render_tab_content(layout[1], frame.buffer_mut());
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
}

impl MaraUi {
    fn render_tab_content(self, area: Rect, buf: &mut Buffer) {
        match self.state.screen {
            Screen::Main => self.render_main(area, buf),
            Screen::Dashboard => self.render_dashboard(area, buf),
            Screen::Graphs => self.render_graphs(area, buf),
            Screen::Debug => self.render_debug(area, buf),
        }
    }

    fn render_main(self, area: Rect, buf: &mut Buffer) {
        // Layout: pixel rat on left, text info on right
        let layout = Layout::default()
            .direction(mousefood::ratatui::layout::Direction::Horizontal)
            .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
            .split(area);

        // Left area: render QOI image as Ratatui widget
        let rat_area = layout[0];
        let data = include_bytes!("../assets/rat_chef.qoi");
        let qoi = Qoi::new(data).unwrap();
        QoiImage::new(&qoi).render(rat_area, buf);

        // Render status text on the right
        let status_area = layout[1];
        self.render_main_status(status_area, buf);
    }

    fn render_main_status(
        self,
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
                    .padding(mousefood::ratatui::widgets::block::Padding::new(0, 0, 1, 0)),
            )
            .render(area, buf);
    }

    fn render_dashboard(self, area: Rect, buf: &mut Buffer) {
        Paragraph::new("Dashboard").render(area, buf);
    }
    fn render_graphs(self, area: Rect, buf: &mut Buffer) {
        Paragraph::new("Graphs").render(area, buf);
    }
    fn render_debug(self, area: Rect, buf: &mut Buffer) {
        Paragraph::new("Debug").render(area, buf);
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
