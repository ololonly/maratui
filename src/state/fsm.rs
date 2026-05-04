use log::info;

use super::{
    AppError, AppEvent, ExtractionState, GlobalAppState, global_state::EXTRACTION_RETURN_DELAY,
};
use crate::button::{Button, ButtonPressType};
use crate::screens::Screen;
use crate::telemetry::{TelemetryFrame, update_state_with_events};
use std::time::{Duration, Instant};

/// Application state machine
/// Handles events and updates the global application state
pub struct AppStateMachine;

impl AppStateMachine {
    /// Handle an application event and update the state
    pub fn handle_event(state: &mut GlobalAppState, event: AppEvent) {
        info!("Handling event: {:?}", event);

        if event.is_telemetry_event() {
            state.events_log.push_front(format!("Event: {:?}", event));
            if state.events_log.len() > 10 {
                state.events_log.pop_back();
            }
        }
        match event {
            // ===== Telemetry Events =====
            AppEvent::ShotStarted => {
                state.extraction_state = ExtractionState::Extracting {
                    started_at: Instant::now(),
                };
                // Clear error when extraction starts
                state.error = None;
                // Auto-switch to Dashboard and remember where to return
                if state.current_screen != Screen::Dashboard
                    && state.current_screen != Screen::Debug
                {
                    state.screen_before_extraction = Some(state.current_screen);
                    state.return_to_screen_at = None;
                    state.current_screen = Screen::Dashboard;
                }
            }

            AppEvent::ShotEnded { duration } => {
                state.extraction_state = ExtractionState::Idle {
                    last_extraction_duration: Some(Duration::from_secs(duration)),
                };
                // Schedule return to the previous screen after the cooldown
                if state.screen_before_extraction.is_some() {
                    state.return_to_screen_at = Some(Instant::now() + EXTRACTION_RETURN_DELAY);
                }
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
                state.screen_before_extraction = None;
                state.return_to_screen_at = None;
            }

            AppEvent::PreviousScreen => {
                state.current_screen = state.current_screen.previous();
                state.screen_before_extraction = None;
                state.return_to_screen_at = None;
            }

            AppEvent::DebugScreen => {
                if state.current_screen == Screen::Debug {
                    // Toggle back to the screen we came from
                    state.current_screen = state.screen_before_debug.take().unwrap_or(Screen::Main);
                } else {
                    state.screen_before_debug = Some(state.current_screen);
                    state.current_screen = Screen::Debug;
                    state.screen_before_extraction = None;
                    state.return_to_screen_at = None;
                }
            }

            AppEvent::ErrorOccurred { error } => {
                state.error = Some(AppError::MachineOffline);
                // Log the error (logging can be added here)
                eprintln!("Application error: {}", error);
            }

            AppEvent::ErrorCleared => {
                state.error = None;
            }

            AppEvent::WifiStatusChanged(status) => {
                state.wifi_status = status;
            }

            AppEvent::MqttStatusChanged(status) => {
                state.mqtt_status = status;
            }

            AppEvent::CupCounterUpdated { cups } => {
                state.cup_counter = Some(cups);
            }

            AppEvent::PublishMqttEvent {
                topic_suffix,
                payload,
            } => {
                state.enqueue_mqtt_message(topic_suffix, payload);
            }
        }
    }

    /// Handle button press events
    pub fn handle_button_press(state: &mut GlobalAppState, button: Button) {
        state.last_activity_at = Some(Instant::now());

        // Block navigation until the first telemetry frame arrives.
        // Debug screen (Both) is always accessible.
        if state.machine_state.last_frame.is_none() {
            if let Button::Both = button {
                Self::handle_event(state, AppEvent::DebugScreen);
            }
            return;
        }

        match button {
            Button::Button1(ButtonPressType::Short) => {
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
                Self::handle_event(state, AppEvent::DebugScreen);
            }
        }
    }

