use ratatui::Frame;
use ratatui::layout::Rect;
use strum::IntoEnumIterator;
use strum::{Display, EnumIter, FromRepr};

#[derive(Clone, Copy, Default, Display, FromRepr, PartialEq, EnumIter)]
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
    pub fn previous(self) -> Self {
        let current_index: usize = self as usize;
        let total = Screen::iter().count();
        let previous_index = if current_index == 0 {
            total - 1 // Wrap to last
        } else {
            current_index - 1
        };
        Self::from_repr(previous_index).unwrap_or(self)
    }

    pub fn next(self) -> Self {
        let current_index = self as usize;
        let total = Screen::iter().count();
        let next_index = (current_index + 1) % total; // Wrap to 0 after last
        Self::from_repr(next_index).unwrap_or(self)
    }
}

pub trait Board {
    fn render(&self, area: Rect, frame: &mut Frame);
}
