use log::{error, info};

use super::global_state::MACHINE_OFFLINE_TIMEOUT;
use super::{AppError, AppEvent, ConnectionStatus, DeviceInfo, ExtractionState, GlobalAppState};
use crate::button::{Button, ButtonPressType};
#[cfg(feature = "home-assistant")]
use crate::home_assistant;
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
                state.last_shot_ended_at = Some(Instant::now());
            }

            AppEvent::ShotAborted { .. } => {
                state.extraction_state = ExtractionState::Idle {
                    last_extraction_duration: None,
                };
                state.last_shot_ended_at = Some(Instant::now());
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
                // Debug uses a full-screen layout unrelated to the normal screens; clear so
                // nothing from the previous layout lingers.
                state.request_redraw();
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
                match &status {
                    ConnectionStatus::Connected | ConnectionStatus::Disconnected => {
                        state.events_log.push_front(format!("Wi-Fi: {:?}", status));
                        if state.events_log.len() > 10 {
                            state.events_log.pop_back();
                        }
                    }
                    _ => {}
                }
                state.wifi_status = status;
            }

            AppEvent::MqttStatusChanged(status) => {
                let should_log = match &status {
                    ConnectionStatus::Connected => state.mqtt_status != ConnectionStatus::Connected,
                    ConnectionStatus::Disconnected | ConnectionStatus::Error(_) => true,
                    _ => false,
                };
                if should_log {
                    state.events_log.push_front(format!("MQTT: {:?}", status));
                    if state.events_log.len() > 10 {
                        state.events_log.pop_back();
                    }
                }
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
                #[cfg(feature = "home-assistant")]
                home_assistant::enqueue_status_states(state, &info);
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
        let now = Instant::now();
        let backlight_was_on = state.backlight_should_be_on(now);
        state.last_activity_at = Some(now);

        // If the screen was dark (backlight timed out), the first press only wakes the
        // backlight and must not also switch screens.
        if !backlight_was_on {
            return;
        }

        if let Button::Button1(ButtonPressType::Short) = button
            && state.current_screen != Screen::Debug
        {
            Self::handle_event(state, AppEvent::NextScreen);
        }
    }

    /// Handle telemetry frame updates
    pub fn handle_telemetry_frame(state: &mut GlobalAppState, frame: TelemetryFrame, now: Instant) {
        state.enqueue_mqtt_message("telemetry", telemetry_payload(&frame));

        // A new session starts on the very first frame, or when telemetry resumes after the
        // machine has been offline (long UART gap). Reset per-session state and ask the render
        // loop to clear the terminal so any accumulated display artifacts are wiped.
        let new_session = state
            .last_uart_frame_at
            .map(|t| now.saturating_duration_since(t) >= MACHINE_OFFLINE_TIMEOUT)
            .unwrap_or(true);
        if new_session {
            state.machine_state.reset_session();
            state.request_redraw();
        }

        // frame moves into update_state_with_events; it is stored in state.machine_state.last_frame
        let (_snapshot, events) = update_state_with_events(&mut state.machine_state, frame, now);

        // Extract Copy fields from last_frame before the mutable borrows below
        let (boiler_target, boiler_now_c, hx_now_c) = {
            let last = state
                .machine_state
                .last_frame
                .as_ref()
                .expect("update_state_with_events always stores the frame in last_frame");
            (
                last.boiler_target_c.map(f64::from).unwrap_or_default(),
                f64::from(last.boiler_now_c),
                f64::from(last.hx_now_c),
            )
        };
        let should_sample = state
            .machine_state
            .last_graph_sample_at
            .map(|t| now.saturating_duration_since(t) >= GRAPH_SAMPLE_INTERVAL)
            .unwrap_or(true);
        if should_sample {
            state.machine_state.last_graph_sample_at = Some(now);
            push_sample(&mut state.machine_state.target_boiler_data, boiler_target);
            push_sample(&mut state.machine_state.current_boiler_data, boiler_now_c);
            push_sample(&mut state.machine_state.current_hx_data, hx_now_c);
            state.machine_state.rebuild_graph_points();
        }
        state.last_activity_at = Some(now);
        state.last_uart_frame_at = Some(now);

        // Process each event through the FSM
        for event in events {
            let event_payload = telemetry_event_payload(&event);
            #[cfg(feature = "home-assistant")]
            home_assistant::enqueue_event_states(state, &event);
            Self::handle_event(state, AppEvent::from_telemetry(event));
            state.enqueue_mqtt_message("events", event_payload);
        }

        #[cfg(feature = "home-assistant")]
        home_assistant::enqueue_telemetry_states(state);
    }

    /// Handle multiple events in sequence
    pub fn handle_events(state: &mut GlobalAppState, events: Vec<AppEvent>) {
        for event in events {
            Self::handle_event(state, event);
        }
    }
}

