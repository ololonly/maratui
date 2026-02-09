use log::info;

use crate::button::{Button, ButtonPressType};
use crate::telemetry::{TelemetryFrame, update_state_with_events};
use std::time::Instant;

use super::{AppError, AppEvent, ExtractionState, GlobalAppState};

/// Application state machine
/// Handles events and updates the global application state
pub struct AppStateMachine;

impl AppStateMachine {
    /// Handle an application event and update the state
    pub fn handle_event(state: &mut GlobalAppState, event: AppEvent) {
        info!("Handling event: {:?}", event);
        state.events_log.push_front(format!("Event: {:?}", event));
        if state.events_log.len() > 10 {
            state.events_log.pop_back();
        }
        match event {
            // ===== Telemetry Events =====
            AppEvent::ShotStarted => {
                state.extraction_state = ExtractionState::Extracting {
                    started_at: Instant::now(),
                };
                // Clear error when extraction starts
                state.error = None;
            }

            AppEvent::ShotEnded => {
                state.extraction_state = ExtractionState::Idle {
                    last_extraction_duration: state.extraction_state.elapsed(),
                };
            }

            AppEvent::ModeChanged { from: _, to: _ } => {
                // Machine mode is already updated in MachineState
                // Additional logic can be added here if needed
            }

            AppEvent::WaterRefillNeeded { code } => {
                state.error = Some(AppError::WaterRefillNeeded { code });
            }

            AppEvent::WaterRefillCleared => {
                state.error = None;
            }

            // ===== UI Events =====
            AppEvent::NextScreen => {
                state.current_screen = state.current_screen.next();
            }

            AppEvent::PreviousScreen => {
                state.current_screen = state.current_screen.previous();
            }

            AppEvent::ErrorOccurred { error } => {
                state.error = Some(AppError::MachineOffline);
                // Log the error (logging can be added here)
                eprintln!("Application error: {}", error);
            }

            AppEvent::ErrorCleared => {
                state.error = None;
            }
        }
    }

    /// Handle button press events
    pub fn handle_button_press(state: &mut GlobalAppState, button: Button) {
        match button {
            Button::Button1(ButtonPressType::Short) => {
                // Handle short press of Button1
                Self::handle_event(state, AppEvent::NextScreen);
            }
            Button::Button2(ButtonPressType::Short) => {
                Self::handle_event(state, AppEvent::PreviousScreen);
            }
            Button::Button1(ButtonPressType::Long) => {
                // Reserved for future use
            }
            Button::Button2(ButtonPressType::Long) => {
                // Reserved for future use
            }
            Button::Both => {
                // Reserved for future use
            }
        }
    }

    /// Handle telemetry frame updates
    pub fn handle_telemetry_frame(state: &mut GlobalAppState, frame: TelemetryFrame, now: Instant) {
        // Update MachineState and get derived events
        let (_snapshot, events) =
            update_state_with_events(&mut state.machine_state, frame.clone(), now);

        // Store the latest telemetry frame
        state.machine_state.last_frame = Some(frame);

        // Process each event through the FSM
        for event in events {
            let app_event = AppEvent::from_telemetry(event);
            Self::handle_event(state, app_event);
        }
    }

    /// Handle multiple events in sequence
    pub fn handle_events(state: &mut GlobalAppState, events: Vec<AppEvent>) {
        for event in events {
            Self::handle_event(state, event);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::screens::Screen;
    use std::time::Duration;

    #[test]
    fn test_handle_shot_started() {
        let mut state = GlobalAppState::default();
        assert_eq!(
            state.extraction_state,
            ExtractionState::Idle {
                last_extraction_duration: None
            }
        );

        AppStateMachine::handle_event(&mut state, AppEvent::ShotStarted);

        assert!(state.extraction_state.is_extracting());
    }

    #[test]
    fn test_handle_shot_ended() {
        let mut state = GlobalAppState::default();
        state.extraction_state = ExtractionState::Extracting {
            started_at: Instant::now(),
        };

        let duration = Duration::from_secs(30);
        AppStateMachine::handle_event(&mut state, AppEvent::ShotEnded);

        assert_eq!(
            state.extraction_state,
            ExtractionState::Idle {
                last_extraction_duration: Some(duration)
            }
        );
    }

    #[test]
    fn test_handle_water_refill_needed() {
        let mut state = GlobalAppState::default();
        assert!(!state.has_error());

        AppStateMachine::handle_event(&mut state, AppEvent::WaterRefillNeeded { code: 65 });

        assert!(state.has_error());
        assert_eq!(state.error, Some(AppError::WaterRefillNeeded { code: 65 }));
    }

    #[test]
    fn test_handle_water_refill_cleared() {
        let mut state = GlobalAppState::default();
        state.error = Some(AppError::WaterRefillNeeded { code: 65 });

        AppStateMachine::handle_event(&mut state, AppEvent::WaterRefillCleared);

        assert!(!state.has_error());
    }

    #[test]
    fn test_handle_next_screen() {
        let mut state = GlobalAppState::default();
        let initial_screen = state.current_screen;

        AppStateMachine::handle_event(&mut state, AppEvent::NextScreen);

        assert_eq!(state.current_screen, initial_screen.next());
    }

    #[test]
    fn test_handle_previous_screen() {
        let mut state = GlobalAppState::default();
        state.current_screen = Screen::Dashboard;

        AppStateMachine::handle_event(&mut state, AppEvent::PreviousScreen);

        assert_eq!(state.current_screen, Screen::Dashboard.previous());
    }

    #[test]
    fn test_handle_button_press_short() {
        let mut state = GlobalAppState::default();
        let initial_screen = state.current_screen;

        AppStateMachine::handle_button_press(&mut state, Button::Button1(ButtonPressType::Short));

        assert_eq!(state.current_screen, initial_screen.next());
    }

    #[test]
    fn test_handle_multiple_events() {
        let mut state = GlobalAppState::default();

        let events = vec![
            AppEvent::ShotStarted,
            AppEvent::NextScreen,
            AppEvent::WaterRefillNeeded { code: 65 },
        ];

        AppStateMachine::handle_events(&mut state, events);

        assert!(state.extraction_state.is_extracting());
        assert!(state.has_error());
    }

    #[test]
    fn test_shot_started_clears_error() {
        let mut state = GlobalAppState::default();
        state.error = Some(AppError::WaterRefillNeeded { code: 65 });

        AppStateMachine::handle_event(&mut state, AppEvent::ShotStarted);

        assert!(!state.has_error());
    }

    #[test]
    fn test_extraction_duration_persistence() {
        let mut state = GlobalAppState::default();

        // Start extraction
        AppStateMachine::handle_event(&mut state, AppEvent::ShotStarted);
        assert!(state.extraction_state.is_extracting());
        assert_eq!(state.extraction_state.last_extraction_duration(), None);

        // End extraction with 30 second duration
        let duration = Duration::from_secs(30);
        AppStateMachine::handle_event(&mut state, AppEvent::ShotEnded);

        // Verify duration is stored
        assert!(!state.extraction_state.is_extracting());
        assert_eq!(
            state.extraction_state.last_extraction_duration(),
            Some(duration)
        );

        // Start new extraction
        AppStateMachine::handle_event(&mut state, AppEvent::ShotStarted);
        assert!(state.extraction_state.is_extracting());
        // Previous duration should be cleared when new extraction starts
        assert_eq!(state.extraction_state.last_extraction_duration(), None);
    }
}
