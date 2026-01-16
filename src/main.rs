use mousefood::prelude::*;
use mousefood::ratatui::widgets::{Block, Paragraph};
use ratatui_mousefood_template::button::Button;
use ratatui_mousefood_template::setup::App;

/// Application state.
///
/// Here you can store any state you need for your application.
#[derive(Default)]
pub struct AppState {
    /// Tracks the last button that was pressed.
    button_pressed: Option<Button>,
}

/// The main application trait that you need to implement.
impl App for AppState {
    /// Draw the UI frame.
    ///
    /// This is being called in the main loop to render the UI.
    fn draw(&self, frame: &mut Frame) {
        let text = Line::from("Ratatui on embedded devices!").dark_gray();

        let button_text = match self.button_pressed {
            Some(button) => Line::from(vec![
                "You pressed: ".dark_gray(),
                button.to_string().yellow(),
            ]),
            None => Line::from("Press a button...").dark_gray(),
        };

        let paragraph = Paragraph::new(Text::from(vec![text, button_text]));

        let bordered_block = Block::bordered()
            .border_style(Style::new().yellow())
            .title("Mousefood");

        frame.render_widget(paragraph.block(bordered_block), frame.area());
    }

    /// Handle button press events.
    fn handle_press(&mut self, button: Button) {
        self.button_pressed = Some(button);
    }
}

fn main() {
    AppState::default().run()
}