fn json_opt_num<T: std::fmt::Display>(opt: Option<T>) -> String {
    opt.map_or_else(|| "null".to_string(), |v| v.to_string())
}

fn json_opt_str(opt: Option<&str>) -> String {
    opt.map_or_else(|| "null".to_string(), |s| format!("\"{}\"", s))
}

fn telemetry_payload(frame: &TelemetryFrame) -> String {
    format!(
        "{{\"mode\":\"{}\",\"sw\":\"{}\",\"boiler_now_c\":{},\"boiler_target_c\":{},\"hx_now_c\":{},\"boost_countdown_s\":{},\"heating_on\":{},\"pump_on\":{},\"no_water_code\":{}}}",
        frame.mode,
        frame.sw_version,
        frame.boiler_now_c,
        json_opt_num(frame.boiler_target_c),
        frame.hx_now_c,
        frame.boost_countdown_s,
        frame.heating_on,
        frame.pump_on,
        json_opt_num(frame.no_water_code),
    )
}

/// One sample per second; 300 samples = 5-minute window shown on the Graphs screen.
const GRAPH_BUF_CAP: usize = 300;
const GRAPH_SAMPLE_INTERVAL: Duration = Duration::from_secs(1);

fn push_sample(buf: &mut std::collections::VecDeque<f64>, value: f64) {
    buf.push_back(value);
    if buf.len() > GRAPH_BUF_CAP {
        buf.pop_front();
    }
}

fn device_status_payload(info: &DeviceInfo) -> String {
    format!(
        "{{\"uptime_s\":{},\"wifi_ssid\":\"{}\",\"wifi_rssi\":{},\"ip\":{},\"free_heap_b\":{},\"last_telemetry_age_s\":{}}}",
        info.uptime_s,
        info.wifi_ssid,
        json_opt_num(info.wifi_rssi),
        json_opt_str(info.ip.as_deref()),
        json_opt_num(info.free_heap_b),
        json_opt_num(info.last_telemetry_age_s),
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
    use crate::state::global_state::BACKLIGHT_TIMEOUT;
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
                duration: duration.as_secs(),
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
        // Backlight must be on for a press to switch screens
        state.last_activity_at = Some(Instant::now());
        let initial_screen = state.current_screen;

        AppStateMachine::handle_button_press(&mut state, Button::Button1(ButtonPressType::Short));

        assert_eq!(state.current_screen, initial_screen.next());
    }

    #[test]
    fn test_press_while_dark_only_wakes_backlight() {
        let mut state = GlobalAppState::default();
        state.machine_state.last_frame = Some(TelemetryFrame::debug_frame());
        // Backlight timed out (last activity well in the past)
        state.last_activity_at = Some(Instant::now() - BACKLIGHT_TIMEOUT - Duration::from_secs(1));
        let initial_screen = state.current_screen;

        AppStateMachine::handle_button_press(&mut state, Button::Button1(ButtonPressType::Short));

        // First press only wakes the backlight: screen must not change, activity refreshed
        assert_eq!(state.current_screen, initial_screen);
        assert!(state.backlight_should_be_on(Instant::now()));

        // Second press (backlight now on) switches screens
        AppStateMachine::handle_button_press(&mut state, Button::Button1(ButtonPressType::Short));
        assert_eq!(state.current_screen, initial_screen.next());
    }

    #[test]
    fn test_telemetry_resume_starts_new_session() {
        let mut state = GlobalAppState::default();
        let t0 = Instant::now();

        // First frame: a shot runs and is sampled into the graph buffers.
        let on = TelemetryFrame::debug_pump_on_frame();
        AppStateMachine::handle_telemetry_frame(&mut state, on, t0);
        assert!(state.take_redraw_request(), "first frame starts a session");
        assert!(!state.machine_state.current_boiler_data.is_empty());

        // Telemetry resumes after the machine was offline: buffers reset, redraw requested,
        // and no spurious shot-end event is logged from the stale pump-on frame.
        let events_before = state.events_log.len();
        let resume = TelemetryFrame::debug_frame();
        AppStateMachine::handle_telemetry_frame(
            &mut state,
            resume,
            t0 + MACHINE_OFFLINE_TIMEOUT + Duration::from_secs(1),
        );
        assert!(state.take_redraw_request(), "resume starts a new session");
        // Exactly one fresh sample after the reset (no stale history)
        assert_eq!(state.machine_state.current_boiler_data.len(), 1);
        assert_eq!(
            state.events_log.len(),
            events_before,
            "no phantom transition events on resume"
        );
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
                duration: duration.as_secs(),
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