    /// Handle telemetry frame updates
    pub fn handle_telemetry_frame(state: &mut GlobalAppState, frame: TelemetryFrame, now: Instant) {
        state.enqueue_mqtt_message("telemetry", telemetry_payload(&frame));

        // Update MachineState and get derived events
        let (_snapshot, events) =
            update_state_with_events(&mut state.machine_state, frame.clone(), now);

        if state.machine_state.target_boiler_data.len() > 300 {
            state.machine_state.target_boiler_data.pop_front();
            state.machine_state.target_boiler_data.pop_front();
            state.machine_state.target_boiler_data.pop_front();
            state.machine_state.current_boiler_data.pop_front();
            state.machine_state.current_boiler_data.pop_front();
            state.machine_state.current_boiler_data.pop_front();
            state.machine_state.current_hx_data.pop_front();
            state.machine_state.current_hx_data.pop_front();
            state.machine_state.current_hx_data.pop_front();
        }

        if let Some(boiler_target_c) = frame.boiler_target_c {
            state
                .machine_state
                .target_boiler_data
                .push_back(boiler_target_c.into());
            state
                .machine_state
                .target_boiler_data
                .push_back(boiler_target_c.into());
            state
                .machine_state
                .target_boiler_data
                .push_back(boiler_target_c.into());
        } else {
            state
                .machine_state
                .target_boiler_data
                .push_back(f64::default());
            state
                .machine_state
                .target_boiler_data
                .push_back(f64::default());
            state
                .machine_state
                .target_boiler_data
                .push_back(f64::default());
        }
        let boiler_now_c = frame.boiler_now_c;
        state
            .machine_state
            .current_boiler_data
            .push_back(boiler_now_c.into());
        state
            .machine_state
            .current_boiler_data
            .push_back(boiler_now_c.into());
        state
            .machine_state
            .current_boiler_data
            .push_back(boiler_now_c.into());
        let hx_now_c = frame.hx_now_c;
        state
            .machine_state
            .current_hx_data
            .push_back(hx_now_c.into());
        state
            .machine_state
            .current_hx_data
            .push_back(hx_now_c.into());
        state
            .machine_state
            .current_hx_data
            .push_back(hx_now_c.into());

        // Store the latest telemetry frame and record activity time
        state.machine_state.last_frame = Some(frame);
        state.last_activity_at = Some(now);
        state.last_uart_frame_at = Some(now);

        // Process each event through the FSM
        for event in events {
            let app_event = AppEvent::from_telemetry(event.clone());
            Self::handle_event(state, app_event);
            let event_payload = telemetry_event_payload(&event);
            state.enqueue_mqtt_message("events", event_payload);
        }
    }

    /// Handle multiple events in sequence
    pub fn handle_events(state: &mut GlobalAppState, events: Vec<AppEvent>) {
        for event in events {
            Self::handle_event(state, event);
        }
    }

    /// Check time-based transitions (call once per main loop iteration)
    pub fn tick(state: &mut GlobalAppState, now: Instant) {
        if let Some(return_at) = state.return_to_screen_at {
            if now >= return_at {
                if let Some(screen) = state.screen_before_extraction.take() {
                    state.current_screen = screen;
                }
                state.return_to_screen_at = None;
            }
        }
    }
}

fn telemetry_payload(frame: &TelemetryFrame) -> String {
    format!(
        "{{\"mode\":\"{}\",\"sw\":\"{}\",\"boiler_now_c\":{},\"boiler_target_c\":{},\"hx_now_c\":{},\"boost_countdown_s\":{},\"heating_on\":{},\"pump_on\":{},\"no_water_code\":{}}}",
        frame.mode,
        frame.sw_version,
        frame.boiler_now_c,
        frame
            .boiler_target_c
            .map(|v| v.to_string())
            .unwrap_or_else(|| "null".to_string()),
        frame.hx_now_c,
        frame.boost_countdown_s,
        frame.heating_on,
        frame.pump_on,
        frame
            .no_water_code
            .map(|v| v.to_string())
            .unwrap_or_else(|| "null".to_string())
    )
}

