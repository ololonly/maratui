use crate::telemetry::AppEvent as TelemetryEvent;

use super::{ConnectionStatus, DeviceInfo};

/// Application events
/// Combines telemetry events and UI events
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AppEvent {
    // ===== Telemetry Events =====
    /// Pump turned on, extraction started
    ShotStarted,

    /// Pump turned off, extraction ended
    ShotEnded { duration: u64 },

    /// Pump ran for less than the minimum shot duration (rinsing / post-heat pump kick)
    ShotAborted { duration: u64 },

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

    /// Board metadata update (WiFi RSSI, IP, uptime, free heap)
    DeviceInfoUpdated(DeviceInfo),

    /// Boot initialization stage — shown on the connecting screen with a progress bar
    LoadingStage { message: &'static str, progress: u8 },

    /// Boot initialization complete — switches connecting screen to "waiting for machine"
    LoadingComplete,
}

impl AppEvent {
    /// Convert a telemetry event to an application event
    pub fn from_telemetry(event: TelemetryEvent) -> Self {
        match event {
            TelemetryEvent::ShotStarted => AppEvent::ShotStarted,
            TelemetryEvent::ShotEnded { duration } => AppEvent::ShotEnded { duration },
            TelemetryEvent::ShotAborted { duration } => AppEvent::ShotAborted { duration },
            TelemetryEvent::ModeChanged { from, to } => AppEvent::ModeChanged { from, to },
            TelemetryEvent::WaterRefillNeeded { code } => AppEvent::WaterRefillNeeded { code },
            TelemetryEvent::WaterRefillCleared => AppEvent::WaterRefillCleared,
        }
    }

    /// Check if the event is a telemetry event (shot/mode/water transitions from the machine).
    /// Only these events are written to the on-screen events log.
    pub fn is_telemetry_event(&self) -> bool {
        matches!(
            self,
            AppEvent::ShotStarted
                | AppEvent::ShotEnded { .. }
                | AppEvent::ShotAborted { .. }
                | AppEvent::ModeChanged { .. }
                | AppEvent::WaterRefillNeeded { .. }
                | AppEvent::WaterRefillCleared
                | AppEvent::CupCounterUpdated { .. }
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
            AppEvent::ShotAborted { duration } => {
                write!(f, "Shot Aborted ({} s)", duration)
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
            AppEvent::DeviceInfoUpdated(info) => {
                write!(f, "Device info updated (uptime={}s)", info.uptime_s)
            }
            AppEvent::LoadingStage { message, progress } => {
                write!(f, "Loading [{progress}%]: {message}")
            }
            AppEvent::LoadingComplete => write!(f, "Loading complete"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::telemetry::MachineMode;

    #[test]
    fn test_app_event_from_telemetry() {
        let tel_event = TelemetryEvent::ShotStarted;
        let app_event = AppEvent::from_telemetry(tel_event);
        assert_eq!(app_event, AppEvent::ShotStarted);
    }

    #[test]
    fn test_from_telemetry_all_variants() {
        assert_eq!(
            AppEvent::from_telemetry(TelemetryEvent::ShotEnded { duration: 30 }),
            AppEvent::ShotEnded { duration: 30 }
        );
        assert_eq!(
            AppEvent::from_telemetry(TelemetryEvent::ShotAborted { duration: 5 }),
            AppEvent::ShotAborted { duration: 5 }
        );
        assert_eq!(
            AppEvent::from_telemetry(TelemetryEvent::ModeChanged {
                from: MachineMode::Coffee,
                to: MachineMode::SteamS
            }),
            AppEvent::ModeChanged {
                from: MachineMode::Coffee,
                to: MachineMode::SteamS
            }
        );
        assert_eq!(
            AppEvent::from_telemetry(TelemetryEvent::WaterRefillNeeded { code: 65 }),
            AppEvent::WaterRefillNeeded { code: 65 }
        );
        assert_eq!(
            AppEvent::from_telemetry(TelemetryEvent::WaterRefillCleared),
            AppEvent::WaterRefillCleared
        );
    }

    #[test]
    fn test_app_event_is_telemetry_event() {
        assert!(AppEvent::ShotStarted.is_telemetry_event());
        assert!(AppEvent::ShotEnded { duration: 30 }.is_telemetry_event());
        assert!(AppEvent::ShotAborted { duration: 5 }.is_telemetry_event());
        assert!(AppEvent::WaterRefillNeeded { code: 65 }.is_telemetry_event());
        assert!(AppEvent::WaterRefillCleared.is_telemetry_event());
        assert!(AppEvent::ModeChanged {
            from: MachineMode::Coffee,
            to: MachineMode::SteamS
        }
        .is_telemetry_event());
        assert!(AppEvent::CupCounterUpdated { cups: 1 }.is_telemetry_event());

        // Infrastructure events must NOT appear in the telemetry log
        assert!(
            !AppEvent::WifiStatusChanged(crate::state::ConnectionStatus::Connected)
                .is_telemetry_event()
        );
        assert!(
            !AppEvent::MqttStatusChanged(crate::state::ConnectionStatus::Connected)
                .is_telemetry_event()
        );
        assert!(
            !AppEvent::PublishMqttEvent {
                topic_suffix: "t".into(),
                payload: "p".into()
            }
            .is_telemetry_event()
        );
        assert!(!AppEvent::NextScreen.is_telemetry_event());
        assert!(!AppEvent::PreviousScreen.is_telemetry_event());
        assert!(!AppEvent::DebugScreen.is_telemetry_event());
        assert!(!AppEvent::ErrorOccurred { error: "e".into() }.is_telemetry_event());
        assert!(!AppEvent::ErrorCleared.is_telemetry_event());
        assert!(!AppEvent::LoadingComplete.is_telemetry_event());
    }

    #[test]
    fn test_app_event_is_ui_event() {
        assert!(AppEvent::NextScreen.is_ui_event());
        assert!(AppEvent::PreviousScreen.is_ui_event());
        assert!(AppEvent::DebugScreen.is_ui_event());
        assert!(AppEvent::ErrorOccurred { error: "oops".into() }.is_ui_event());
        assert!(AppEvent::ErrorCleared.is_ui_event());

        // Non-UI events
        assert!(!AppEvent::ShotStarted.is_ui_event());
        assert!(!AppEvent::ShotEnded { duration: 30 }.is_ui_event());
        assert!(!AppEvent::WaterRefillCleared.is_ui_event());
        assert!(!AppEvent::CupCounterUpdated { cups: 1 }.is_ui_event());
        assert!(
            !AppEvent::WifiStatusChanged(crate::state::ConnectionStatus::Connected).is_ui_event()
        );
        assert!(!AppEvent::LoadingComplete.is_ui_event());
    }

    #[test]
    fn test_app_event_display() {
        assert_eq!(AppEvent::ShotStarted.to_string(), "Shot Started");
        assert_eq!(
            AppEvent::ShotEnded { duration: 42 }.to_string(),
            "Shot Ended (42 s)"
        );
        assert_eq!(
            AppEvent::ShotAborted { duration: 3 }.to_string(),
            "Shot Aborted (3 s)"
        );
        assert_eq!(
            AppEvent::ModeChanged {
                from: MachineMode::Coffee,
                to: MachineMode::SteamS
            }
            .to_string(),
            "Mode Changed: Coffee → Steam"
        );
        assert_eq!(
            AppEvent::WaterRefillNeeded { code: 7 }.to_string(),
            "Water Refill Needed (code: 7)"
        );
        assert_eq!(
            AppEvent::WaterRefillCleared.to_string(),
            "Water Refill Cleared"
        );
        assert_eq!(AppEvent::NextScreen.to_string(), "Next Screen");
        assert_eq!(AppEvent::PreviousScreen.to_string(), "Previous Screen");
        assert_eq!(AppEvent::DebugScreen.to_string(), "Debug Screen");
        assert_eq!(
            AppEvent::ErrorOccurred {
                error: "boom".into()
            }
            .to_string(),
            "Error: boom"
        );
        assert_eq!(AppEvent::ErrorCleared.to_string(), "Error Cleared");
        assert_eq!(
            AppEvent::WifiStatusChanged(crate::state::ConnectionStatus::Connected).to_string(),
            "Wi-Fi status: Connected"
        );
        assert_eq!(
            AppEvent::MqttStatusChanged(crate::state::ConnectionStatus::Disconnected).to_string(),
            "MQTT status: Disconnected"
        );
        assert_eq!(
            AppEvent::CupCounterUpdated { cups: 5 }.to_string(),
            "Cup counter updated: 5"
        );
        assert_eq!(
            AppEvent::PublishMqttEvent {
                topic_suffix: "evt".into(),
                payload: "{}".into()
            }
            .to_string(),
            "Publish MQTT event: suffix='evt' payload='{}'"
        );
        assert_eq!(
            AppEvent::DeviceInfoUpdated(crate::state::DeviceInfo {
                uptime_s: 120,
                ..Default::default()
            })
            .to_string(),
            "Device info updated (uptime=120s)"
        );
        assert_eq!(
            AppEvent::LoadingStage {
                message: "Connecting",
                progress: 50
            }
            .to_string(),
            "Loading [50%]: Connecting"
        );
        assert_eq!(AppEvent::LoadingComplete.to_string(), "Loading complete");
    }
}
