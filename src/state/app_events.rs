use crate::telemetry::AppEvent as TelemetryEvent;

use super::ConnectionStatus;

/// Application events
/// Combines telemetry events and UI events
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AppEvent {
    // ===== Telemetry Events =====
    /// Pump turned on, extraction started
    ShotStarted,

    /// Pump turned off, extraction ended
    ShotEnded { duration: u64 },

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

    /// Wi-Fi status changed
    WifiStatusChanged(ConnectionStatus),

    /// MQTT status changed
    MqttStatusChanged(ConnectionStatus),

    /// Cup counter value updated from MQTT retained/incoming message
    CupCounterUpdated { cups: u64 },

    /// Request publish custom app event to MQTT
    PublishMqttEvent {
        topic_suffix: String,
        payload: String,
    },
}

impl AppEvent {
    /// Convert a telemetry event to an application event
    pub fn from_telemetry(event: TelemetryEvent) -> Self {
        match event {
            TelemetryEvent::ShotStarted => AppEvent::ShotStarted,
            TelemetryEvent::ShotEnded { duration } => AppEvent::ShotEnded { duration },
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
                | AppEvent::WifiStatusChanged(..)
                | AppEvent::MqttStatusChanged(..)
                | AppEvent::CupCounterUpdated { .. }
                | AppEvent::PublishMqttEvent { .. }
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
            AppEvent::ShotEnded { duration } => {
                write!(f, "Shot Ended ({} s)", duration)
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
            AppEvent::WifiStatusChanged(status) => write!(f, "Wi-Fi status: {:?}", status),
            AppEvent::MqttStatusChanged(status) => write!(f, "MQTT status: {:?}", status),
            AppEvent::CupCounterUpdated { cups } => write!(f, "Cup counter updated: {}", cups),
            AppEvent::PublishMqttEvent {
                topic_suffix,
                payload,
            } => write!(
                f,
                "Publish MQTT event: suffix='{}' payload='{}'",
                topic_suffix, payload
            ),
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
