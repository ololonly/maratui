use log::{error, info};

use super::{AppError, AppEvent, DeviceInfo, ExtractionState, GlobalAppState};
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
                state.error = None;
            }

            AppEvent::ShotEnded { duration } => {
                state.extraction_state = ExtractionState::Idle {
                    last_extraction_duration: Some(Duration::from_secs(duration)),
                };
            }

            AppEvent::ShotAborted { .. } => {
                state.extraction_state = ExtractionState::Idle {
                    last_extraction_duration: None,
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

            AppEvent::DebugScreen => {
                if state.current_screen == Screen::Debug {
                    state.current_screen = state
                        .screen_before_debug
                        .take()
                        .unwrap_or(Screen::Dashboard);
                } else {
                    state.screen_before_debug = Some(state.current_screen);
                    state.current_screen = Screen::Debug;
                }
            }

            AppEvent::ErrorOccurred { error } => {
                state.error = Some(AppError::MachineOffline);
                // Log the error (logging can be added here)
                error!("Application error: {}", error);
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

            AppEvent::DeviceInfoUpdated(info) => {
                let payload = device_status_payload(&info);
                state.device_info = info;
                state.enqueue_mqtt_message("status", payload);
            }

            AppEvent::LoadingStage { message, progress } => {
                state.loading_status = Some((message, progress));
            }

            AppEvent::LoadingComplete => {
                state.loading_status = Some(("waiting for machine...", 100));
            }
        }
    }

    /// Handle button press events
    pub fn handle_button_press(state: &mut GlobalAppState, button: Button) {
        // Long press always toggles Debug, even during loading.
        if let Button::Button1(ButtonPressType::Long) = button {
            Self::handle_event(state, AppEvent::DebugScreen);
            return;
        }

        // During loading: short press manually toggles the backlight.
        if state.machine_state.last_frame.is_none() {
            if let Button::Button1(ButtonPressType::Short) = button {
                state.backlight_on = !state.backlight_on;
            }
            return;
        }

        // Normal operation after first UART frame arrives.
        state.last_activity_at = Some(Instant::now());
        if let Button::Button1(ButtonPressType::Short) = button {
            if state.current_screen != Screen::Debug {
                Self::handle_event(state, AppEvent::NextScreen);
            }
        }
    }

    /// Handle telemetry frame updates
    pub fn handle_telemetry_frame(state: &mut GlobalAppState, frame: TelemetryFrame, now: Instant) {
        state.enqueue_mqtt_message("telemetry", telemetry_payload(&frame));

        // frame moves into update_state_with_events; it is stored in state.machine_state.last_frame
        let (_snapshot, events) = update_state_with_events(&mut state.machine_state, frame, now);

        // Extract Copy fields from last_frame before the mutable borrows below
        let (boiler_target, boiler_now_c, hx_now_c) = {
            let last = state.machine_state.last_frame.as_ref().unwrap();
            (
                last.boiler_target_c.map(f64::from).unwrap_or_default(),
                f64::from(last.boiler_now_c),
                f64::from(last.hx_now_c),
            )
        };
        push_triple(&mut state.machine_state.target_boiler_data, boiler_target);
        push_triple(&mut state.machine_state.current_boiler_data, boiler_now_c);
        push_triple(&mut state.machine_state.current_hx_data, hx_now_c);
        state.machine_state.rebuild_graph_points();
        state.last_activity_at = Some(now);
        state.last_uart_frame_at = Some(now);

        // Process each event through the FSM
        for event in events {
            let event_payload = telemetry_event_payload(&event);
            Self::handle_event(state, AppEvent::from_telemetry(event));
            state.enqueue_mqtt_message("events", event_payload);
        }
    }

    /// Handle multiple events in sequence
    pub fn handle_events(state: &mut GlobalAppState, events: Vec<AppEvent>) {
        for event in events {
            Self::handle_event(state, event);
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

/// Push `value` three times into `buf`, then trim the oldest triple if the cap is exceeded.
/// The triple push keeps graph data aligned: each telemetry frame maps to 3 x-axis points,
/// which smooths the braille-character chart at the Graphs screen's 300-point x bound.
const GRAPH_BUF_CAP: usize = 300;
fn push_triple(buf: &mut std::collections::VecDeque<f64>, value: f64) {
    buf.push_back(value);
    buf.push_back(value);
    buf.push_back(value);
    while buf.len() > GRAPH_BUF_CAP {
        buf.pop_front();
    }
}

fn device_status_payload(info: &DeviceInfo) -> String {
    let rssi = info
        .wifi_rssi
        .map(|v| v.to_string())
        .unwrap_or_else(|| "null".to_string());
    let ip = info
        .ip
        .as_deref()
        .map(|s| format!("\"{}\"", s))
        .unwrap_or_else(|| "null".to_string());
    let heap = info
        .free_heap_b
        .map(|v| v.to_string())
        .unwrap_or_else(|| "null".to_string());
    format!(
        "{{\"uptime_s\":{},\"wifi_ssid\":\"{}\",\"wifi_rssi\":{},\"ip\":{},\"free_heap_b\":{}}}",
        info.uptime_s, info.wifi_ssid, rssi, ip, heap,
    )
}

fn telemetry_event_payload(event: &crate::telemetry::AppEvent) -> String {
    match event {
        crate::telemetry::AppEvent::ShotStarted => "{\"type\":\"shot_started\"}".to_string(),
        crate::telemetry::AppEvent::ShotEnded { duration } => {
            format!("{{\"type\":\"shot_ended\",\"duration\":{}}}", duration)
        }
        crate::telemetry::AppEvent::ShotAborted { duration } => {
            format!("{{\"type\":\"shot_aborted\",\"duration\":{}}}", duration)
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

        // Screen must not change while no telemetry
        assert_eq!(state.current_screen, initial_screen);
    }

    #[test]
    fn test_loading_short_press_toggles_backlight() {
        let mut state = GlobalAppState::default();
        assert!(state.machine_state.last_frame.is_none());
        assert!(state.backlight_on);

        AppStateMachine::handle_button_press(&mut state, Button::Button1(ButtonPressType::Short));
        assert!(!state.backlight_on);

        AppStateMachine::handle_button_press(&mut state, Button::Button1(ButtonPressType::Short));
        assert!(state.backlight_on);
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
    fn test_shot_aborted_resets_timer() {
        let mut state = GlobalAppState::default();

        // Simulate a previous real shot
        AppStateMachine::handle_event(&mut state, AppEvent::ShotStarted);
        AppStateMachine::handle_event(&mut state, AppEvent::ShotEnded { duration: 30 });
        assert!(state.extraction_state.last_extraction_duration().is_some());

        // Short pump run — must not overwrite last_extraction_duration
        AppStateMachine::handle_event(&mut state, AppEvent::ShotStarted);
        AppStateMachine::handle_event(&mut state, AppEvent::ShotAborted { duration: 8 });

        assert!(!state.extraction_state.is_extracting());
        assert_eq!(state.extraction_state.last_extraction_duration(), None);
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
    fn test_debug_screen_toggle_fallback_to_dashboard() {
        let mut state = GlobalAppState::default();
        // Enter Debug without prior screen set (default Dashboard)
        AppStateMachine::handle_event(&mut state, AppEvent::DebugScreen);
        AppStateMachine::handle_event(&mut state, AppEvent::DebugScreen);
        assert_eq!(state.current_screen, Screen::Dashboard);
    }
}
