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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_screen_next_cycles_dashboard_graphs() {
        assert_eq!(Screen::Dashboard.next(), Screen::Graphs);
        assert_eq!(Screen::Graphs.next(), Screen::Dashboard);
    }

    #[test]
    fn test_screen_next_debug_stays() {
        assert_eq!(Screen::Debug.next(), Screen::Debug);
    }

    #[test]
    fn test_screen_previous_mirrors_next() {
        assert_eq!(Screen::Dashboard.previous(), Screen::Dashboard.next());
        assert_eq!(Screen::Graphs.previous(), Screen::Graphs.next());
        assert_eq!(Screen::Debug.previous(), Screen::Debug.next());
    }

    #[test]
    fn test_screen_default_is_dashboard() {
        assert_eq!(Screen::default(), Screen::Dashboard);
    }
}
