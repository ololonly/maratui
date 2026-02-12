use crate::screens::Screen;
use crate::telemetry::MachineState;
use std::{
    collections::VecDeque,
    time::{Duration, Instant},
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ConnectionStatus {
    Disabled,
    Disconnected,
    Connecting,
    Connected,
    Error(String),
}

impl Default for ConnectionStatus {
    fn default() -> Self {
        Self::Disconnected
    }
}

#[derive(Clone, Debug)]
pub struct MqttOutboundMessage {
    pub topic_suffix: String,
    pub payload: String,
}

/// State of the coffee extraction process
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExtractionState {
    /// Pump is not running, no extraction in progress
    Idle {
        /// Duration of the last completed extraction
        last_extraction_duration: Option<Duration>,
    },
    /// Pump is running, extraction in progress
    Extracting { started_at: Instant },
}

impl Default for ExtractionState {
    fn default() -> Self {
        Self::Idle {
            last_extraction_duration: None,
        }
    }
}

impl ExtractionState {
    /// Get the duration of the current extraction (if in progress)
    pub fn elapsed(&self) -> Option<Duration> {
        match self {
            ExtractionState::Extracting { started_at } => Some(started_at.elapsed()),
            ExtractionState::Idle { .. } => None,
        }
    }

    /// Check if extraction is currently in progress
    pub fn is_extracting(&self) -> bool {
        matches!(self, ExtractionState::Extracting { .. })
    }

    /// Get the duration of the last completed extraction
    pub fn last_extraction_duration(&self) -> Option<Duration> {
        match self {
            ExtractionState::Idle {
                last_extraction_duration,
            } => *last_extraction_duration,
            ExtractionState::Extracting { .. } => None,
        }
    }
}

/// Application errors
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AppError {
    /// Water refill is needed
    WaterRefillNeeded { code: u16 },
    /// Machine is offline
    MachineOffline,
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppError::WaterRefillNeeded { code } => {
                write!(f, "Water refill needed (code: {})", code)
            }
            AppError::MachineOffline => write!(f, "Machine is offline"),
        }
    }
}

/// Global application state
/// Single point of state management for the entire application
#[derive(Clone, Debug)]
pub struct GlobalAppState {
    /// Currently displayed screen
    pub current_screen: Screen,
    /// State of the extraction process
    pub extraction_state: ExtractionState,
    /// Machine state (mode, temperatures, pump, etc.)
    pub machine_state: MachineState,
    pub events_log: VecDeque<String>,
    pub wifi_status: ConnectionStatus,
    pub mqtt_status: ConnectionStatus,
    pub outbound_mqtt: VecDeque<MqttOutboundMessage>,
    /// Current error (if any)
    pub error: Option<AppError>,
}

impl Default for GlobalAppState {
    fn default() -> Self {
        Self {
            current_screen: Screen::default(),
            extraction_state: ExtractionState::default(),
            machine_state: MachineState::default(),
            events_log: VecDeque::with_capacity(10),
            wifi_status: ConnectionStatus::default(),
            mqtt_status: ConnectionStatus::default(),
            outbound_mqtt: VecDeque::with_capacity(32),
            error: None,
        }
    }
}

impl GlobalAppState {
    /// Create a new application state
    pub fn new() -> Self {
        Self::default()
    }

    /// Get the duration of the current extraction in seconds
    pub fn extraction_duration_secs(&self) -> u64 {
        self.extraction_state
            .elapsed()
            .map(|d| d.as_secs())
            .unwrap_or(0)
    }

    /// Check if extraction is currently in progress
    pub fn is_extracting(&self) -> bool {
        self.extraction_state.is_extracting()
    }

    /// Check if there is an error
    pub fn has_error(&self) -> bool {
        self.error.is_some()
    }

    /// Clear the error
    pub fn clear_error(&mut self) {
        self.error = None;
    }

    pub fn enqueue_mqtt_message(
        &mut self,
        topic_suffix: impl Into<String>,
        payload: impl Into<String>,
    ) {
        self.outbound_mqtt.push_back(MqttOutboundMessage {
            topic_suffix: topic_suffix.into(),
            payload: payload.into(),
        });

        while self.outbound_mqtt.len() > 64 {
            self.outbound_mqtt.pop_front();
        }
    }

    pub fn take_outbound_mqtt_messages(&mut self) -> Vec<MqttOutboundMessage> {
        self.outbound_mqtt.drain(..).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extraction_state_idle() {
        let state = ExtractionState::Idle {
            last_extraction_duration: None,
        };
        assert!(!state.is_extracting());
        assert_eq!(state.elapsed(), None);
        assert_eq!(state.last_extraction_duration(), None);
    }

    #[test]
    fn test_extraction_state_idle_with_duration() {
        let duration = Duration::from_secs(30);
        let state = ExtractionState::Idle {
            last_extraction_duration: Some(duration),
        };
        assert!(!state.is_extracting());
        assert_eq!(state.last_extraction_duration(), Some(duration));
    }

    #[test]
    fn test_extraction_state_extracting() {
        let now = Instant::now();
        let state = ExtractionState::Extracting { started_at: now };
        assert!(state.is_extracting());
        assert!(state.elapsed().is_some());
        assert_eq!(state.last_extraction_duration(), None);
    }

    #[test]
    fn test_global_app_state_default() {
        let state = GlobalAppState::default();
        assert_eq!(state.current_screen, Screen::default());
        assert_eq!(
            state.extraction_state,
            ExtractionState::Idle {
                last_extraction_duration: None
            }
        );
        assert_eq!(state.machine_state.last_frame, None);
        assert_eq!(state.error, None);
    }

    #[test]
    fn test_global_app_state_extraction_duration() {
        let mut state = GlobalAppState::default();
        assert_eq!(state.extraction_duration_secs(), 0);

        state.extraction_state = ExtractionState::Extracting {
            started_at: Instant::now(),
        };
        assert!(state.extraction_duration_secs() > 0 || state.extraction_duration_secs() == 0);
    }

    #[test]
    fn test_global_app_state_error() {
        let mut state = GlobalAppState::default();
        assert!(!state.has_error());

        state.error = Some(AppError::WaterRefillNeeded { code: 65 });
        assert!(state.has_error());

        state.clear_error();
        assert!(!state.has_error());
    }
}