fn telemetry_event_payload(event: &crate::telemetry::AppEvent) -> String {
    match event {
        crate::telemetry::AppEvent::ShotStarted => "{\"type\":\"shot_started\"}".to_string(),
        crate::telemetry::AppEvent::ShotEnded { duration } => {
            format!("{{\"type\":\"shot_ended\",\"duration\":{}}}", duration)
        }
        crate::telemetry::AppEvent::WaterRefillNeeded { code } => {
            format!("{{\"type\":\"water_refill_needed\",\"code\":{}}}", code)
        }
        crate::telemetry::AppEvent::WaterRefillCleared => {
            "{\"type\":\"water_refill_cleared\"}".to_string()
        }
        crate::telemetry::AppEvent::ModeChanged { from, to } => {
            format!(
                "{{\"type\":\"mode_changed\",\"from\":\"{}\",\"to\":\"{}\"}}",
                from, to
            )
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
        AppStateMachine::handle_event(
            &mut state,
            AppEvent::ShotEnded {
                duration: duration.as_secs() as u64,
            },
        );

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
        // Navigation is blocked until first telemetry arrives
        state.machine_state.last_frame = Some(TelemetryFrame::debug_frame());
        let initial_screen = state.current_screen;

        AppStateMachine::handle_button_press(&mut state, Button::Button1(ButtonPressType::Short));

        assert_eq!(state.current_screen, initial_screen.next());
    }

    #[test]
    fn test_handle_button_press_blocked_before_telemetry() {
        let mut state = GlobalAppState::default();
        assert!(state.machine_state.last_frame.is_none());
        let initial_screen = state.current_screen;

        AppStateMachine::handle_button_press(&mut state, Button::Button1(ButtonPressType::Short));
        AppStateMachine::handle_button_press(&mut state, Button::Button2(ButtonPressType::Short));

        // Screen must not change while no telemetry
        assert_eq!(state.current_screen, initial_screen);
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
        AppStateMachine::handle_event(
            &mut state,
            AppEvent::ShotEnded {
                duration: duration.as_secs() as u64,
            },
        );

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

    #[test]
    fn test_handle_cup_counter_updated() {
        let mut state = GlobalAppState::default();
        assert_eq!(state.cup_counter, None);

        AppStateMachine::handle_event(&mut state, AppEvent::CupCounterUpdated { cups: 42 });

        assert_eq!(state.cup_counter, Some(42));
    }

    #[test]
    fn test_shot_started_switches_to_dashboard() {
        let mut state = GlobalAppState::default();
        assert_eq!(state.current_screen, Screen::Main);

        AppStateMachine::handle_event(&mut state, AppEvent::ShotStarted);

        assert_eq!(state.current_screen, Screen::Dashboard);
        assert_eq!(state.screen_before_extraction, Some(Screen::Main));
        assert!(state.return_to_screen_at.is_none());
    }

    #[test]
    fn test_shot_started_from_dashboard_does_not_save_dashboard() {
        let mut state = GlobalAppState::default();
        state.current_screen = Screen::Dashboard;

        AppStateMachine::handle_event(&mut state, AppEvent::ShotStarted);

        assert_eq!(state.current_screen, Screen::Dashboard);
        assert_eq!(state.screen_before_extraction, None);
    }

    #[test]
    fn test_shot_ended_schedules_return() {
        let mut state = GlobalAppState::default();
        AppStateMachine::handle_event(&mut state, AppEvent::ShotStarted);
        AppStateMachine::handle_event(&mut state, AppEvent::ShotEnded { duration: 30 });

        assert!(state.return_to_screen_at.is_some());
    }

    #[test]
    fn test_tick_returns_to_saved_screen_after_delay() {
        let mut state = GlobalAppState::default();
        AppStateMachine::handle_event(&mut state, AppEvent::ShotStarted);
        AppStateMachine::handle_event(&mut state, AppEvent::ShotEnded { duration: 30 });

        assert_eq!(state.current_screen, Screen::Dashboard);

        // Tick before the delay — should stay on Dashboard
        AppStateMachine::tick(&mut state, Instant::now());
        assert_eq!(state.current_screen, Screen::Dashboard);

        // Tick after the delay has elapsed
        let past = Instant::now() - EXTRACTION_RETURN_DELAY - Duration::from_millis(1);
        state.return_to_screen_at = Some(past);
        AppStateMachine::tick(&mut state, Instant::now());

        assert_eq!(state.current_screen, Screen::Main);
        assert!(state.screen_before_extraction.is_none());
        assert!(state.return_to_screen_at.is_none());
    }

    #[test]
    fn test_debug_screen_toggle() {
        let mut state = GlobalAppState::default();
        state.current_screen = Screen::Dashboard;

        // Enter Debug
        AppStateMachine::handle_event(&mut state, AppEvent::DebugScreen);
        assert_eq!(state.current_screen, Screen::Debug);

        // Toggle back — should return to Dashboard
        AppStateMachine::handle_event(&mut state, AppEvent::DebugScreen);
        assert_eq!(state.current_screen, Screen::Dashboard);
        assert!(state.screen_before_debug.is_none());
    }

    #[test]
    fn test_debug_screen_toggle_fallback_to_main() {
        let mut state = GlobalAppState::default();
        // Enter Debug without prior screen set (default Main)
        AppStateMachine::handle_event(&mut state, AppEvent::DebugScreen);
        AppStateMachine::handle_event(&mut state, AppEvent::DebugScreen);
        assert_eq!(state.current_screen, Screen::Main);
    }

    #[test]
    fn test_manual_navigation_cancels_return() {
        let mut state = GlobalAppState::default();
        AppStateMachine::handle_event(&mut state, AppEvent::ShotStarted);
        AppStateMachine::handle_event(&mut state, AppEvent::ShotEnded { duration: 30 });

        assert!(state.return_to_screen_at.is_some());

        AppStateMachine::handle_event(&mut state, AppEvent::NextScreen);

        assert!(state.screen_before_extraction.is_none());
        assert!(state.return_to_screen_at.is_none());
    }
}
