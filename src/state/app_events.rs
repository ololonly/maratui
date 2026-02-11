use crate::telemetry::AppEvent as TelemetryEvent;

/// Application events
/// Combines telemetry events and UI events
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AppEvent {
    // ===== Telemetry Events =====
    /// Pump turned on, extraction started
    ShotStarted,

    /// Pump turned off, extraction ended
    ShotEnded,

    /// Machine mode changed
    ModeChanged {
        from: crate::telemetry::MachineMode,
        to: crate::telemetry::MachineMode,
    },

    /// Water refill is needed
    WaterRefillNeeded { code: u16 },

    /// Water refilled, error cleared
    WaterRefillCleared,

    // ===== UI Events =====
    /// Switch to the next screen
    NextScreen,

    /// Switch to the previous screen
    PreviousScreen,

    /// Switch to the debug screen
    DebugScreen,

    /// An error occurred
    ErrorOccurred { error: String },

    /// Error cleared
    ErrorCleared,
}

impl AppEvent {
    /// Convert a telemetry event to an application event
    pub fn from_telemetry(event: TelemetryEvent) -> Self {
        match event {
            TelemetryEvent::ShotStarted => AppEvent::ShotStarted,
            TelemetryEvent::ShotEnded => AppEvent::ShotEnded,
            TelemetryEvent::ModeChanged { from, to } => AppEvent::ModeChanged { from, to },
            TelemetryEvent::WaterRefillNeeded { code } => AppEvent::WaterRefillNeeded { code },
            TelemetryEvent::WaterRefillCleared => AppEvent::WaterRefillCleared,
        }
    }

    /// Check if the event is a telemetry event
    pub fn is_telemetry_event(&self) -> bool {
        matches!(
            self,
            AppEvent::ShotStarted
                | AppEvent::ShotEnded { .. }
                | AppEvent::ModeChanged { .. }
                | AppEvent::WaterRefillNeeded { .. }
                | AppEvent::WaterRefillCleared
        )
    }

    /// Check if the event is a UI event
    pub fn is_ui_event(&self) -> bool {
        matches!(
            self,
            AppEvent::NextScreen
                | AppEvent::PreviousScreen
                | AppEvent::DebugScreen
                | AppEvent::ErrorOccurred { .. }
                | AppEvent::ErrorCleared
        )
    }
}

impl std::fmt::Display for AppEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppEvent::ShotStarted => write!(f, "Shot Started"),
            AppEvent::ShotEnded => {
                write!(f, "Shot Ended")
            }
            AppEvent::ModeChanged { from, to } => {
                write!(f, "Mode Changed: {} → {}", from, to)
            }
            AppEvent::WaterRefillNeeded { code } => {
                write!(f, "Water Refill Needed (code: {})", code)
            }
            AppEvent::WaterRefillCleared => write!(f, "Water Refill Cleared"),
            AppEvent::NextScreen => write!(f, "Next Screen"),
            AppEvent::PreviousScreen => write!(f, "Previous Screen"),
            AppEvent::DebugScreen => write!(f, "Debug Screen"),
            AppEvent::ErrorOccurred { error } => write!(f, "Error: {}", error),
            AppEvent::ErrorCleared => write!(f, "Error Cleared"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_event_from_telemetry() {
        let tel_event = TelemetryEvent::ShotStarted;
        let app_event = AppEvent::from_telemetry(tel_event);
        assert_eq!(app_event, AppEvent::ShotStarted);
    }

    #[test]
    fn test_app_event_is_telemetry_event() {
        assert!(AppEvent::ShotStarted.is_telemetry_event());
        assert!(AppEvent::NextScreen.is_ui_event());
    }

    #[test]
    fn test_app_event_display() {
        let event = AppEvent::ShotStarted;
        assert_eq!(event.to_string(), "Shot Started");
    }
}
