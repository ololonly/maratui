use core::fmt;
use std::time::Instant;

/// Type of button press: short or long.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonPressType {
    Short,
    Long,
}

impl fmt::Display for ButtonPressType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ButtonPressType::Short => write!(f, "Short Press"),
            ButtonPressType::Long => write!(f, "Long Press"),
        }
    }
}

/// Button enum representing different button actions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Button {
    Button1(ButtonPressType),
}

impl fmt::Display for Button {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Button::Button1(press_type) => write!(f, "Button 1 ({press_type})"),
        }
    }
}

impl Button {
    /// Check if a short press was detected.
    pub fn is_short_press(&self) -> bool {
        matches!(self, Button::Button1(ButtonPressType::Short))
    }

    /// Check if a long press was detected.
    pub fn is_long_press(&self) -> bool {
        matches!(self, Button::Button1(ButtonPressType::Long))
    }
}

/// State of a button, tracking press duration.
#[derive(Default)]
pub struct ButtonState {
    pressed_at: Option<Instant>,
}

impl ButtonState {
    /// Update the button state based on whether it is currently pressed.
    ///
    /// If the button was just released, it calls the `on_press` callback with the type of press
    /// detected.
    pub fn update<F>(&mut self, is_pressed: bool, on_press: F)
    where
        F: FnOnce(ButtonPressType),
    {
        if is_pressed {
            // Button is currently down
            if self.pressed_at.is_none() {
                self.pressed_at = Some(Instant::now());
            }
        } else if let Some(pressed_at) = self.pressed_at.take() {
            // Button just released
            let duration = pressed_at.elapsed().as_millis() as u64;
            let press_type = if duration < 500 {
                ButtonPressType::Short
            } else {
                ButtonPressType::Long
            };
            on_press(press_type);
        }
    }
}
