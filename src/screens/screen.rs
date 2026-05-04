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
    fn rotatable() -> Vec<Self> {
        Screen::iter().filter(|s| *s != Screen::Debug).collect()
    }

    /// Navigate to the previous screen (wraps around)
    pub fn previous(self) -> Self {
        let screens = Self::rotatable();
        let pos = screens.iter().position(|s| *s == self);
        match pos {
            Some(0) => screens[screens.len() - 1],
            Some(i) => screens[i - 1],
            None => Screen::Main,
        }
    }

    /// Navigate to the next screen (wraps around)
    pub fn next(self) -> Self {
        let screens = Self::rotatable();
        let pos = screens.iter().position(|s| *s == self);
        match pos {
            Some(i) => screens[(i + 1) % screens.len()],
            None => Screen::Main,
        }
    }
}

/// Trait for rendering screen content
pub trait Board {
    /// Render the screen with the given application state
    fn render(state: &GlobalAppState, area: Rect, frame: &mut Frame);
}
