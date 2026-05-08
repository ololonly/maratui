use crate::state::GlobalAppState;
use ratatui::Frame;
use ratatui::layout::Rect;
use strum::{Display, EnumIter, FromRepr};

/// Available screens in the application
#[derive(Clone, Copy, Default, Display, FromRepr, PartialEq, Eq, Debug, EnumIter)]
pub enum Screen {
    #[default]
    #[strum(to_string = "Dashboard")]
    Dashboard,
    #[strum(to_string = "Graphs")]
    Graphs,
    #[strum(to_string = "Debug")]
    Debug,
}

impl Screen {
    /// Toggle between Dashboard and Graphs (single-button navigation)
    pub fn next(self) -> Self {
        match self {
            Screen::Dashboard => Screen::Graphs,
            Screen::Graphs => Screen::Dashboard,
            Screen::Debug => Screen::Debug,
        }
    }

    /// Alias for next — with one button there is no separate "previous"
    pub fn previous(self) -> Self {
        self.next()
    }
}

/// Trait for rendering screen content
pub trait Board {
    /// Render the screen with the given application state
    fn render(state: &GlobalAppState, area: Rect, frame: &mut Frame);
}
