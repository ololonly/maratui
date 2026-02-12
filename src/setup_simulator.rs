use crate::app::MaraUiApp;
use crate::button::{Button, ButtonPressType};
use crate::config::AppConfig;
use crate::state::{AppEvent, ConnectionStatus};
use crate::telemetry::TelemetryFrame;
use mousefood::embedded_graphics::prelude::Size;
use mousefood::fonts::*;
use mousefood::prelude::*;
use ratatui::Terminal;
use rumqttc::{Client, MqttOptions, QoS};

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use embedded_graphics_simulator::{
    OutputSettingsBuilder, SimulatorDisplay, SimulatorEvent, Window, sdl2::Keycode,
};

pub fn run_app(app: impl MaraUiApp) {
    run_app_simulator(app);
}

fn run_app_simulator(mut app: impl MaraUiApp) {
    let app_config = AppConfig::from_env().expect("Invalid MARATUI_* configuration");

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

    let mut mqtt = init_simulator_mqtt(&app_config);
    if mqtt.is_some() {
        app.handle_event(AppEvent::MqttStatusChanged(ConnectionStatus::Connected));
    } else {
        app.handle_event(AppEvent::MqttStatusChanged(ConnectionStatus::Disabled));
    }

    loop {
        app.render_image(terminal.backend_mut().display_mut());
        terminal
            .draw(|f| {
                app.draw(f);
            })
            .unwrap();

        for event in simulator_window.borrow_mut().events() {
            match event {
                SimulatorEvent::Quit => panic!("simulator window closed"),
                SimulatorEvent::KeyDown {
                    keycode, repeat, ..
                } => {
                    if repeat {
                        continue;
                    }
                    match keycode {
                        Keycode::Left => {
                            terminal.clear().unwrap();
                            app.handle_press(Button::Button2(ButtonPressType::Short));
                        }
                        Keycode::Right => {
                            terminal.clear().unwrap();
                            app.handle_press(Button::Button1(ButtonPressType::Short));
                        }
                        Keycode::Up => {
                            let frame = TelemetryFrame::debug_pump_on_frame();
                            app.update_telemetry(frame);
                        }
                        Keycode::Down => {
                            let frame = TelemetryFrame::debug_frame();
                            app.update_telemetry(frame);
                        }
                        Keycode::Space => {
                            let frame = TelemetryFrame::debug_no_water_frame();
                            app.update_telemetry(frame);
                        }
                        Keycode::M => {
                            app.handle_event(AppEvent::PublishMqttEvent {
                                topic_suffix: "events".to_string(),
                                payload: "{\"type\":\"manual_sim_event\"}".to_string(),
                            });
                        }
                        Keycode::D => {
                            terminal.clear().unwrap();
                            app.handle_press(Button::Both);
                        }

                        _ => {}
                    }
                }
                _ => {}
            }
        }

        for (topic_suffix, payload) in app.take_outbound_mqtt_messages() {
            publish_mqtt_message(&mut mqtt, &app_config, &topic_suffix, &payload);
        }
    }
}

fn init_simulator_mqtt(cfg: &AppConfig) -> Option<Client> {
    if !cfg.mqtt.enabled {
        return None;
    }

    let (host, port) = parse_mqtt_host_port(&cfg.mqtt.url)?;
    let mut options = MqttOptions::new(cfg.mqtt.client_id.clone(), host, port);
    options.set_keep_alive(Duration::from_secs(30));

    if let Some(username) = cfg.mqtt.username.as_ref() {
        options.set_credentials(username, cfg.mqtt.password.as_deref().unwrap_or_default());
    }

    let (client, mut connection) = Client::new(options, 10);
    std::thread::spawn(move || for _ in connection.iter() {});
    Some(client)
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
    topic_suffix: &str,
    payload: &str,
) {
    let Some(client) = client.as_mut() else {
        return;
    };

    let topic = format!("{}/{}", cfg.mqtt.topic_prefix, topic_suffix);
    let _ = client.publish(topic, QoS::AtMostOnce, false, payload);
}
