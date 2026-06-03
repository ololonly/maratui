use crate::state::global_state::{DeviceInfo, GlobalAppState};
use crate::telemetry::AppEvent as TelemetryEvent;

const DEVICE: &str = r#"{"identifiers":["maratui_esp32"],"name":"Lelit Mara","model":"MaraTUI","manufacturer":"Lelit"}"#;

fn s_topic(prefix: &str, id: &str) -> String {
    format!("{prefix}/ha/{id}")
}

fn b_topic(prefix: &str, id: &str) -> String {
    format!("{prefix}/ha/{id}")
}

fn sensor_config(id: &str, name: &str, extra: &str, state_prefix: &str) -> (String, String) {
    let extra_part = if extra.is_empty() {
        String::new()
    } else {
        format!(",{extra}")
    };
    (
        format!("homeassistant/sensor/maratui_{id}/config"),
        format!(
            r#"{{"name":"{name}","unique_id":"maratui_{id}","state_topic":"{state_prefix}/ha/{id}","device":{DEVICE}{extra_part}}}"#
        ),
    )
}

fn binary_config(id: &str, name: &str, extra: &str, state_prefix: &str) -> (String, String) {
    let extra_part = if extra.is_empty() {
        String::new()
    } else {
        format!(",{extra}")
    };
    (
        format!("homeassistant/binary_sensor/maratui_{id}/config"),
        format!(
            r#"{{"name":"{name}","unique_id":"maratui_{id}","state_topic":"{state_prefix}/ha/{id}","payload_on":"ON","payload_off":"OFF","device":{DEVICE}{extra_part}}}"#
        ),
    )
}

