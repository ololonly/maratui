use crate::state::GlobalAppState;
use ratatui::Frame;
use ratatui::layout::Rect;
use strum::IntoEnumIterator;
use strum::{Display, EnumIter, FromRepr};

/// Available screens in the application
#[derive(Clone, Copy, Default, Display, FromRepr, PartialEq, Eq, Debug, EnumIter)]
pub enum Screen {
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

impl Screen {
    /// Navigate to the previous screen (wraps around)
    pub fn previous(self) -> Self {
        let current_index: usize = self as usize;
        let total = Screen::iter().count();
        let previous_index = if current_index == 0 {
            total - 1
        } else {
            current_index - 1
        };
        Self::from_repr(previous_index).unwrap_or(self)
    }

    /// Navigate to the next screen (wraps around)
    pub fn next(self) -> Self {
        let current_index = self as usize;
        let total = Screen::iter().count();
        let next_index = (current_index + 1) % total;
        Self::from_repr(next_index).unwrap_or(self)
    }
}

/// Trait for rendering screen content
pub trait Board {
    /// Render the screen with the given application state
    fn render(state: &GlobalAppState, area: Rect, frame: &mut Frame);
}
