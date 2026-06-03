use crate::app::MaraUiApp;
use crate::button::{Button, ButtonPressType};
use crate::config::AppConfig;
use crate::state::{AppEvent, ConnectionStatus, DeviceInfo};
use crate::state::global_state::MqttOutboundMessage;
use crate::telemetry::TelemetryFrame;
use mousefood::embedded_graphics::prelude::Size;
use mousefood::fonts::*;
use mousefood::prelude::*;
use ratatui::Terminal;
use rumqttc::{Client, Event, MqttOptions, Packet, QoS};

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant};

use embedded_graphics_simulator::{
    OutputSettingsBuilder, SimulatorDisplay, SimulatorEvent, Window, sdl2::Keycode,
};

pub fn run_app(app: impl MaraUiApp) {
    run_app_simulator(app);
}

fn run_app_simulator(mut app: impl MaraUiApp) {
    let app_config = AppConfig::from_env().expect("Invalid MARATUI_* configuration");
    app.set_mqtt_prefix(&app_config.mqtt.topic_prefix);

    app.handle_event(AppEvent::WifiStatusChanged(ConnectionStatus::Disabled));
    app.handle_event(AppEvent::MqttStatusChanged(ConnectionStatus::Connecting));

    let output_settings = OutputSettingsBuilder::new().scale(2).build();
    let simulator_window = Rc::new(RefCell::new(Window::new(
        "Maratui Simulator",
        &output_settings,
    )));
    simulator_window.borrow_mut().set_max_fps(30);

    let mut display = SimulatorDisplay::<Rgb565>::new(Size::new(320, 240));

    let simulator_window_for_flush = Rc::clone(&simulator_window);
    let config = EmbeddedBackendConfig {
        flush_callback: Box::new(move |display: &mut SimulatorDisplay<Rgb565>| {
            let mut window = simulator_window_for_flush.borrow_mut();
            window.update(display);
        }),
        font_regular: MONO_7X14,
        font_bold: Some(MONO_7X14_BOLD),
        font_italic: Some(MONO_7X14),
        ..Default::default()
    };

    let backend = EmbeddedBackend::new(&mut display, config);
    let mut terminal = Terminal::new(backend).unwrap();

    let (mut mqtt, mut cup_counter_rx, mut mqtt_status_rx) = init_simulator_mqtt(&app_config);
    if mqtt.is_some() {
        app.handle_event(AppEvent::MqttStatusChanged(ConnectionStatus::Connecting));
    } else {
        app.handle_event(AppEvent::MqttStatusChanged(ConnectionStatus::Disabled));
    }

    // Simulate the loading stages so the loading screen is visible in the simulator
    let stages: &[(&'static str, u8, u64)] = &[
        ("connecting wifi...", 20, 800),
        ("putting on hat...", 55, 600),
        ("heating up the machine...", 85, 700),
    ];
    for &(message, progress, delay_ms) in stages {
        app.handle_event(AppEvent::LoadingStage { message, progress });
        app.render_image(terminal.backend_mut().display_mut());
        terminal.draw(|f| app.draw(f)).unwrap();
        std::thread::sleep(Duration::from_millis(delay_ms));
    }
    app.handle_event(AppEvent::LoadingComplete);

    const STATUS_INTERVAL: Duration = Duration::from_secs(30);
    let boot_time = Instant::now();
    let mut last_status_at: Option<Instant> = None;

    let mut prev_had_telemetry = false;

    loop {
        app.tick();

        app.render_image(terminal.backend_mut().display_mut());
        terminal
            .draw(|f| {
                app.draw(f);
            })
            .unwrap();

        // Clear terminal once when the first UART frame arrives
        let now_has_telemetry = app.has_telemetry();
        if !prev_had_telemetry && now_has_telemetry {
            terminal.clear().unwrap();
        }
        prev_had_telemetry = now_has_telemetry;

        for event in simulator_window.borrow_mut().events() {
            match event {
                SimulatorEvent::Quit => std::process::exit(0),
                SimulatorEvent::KeyDown {
                    keycode, repeat, ..
                } => {
                    if repeat {
                        continue;
                    }
                    match keycode {
                        Keycode::Right | Keycode::Left => {
                            terminal.clear().unwrap();
                            app.handle_press(Button::Button1(ButtonPressType::Short));
                        }
                        Keycode::Up => {
                            let had = app.has_telemetry();
                            app.update_telemetry(TelemetryFrame::debug_pump_on_frame());
                            if !had && app.has_telemetry() {
                                terminal.clear().unwrap();
                            }
                        }
                        Keycode::Down => {
                            let had = app.has_telemetry();
                            app.update_telemetry(TelemetryFrame::debug_frame());
                            if !had && app.has_telemetry() {
                                terminal.clear().unwrap();
                            }
                        }
                        Keycode::Space => {
                            let had = app.has_telemetry();
                            app.update_telemetry(TelemetryFrame::debug_no_water_frame());
                            if !had && app.has_telemetry() {
                                terminal.clear().unwrap();
                            }
                        }
                        Keycode::M => {
                            app.handle_event(AppEvent::PublishMqttEvent {
                                topic_suffix: "events".to_string(),
                                payload: "{\"type\":\"manual_sim_event\"}".to_string(),
                            });
                        }
                        Keycode::D => {
                            terminal.clear().unwrap();
                            app.handle_press(Button::Button1(ButtonPressType::Long));
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        }

        if let Some(status_rx) = mqtt_status_rx.as_mut() {
            while let Ok(status) = status_rx.try_recv() {
                if status == ConnectionStatus::Connected {
                    #[cfg(feature = "home-assistant")]
                    app.enqueue_home_assistant(&app_config.mqtt.topic_prefix);
                }
                app.handle_event(AppEvent::MqttStatusChanged(status));
            }
        }

        let now = Instant::now();
        let should_publish_status = last_status_at
            .map(|t| now.saturating_duration_since(t) >= STATUS_INTERVAL)
            .unwrap_or(true);
        if should_publish_status {
            last_status_at = Some(now);
            let ssid = app_config
                .wifi
                .as_ref()
                .map(|w| w.ssid.clone())
                .unwrap_or_else(|| "simulator".to_string());
            app.handle_event(AppEvent::DeviceInfoUpdated(DeviceInfo {
                wifi_ssid: ssid,
                wifi_rssi: Some(-65),
                ip: Some("127.0.0.1".to_string()),
                uptime_s: now.saturating_duration_since(boot_time).as_secs(),
                free_heap_b: None,
                last_telemetry_age_s: app
                    .last_uart_frame_at()
                    .map(|t| now.saturating_duration_since(t).as_secs()),
            }));
        }

        let (_, mqtt_status) = app.connection_statuses();
        let messages = app.take_outbound_mqtt_messages();
        if mqtt_status == ConnectionStatus::Connected {
            for msg in messages {
                publish_mqtt_message(&mut mqtt, &app_config, &msg);
            }
        }

        if let Some(cup_counter_rx) = cup_counter_rx.as_mut() {
            while let Ok(cups) = cup_counter_rx.try_recv() {
                app.handle_event(AppEvent::CupCounterUpdated { cups });
            }
        }
    }
}

fn init_simulator_mqtt(
    cfg: &AppConfig,
) -> (
    Option<Client>,
    Option<Receiver<u64>>,
    Option<Receiver<ConnectionStatus>>,
) {
    if !cfg.mqtt.enabled {
        return (None, None, None);
    }

    let Some((host, port)) = parse_mqtt_host_port(&cfg.mqtt.url) else {
        return (None, None, None);
    };
    let mut options = MqttOptions::new(format!("{}-sim", cfg.mqtt.client_id), host, port);
    options.set_keep_alive(Duration::from_secs(30));

    if let Some(username) = cfg.mqtt.username.as_ref() {
        options.set_credentials(username, cfg.mqtt.password.as_deref().unwrap_or_default());
    }

    let (client, mut connection) = Client::new(options, 10);
    let cup_counter_topic = format!("{}/cup_counter", cfg.mqtt.topic_prefix);
    let (cup_counter_tx, cup_counter_rx) = mpsc::channel::<u64>();
    let (mqtt_status_tx, mqtt_status_rx) = mpsc::channel::<ConnectionStatus>();

    let _ = client.subscribe(&cup_counter_topic, QoS::AtMostOnce);

    std::thread::spawn(move || {
        for notification in connection.iter() {
            match notification {
                Ok(Event::Incoming(Packet::ConnAck(_))) => {
                    let _ = mqtt_status_tx.send(ConnectionStatus::Connected);
                }
                Err(_) => {
                    let _ = mqtt_status_tx.send(ConnectionStatus::Connecting);
                }
                Ok(Event::Incoming(Packet::Publish(publish))) => {
                    if publish.topic == cup_counter_topic
                        && let Ok(payload) = core::str::from_utf8(&publish.payload)
                        && let Ok(cups) = payload.trim().parse::<u64>()
                    {
                        let _ = cup_counter_tx.send(cups);
                    }
                }
                _ => {}
            }
        }
    });
    (Some(client), Some(cup_counter_rx), Some(mqtt_status_rx))
}

fn parse_mqtt_host_port(url: &str) -> Option<(String, u16)> {
    let without_proto = url
        .strip_prefix("mqtt://")
        .or_else(|| url.strip_prefix("tcp://"))
        .or_else(|| url.strip_prefix("ws://"))
        .unwrap_or(url);

    let host_port = without_proto.split('/').next()?;
    if let Some((host, port)) = host_port.rsplit_once(':') {
        return Some((host.to_string(), port.parse().ok()?));
    }

    Some((host_port.to_string(), 1883))
}

fn publish_mqtt_message(
    client: &mut Option<Client>,
    cfg: &AppConfig,
    msg: &MqttOutboundMessage,
) {
    let Some(client) = client.as_mut() else {
        return;
    };

    let topic = if msg.absolute {
        msg.topic_suffix.clone()
    } else {
        format!("{}/{}", cfg.mqtt.topic_prefix, msg.topic_suffix)
    };
    let qos = if msg.retain {
        QoS::AtLeastOnce
    } else {
        QoS::AtMostOnce
    };
    let _ = client.publish(topic, qos, msg.retain, msg.payload.as_bytes());
}