/// Publish retained HA MQTT Discovery config payloads for all entities.
/// Called once on MQTT connect from setup.rs / setup_simulator.rs.
pub fn enqueue_discovery_configs(state: &mut GlobalAppState, topic_prefix: &str) {
    let sensors: &[(&str, &str, &str)] = &[
        ("mode", "Mode", r#""icon":"mdi:coffee-maker""#),
        (
            "firmware",
            "Firmware Version",
            r#""icon":"mdi:chip","entity_category":"diagnostic""#,
        ),
        (
            "boiler_now",
            "Boiler Temperature",
            r#""unit_of_measurement":"°C","device_class":"temperature","state_class":"measurement""#,
        ),
        (
            "boiler_target",
            "Boiler Target",
            r#""unit_of_measurement":"°C","device_class":"temperature","state_class":"measurement""#,
        ),
        (
            "hx_now",
            "HX Temperature",
            r#""unit_of_measurement":"°C","device_class":"temperature","state_class":"measurement""#,
        ),
        (
            "last_extraction_duration",
            "Last Extraction Duration",
            r#""unit_of_measurement":"s","icon":"mdi:coffee","state_class":"measurement""#,
        ),
        (
            "extraction_timer",
            "Extraction Timer",
            r#""unit_of_measurement":"s","icon":"mdi:timer-play","state_class":"measurement","force_update":true"#,
        ),
        (
            "time_since_last_shot",
            "Time Since Last Shot",
            r#""unit_of_measurement":"min","icon":"mdi:clock-outline","state_class":"measurement""#,
        ),
        (
            "uptime",
            "Uptime",
            r#""unit_of_measurement":"min","icon":"mdi:timer-outline","state_class":"total_increasing","entity_category":"diagnostic""#,
        ),
        (
            "wifi_rssi",
            "Wi-Fi RSSI",
            r#""unit_of_measurement":"dBm","device_class":"signal_strength","state_class":"measurement","entity_category":"diagnostic""#,
        ),
        (
            "wifi_ssid",
            "Wi-Fi SSID",
            r#""icon":"mdi:wifi","entity_category":"diagnostic""#,
        ),
        (
            "ip",
            "IP Address",
            r#""icon":"mdi:ip-network","entity_category":"diagnostic""#,
        ),
        (
            "free_heap",
            "Free Heap",
            r#""unit_of_measurement":"kB","icon":"mdi:memory","state_class":"measurement","entity_category":"diagnostic""#,
        ),
        (
            "telemetry_age",
            "Telemetry Age",
            r#""unit_of_measurement":"s","icon":"mdi:timer-sand","state_class":"measurement","entity_category":"diagnostic""#,
        ),
    ];

    for (id, name, extra) in sensors {
        let (topic, payload) = sensor_config(id, name, extra, topic_prefix);
        state.enqueue_absolute_mqtt_message(topic, payload, true);
    }

    // Cup counter: state_topic is the MQTT prefix topic that HA automation publishes to.
    {
        let cup_state_topic = format!("{topic_prefix}/cup_counter");
        let (config_topic, _) = sensor_config("cup_counter", "Cup Counter", "", topic_prefix);
        let payload = format!(
            r#"{{"name":"Cup Counter","unique_id":"maratui_cup_counter","state_topic":"{cup_state_topic}","icon":"mdi:coffee","state_class":"total_increasing","device":{DEVICE}}}"#
        );
        state.enqueue_absolute_mqtt_message(config_topic, payload, true);
    }

    let binary_sensors: &[(&str, &str, &str)] = &[
        ("heating", "Heating", r#""device_class":"heat""#),
        ("pump", "Pump Active", r#""icon":"mdi:pump""#),
        ("water_low", "Water Level Low", r#""device_class":"problem""#),
    ];

    for (id, name, extra) in binary_sensors {
        let (topic, payload) = binary_config(id, name, extra, topic_prefix);
        state.enqueue_absolute_mqtt_message(topic, payload, true);
    }
}

/// Publish current telemetry values as HA sensor states (~1 Hz).
/// Called at the end of fsm::handle_telemetry_frame, after events are processed.
pub fn enqueue_telemetry_states(state: &mut GlobalAppState) {
    let (mode_str, sw, boiler_now, boiler_target, hx_now, heating_on, pump_on, water_low) = {
        let Some(frame) = state.machine_state.last_frame.as_ref() else {
            return;
        };
        (
            frame.mode.to_string(),
            frame.sw_version.clone(),
            frame.boiler_now_c,
            frame.boiler_target_c,
            frame.hx_now_c,
            frame.heating_on,
            frame.pump_on,
            frame.no_water_code.is_some(),
        )
    };

    let timer_secs = state
        .extraction_state
        .elapsed()
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let last_shot_ended_at = state.last_shot_ended_at;
    let is_extracting = state.extraction_state.is_extracting();
    let prefix = state.mqtt_topic_prefix.clone();

    state.enqueue_absolute_mqtt_message(s_topic(&prefix, "mode"), mode_str, false);
    state.enqueue_absolute_mqtt_message(s_topic(&prefix, "firmware"), sw, false);
    state.enqueue_absolute_mqtt_message(s_topic(&prefix, "boiler_now"), boiler_now.to_string(), false);
    if let Some(t) = boiler_target {
        state.enqueue_absolute_mqtt_message(s_topic(&prefix, "boiler_target"), t.to_string(), false);
    }
    state.enqueue_absolute_mqtt_message(s_topic(&prefix, "hx_now"), hx_now.to_string(), false);
    state.enqueue_absolute_mqtt_message(s_topic(&prefix, "extraction_timer"), timer_secs.to_string(), false);

    if let Some(ended_at) = last_shot_ended_at {
        if !is_extracting {
            let mins = ended_at.elapsed().as_secs() / 60;
            state.enqueue_absolute_mqtt_message(
                s_topic(&prefix, "time_since_last_shot"),
                mins.to_string(),
                false,
            );
        }
    }

    state.enqueue_absolute_mqtt_message(
        b_topic(&prefix, "heating"),
        if heating_on { "ON" } else { "OFF" },
        false,
    );
    state.enqueue_absolute_mqtt_message(
        b_topic(&prefix, "pump"),
        if pump_on { "ON" } else { "OFF" },
        false,
    );
    state.enqueue_absolute_mqtt_message(
        b_topic(&prefix, "water_low"),
        if water_low { "ON" } else { "OFF" },
        false,
    );
}

/// Publish HA sensor/binary state updates derived from telemetry events.
/// Called in the events loop of fsm::handle_telemetry_frame, before the event moves.
pub fn enqueue_event_states(state: &mut GlobalAppState, event: &TelemetryEvent) {
    let prefix = state.mqtt_topic_prefix.clone();
    match event {
        TelemetryEvent::ShotEnded { duration } => {
            state.enqueue_absolute_mqtt_message(
                s_topic(&prefix, "last_extraction_duration"),
                duration.to_string(),
                true,
            );
            state.enqueue_absolute_mqtt_message(
                s_topic(&prefix, "extraction_timer"),
                duration.to_string(),
                true,
            );
            state.enqueue_absolute_mqtt_message(s_topic(&prefix, "time_since_last_shot"), "0", false);
        }
        TelemetryEvent::ShotStarted => {
            state.enqueue_absolute_mqtt_message(s_topic(&prefix, "extraction_timer"), "0", false);
        }
        TelemetryEvent::WaterRefillNeeded { .. } => {
            state.enqueue_absolute_mqtt_message(b_topic(&prefix, "water_low"), "ON", false);
        }
        TelemetryEvent::WaterRefillCleared => {
            state.enqueue_absolute_mqtt_message(b_topic(&prefix, "water_low"), "OFF", false);
        }
        TelemetryEvent::ShotAborted { .. } | TelemetryEvent::ModeChanged { .. } => {}
    }
}

/// Publish HA sensor states from the device status payload.
/// Called alongside the existing `device_status_payload` enqueue in fsm::handle_event.
pub fn enqueue_status_states(state: &mut GlobalAppState, info: &DeviceInfo) {
    let prefix = state.mqtt_topic_prefix.clone();
    state.enqueue_absolute_mqtt_message(
        s_topic(&prefix, "uptime"),
        (info.uptime_s / 60).to_string(),
        false,
    );
    if let Some(rssi) = info.wifi_rssi {
        state.enqueue_absolute_mqtt_message(s_topic(&prefix, "wifi_rssi"), rssi.to_string(), false);
    }
    state.enqueue_absolute_mqtt_message(s_topic(&prefix, "wifi_ssid"), info.wifi_ssid.clone(), false);
    if let Some(ip) = &info.ip {
        state.enqueue_absolute_mqtt_message(s_topic(&prefix, "ip"), ip.clone(), false);
    }
    if let Some(heap_b) = info.free_heap_b {
        state.enqueue_absolute_mqtt_message(s_topic(&prefix, "free_heap"), (heap_b / 1024).to_string(), false);
    }
    if let Some(age) = info.last_telemetry_age_s {
        state.enqueue_absolute_mqtt_message(s_topic(&prefix, "telemetry_age"), age.to_string(), false);
    }
}
