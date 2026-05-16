use crate::app::MaraUiApp;
use crate::button::{Button, ButtonState};
use crate::config::AppConfig;
use crate::screens::Screen;
use crate::state::{AppEvent, ConnectionStatus, DeviceInfo};
use crate::telemetry::TelemetryFrame;
use mousefood::embedded_graphics::prelude::{DrawTarget, RgbColor};
use mousefood::fonts::*;
use mousefood::prelude::*;
use ratatui::Terminal;

use crate::uart_reader::UartReader;
use display_interface_spi::SPIInterface;
use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::hal::gpio::{AnyIOPin, Gpio27, Gpio33, InterruptType, Output, PinDriver, Pull};
use esp_idf_svc::hal::prelude::*;
use esp_idf_svc::hal::spi::{SPI2, SpiConfig, SpiDeviceDriver, SpiDriver};
use esp_idf_svc::ipv4::{
    ClientConfiguration as IpClientConfiguration, Configuration as IpConfiguration,
    DHCPClientSettings,
};
use esp_idf_svc::mqtt::client::{
    EspMqttClient, EventPayload, MqttClientConfiguration, MqttProtocolVersion, QoS,
};
use esp_idf_svc::netif::{EspNetif, NetifConfiguration, NetifStack};
use esp_idf_svc::nvs::EspDefaultNvsPartition;
use esp_idf_svc::wifi::{
    AuthMethod, BlockingWifi, ClientConfiguration, Configuration, EspWifi, WifiDriver,
};
use ili9341::{DisplaySize240x320, Ili9341, Orientation};
use log::{info, warn};
use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant};

type DisplayResult<'a> = anyhow::Result<
    Ili9341<
        SPIInterface<SpiDeviceDriver<'a, SpiDriver<'a>>, PinDriver<'a, Gpio27, Output>>,
        PinDriver<'a, Gpio33, Output>,
    >,
>;

fn get_ili9341<'a>(
    spi_p: SPI2,
    dc: esp_idf_svc::hal::gpio::Gpio27,
    mosi: esp_idf_svc::hal::gpio::Gpio23,
    sclk: esp_idf_svc::hal::gpio::Gpio18,
    cs: Option<esp_idf_svc::hal::gpio::Gpio25>,
    rst: esp_idf_svc::hal::gpio::Gpio33,
) -> DisplayResult<'a> {
    let sdi = Option::<AnyIOPin>::None; // MISO not used for display

    let rst = PinDriver::output(rst).unwrap();
    let dc = PinDriver::output(dc).unwrap();
    let driver_config = Default::default();
    let spi_config = SpiConfig::new()
        .baudrate(Hertz(20_000_000))
        .duplex(esp_idf_svc::hal::spi::config::Duplex::Half)
        .into();
    let spi = SpiDeviceDriver::new_single(spi_p, sclk, mosi, sdi, cs, &driver_config, &spi_config)
        .unwrap();

    let di = SPIInterface::new(spi, dc);

    let mut display = Ili9341::new(
        di,
        rst,
        &mut esp_idf_svc::hal::delay::FreeRtos,
        Orientation::Landscape,
        DisplaySize240x320,
    )
    .unwrap();

    display.clear(Rgb565::BLACK).unwrap();
    Ok(display)
}

pub fn run_app(app: impl MaraUiApp) {
    esp_idf_svc::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();
    run_app_hardware(app);
}

fn run_app_hardware(mut app: impl MaraUiApp) {
    let app_config = AppConfig::from_env().expect("Invalid MARATUI_* configuration");
    let peripherals = Peripherals::take().unwrap();
    let modem = peripherals.modem;
    let spi_p = peripherals.spi2;
    let dc = peripherals.pins.gpio27;
    let mosi = peripherals.pins.gpio23;
    let sclk = peripherals.pins.gpio18;
    let cs = Some(peripherals.pins.gpio25);
    let rst = peripherals.pins.gpio33;
    let uart1 = peripherals.uart1;
    let uart_tx = peripherals.pins.gpio17;
    let uart_rx = peripherals.pins.gpio16;
    let button1_pin = peripherals.pins.gpio32;

    let mut display =
        get_ili9341(spi_p, dc, mosi, sclk, cs, rst).expect("Failed to initialize display");

    let mut backlight = PinDriver::output(peripherals.pins.gpio14).unwrap();
    backlight.set_high().unwrap();

    let mut button1 = PinDriver::input(button1_pin).unwrap();
    button1.set_interrupt_type(InterruptType::NegEdge).unwrap();
    let mut button1_state = ButtonState::default();

    button1.set_pull(Pull::Up).unwrap();

    let config = EmbeddedBackendConfig {
        font_regular: MONO_7X14,
        font_bold: Some(MONO_7X14_BOLD),
        font_italic: Some(MONO_7X14),
        ..Default::default()
    };

    // Create the Ratatui terminal first — loading stages render through it
    let backend = EmbeddedBackend::new(&mut display, config);
    let mut terminal = Terminal::new(backend).unwrap();

    // ── WiFi ────────────────────────────────────────────────────────────────
    app.handle_event(AppEvent::WifiStatusChanged(ConnectionStatus::Connecting));
    app.handle_event(AppEvent::MqttStatusChanged(ConnectionStatus::Connecting));
    app.handle_event(AppEvent::LoadingStage {
        message: "connecting wifi...",
        progress: 20,
    });
    app.render_image(terminal.backend_mut().display_mut());
    terminal.draw(|f| app.draw(f)).unwrap();

    let (mut wifi, _) = init_wifi(modem, &app_config);
    app.handle_event(AppEvent::WifiStatusChanged(ConnectionStatus::Connected));

    // ── MQTT ────────────────────────────────────────────────────────────────
    let (mut mqtt_client, mut cup_counter_rx, mut mqtt_status_rx) = if app_config.mqtt.enabled {
        app.handle_event(AppEvent::LoadingStage {
            message: "putting on hat...",
            progress: 55,
        });
        app.render_image(terminal.backend_mut().display_mut());
        terminal.draw(|f| app.draw(f)).unwrap();

        let stage_start = Instant::now();
        let (client, counter_rx, status_rx) = init_mqtt(&app_config);
        min_stage_delay(stage_start);
        (Some(client), Some(counter_rx), Some(status_rx))
    } else {
        app.handle_event(AppEvent::MqttStatusChanged(ConnectionStatus::Disabled));
        (None, None, None)
    };

    // ── UART ────────────────────────────────────────────────────────────────
    app.handle_event(AppEvent::LoadingStage {
        message: "heating up the machine...",
        progress: 85,
    });
    app.render_image(terminal.backend_mut().display_mut());
    terminal.draw(|f| app.draw(f)).unwrap();

    let stage_start = Instant::now();
    let (tx, rx) = std::sync::mpsc::channel::<TelemetryFrame>();

    info!("Initializing UART1: TX=GPIO17, RX=GPIO16, baud=9600");
    let uart_reader = match UartReader::new(uart1, uart_tx, uart_rx) {
        Ok(reader) => {
            info!("UART1 initialized successfully");
            reader
        }
        Err(e) => {
            warn!("Failed to initialize UART1: {:?}", e);
            panic!("UART initialization failed");
        }
    };

    info!("Spawning UART task");
    match uart_reader.spawn_uart_task(tx) {
        Ok(()) => info!("UART task spawn command completed"),
        Err(e) => {
            warn!("Failed to spawn UART task: {:?}", e);
            panic!("UART task spawn failed");
        }
    }
    min_stage_delay(stage_start);

    // Freeze the bar at 100% with "waiting for machine..." until first UART frame
    app.handle_event(AppEvent::LoadingComplete);

    let boot_time = Instant::now();
    let mut last_status_at: Option<Instant> = None;
    let mut last_wifi_check_at: Option<Instant> = None;
    let mut wifi_reconnect_at: Option<Instant> = None;
    let mut wifi_was_connected = true;
    let mut prev_had_telemetry = false;

    loop {
        app.tick();

        button1_state.update(button1.is_low(), |press_type| {
            let was_on_loading = !app.has_telemetry() && app.current_screen() != Screen::Debug;
            app.handle_press(Button::Button1(press_type));
            if was_on_loading && app.current_screen() == Screen::Debug {
                terminal.clear().unwrap();
            }
        });

        while let Ok(telemetry) = rx.try_recv() {
            app.update_telemetry(telemetry);
        }

        // Clear terminal once when the first UART frame arrives so the loading
        // screen (rat image + connecting block) is fully overwritten.
        let now_has_telemetry = app.has_telemetry();
        if !prev_had_telemetry && now_has_telemetry {
            terminal.clear().unwrap();
        }
        prev_had_telemetry = now_has_telemetry;

        if let Some(cup_counter_rx) = cup_counter_rx.as_mut() {
            while let Ok(cups) = cup_counter_rx.try_recv() {
                app.handle_event(AppEvent::CupCounterUpdated { cups });
            }
        }

        if let Some(status_rx) = mqtt_status_rx.as_mut() {
            while let Ok(status) = status_rx.try_recv() {
                if status == ConnectionStatus::Connected {
                    let topic = format!("{}/cup_counter", app_config.mqtt.topic_prefix);
                    if let Some(client) = mqtt_client.as_mut() {
                        if let Err(e) = client.subscribe(&topic, QoS::AtMostOnce) {
                            warn!("Failed to re-subscribe to {}: {:?}", topic, e);
                        }
                    }
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
                .unwrap_or_default();
            app.handle_event(AppEvent::DeviceInfoUpdated(DeviceInfo {
                wifi_ssid: ssid,
                wifi_rssi: query_wifi_rssi(),
                ip: wifi
                    .sta_netif()
                    .get_ip_info()
                    .ok()
                    .map(|i| i.ip.to_string()),
                uptime_s: now.saturating_duration_since(boot_time).as_secs(),
                free_heap_b: Some(query_free_heap()),
            }));
        }

        let should_check_wifi = last_wifi_check_at
            .map(|t| now.saturating_duration_since(t) >= WIFI_CHECK_INTERVAL)
            .unwrap_or(true);
        if should_check_wifi {
            last_wifi_check_at = Some(now);
            let is_connected = query_wifi_rssi().is_some();
            if is_connected != wifi_was_connected {
                wifi_was_connected = is_connected;
                if is_connected {
                    info!("Wi-Fi reconnected");
                    app.handle_event(AppEvent::WifiStatusChanged(ConnectionStatus::Connected));
                } else {
                    info!("Wi-Fi disconnected");
                    app.handle_event(AppEvent::WifiStatusChanged(ConnectionStatus::Disconnected));
                }
            }
            if !is_connected {
                let should_reconnect = wifi_reconnect_at
                    .map(|t| now.saturating_duration_since(t) >= WIFI_RECONNECT_INTERVAL)
                    .unwrap_or(true);
                if should_reconnect {
                    wifi_reconnect_at = Some(now);
                    info!("Wi-Fi disconnected, attempting reconnect");
                    if let Err(e) = wifi.connect() {
                        warn!("Wi-Fi reconnect failed: {:?}", e);
                    }
                }
            } else {
                wifi_reconnect_at = None;
            }
        }

        for (topic_suffix, payload) in app.take_outbound_mqtt_messages() {
            publish_mqtt_message(&mut mqtt_client, &app_config, &topic_suffix, &payload);
        }

        if app.backlight_active() {
            backlight.set_high().unwrap();
        } else {
            backlight.set_low().unwrap();
        }

        app.render_image(terminal.backend_mut().display_mut());

        terminal
            .draw(|f| {
                app.draw(f);
            })
            .unwrap();

        // Yield to the FreeRTOS scheduler so the UART task and Wi-Fi/MQTT stack
        // get CPU time between render frames.
        esp_idf_svc::hal::delay::FreeRtos::delay_ms(0);
    }
}

const STATUS_INTERVAL: Duration = Duration::from_secs(30);
const WIFI_CHECK_INTERVAL: Duration = Duration::from_secs(10);
const WIFI_RECONNECT_INTERVAL: Duration = Duration::from_secs(15);
const MIN_STAGE_MS: u64 = 800;

fn min_stage_delay(started_at: Instant) {
    let elapsed = started_at.elapsed();
    let min = Duration::from_millis(MIN_STAGE_MS);
    if elapsed < min {
        esp_idf_svc::hal::delay::FreeRtos::delay_ms((min - elapsed).as_millis() as u32);
    }
}

fn query_wifi_rssi() -> Option<i32> {
    let mut ap_info: esp_idf_svc::sys::wifi_ap_record_t = unsafe { core::mem::zeroed() };
    if unsafe { esp_idf_svc::sys::esp_wifi_sta_get_ap_info(&mut ap_info) } == 0 {
        Some(i32::from(ap_info.rssi))
    } else {
        None
    }
}

fn query_free_heap() -> u32 {
    unsafe { esp_idf_svc::sys::esp_get_free_heap_size() }
}

fn init_wifi(
    modem: esp_idf_svc::hal::modem::Modem,
    app_config: &AppConfig,
) -> (EspWifi<'static>, Option<String>) {
    let sys_loop = EspSystemEventLoop::take().expect("Failed to take system event loop");
    let nvs = EspDefaultNvsPartition::take().ok();

    let wifi_cfg = app_config
        .wifi
        .as_ref()
        .expect("Wi-Fi config is required on device");

    let wifi_driver =
        WifiDriver::new(modem, sys_loop.clone(), nvs).expect("Failed to create Wi-Fi driver");
    let mut esp_wifi = EspWifi::wrap_all(
        wifi_driver,
        EspNetif::new_with_conf(&NetifConfiguration {
            ip_configuration: Some(IpConfiguration::Client(IpClientConfiguration::DHCP(
                DHCPClientSettings {
                    hostname: Some("maratui".try_into().expect("hostname too long")),
                },
            ))),
            ..NetifConfiguration::wifi_default_client()
        })
        .expect("Failed to create STA netif"),
        EspNetif::new(NetifStack::Ap).expect("Failed to create AP netif"),
    )
    .expect("Failed to create Wi-Fi");

    {
        let mut wifi =
            BlockingWifi::wrap(&mut esp_wifi, sys_loop.clone()).expect("Failed to wrap Wi-Fi");

        let auth_method = if wifi_cfg.password.is_empty() {
            warn!(
                "MARATUI_WIFI_PASSWORD is empty — connecting to '{}' as an open network (no encryption).",
                wifi_cfg.ssid
            );
            AuthMethod::None
        } else {
            AuthMethod::WPA2Personal
        };

        wifi.set_configuration(&Configuration::Client(ClientConfiguration {
            ssid: wifi_cfg.ssid.as_str().try_into().expect("SSID too long"),
            password: wifi_cfg
                .password
                .as_str()
                .try_into()
                .expect("password too long"),
            auth_method,
            ..Default::default()
        }))
        .expect("Failed to set Wi-Fi config");

        wifi.start().expect("Failed to start Wi-Fi");
        wifi.connect().expect("Failed to connect Wi-Fi");
        wifi.wait_netif_up().expect("Failed to obtain IP");
    }

    let initial_ip = esp_wifi
        .sta_netif()
        .get_ip_info()
        .ok()
        .map(|info| info.ip.to_string());
    info!("Wi-Fi connected, IP: {:?}", initial_ip);

    (esp_wifi, initial_ip)
}

fn init_mqtt(
    app_config: &AppConfig,
) -> (
    EspMqttClient<'static>,
    Receiver<u64>,
    Receiver<ConnectionStatus>,
) {
    let cup_counter_topic = format!("{}/cup_counter", app_config.mqtt.topic_prefix);
    let callback_topic = cup_counter_topic.clone();
    let (cup_counter_tx, cup_counter_rx) = mpsc::channel::<u64>();
    let (connected_tx, connected_rx) = mpsc::sync_channel::<()>(1);
    let (mqtt_status_tx, mqtt_status_rx) = mpsc::channel::<ConnectionStatus>();

    info!("Starting MQTT client: {}", app_config.mqtt.url);
    let mut mqtt_client = EspMqttClient::new_cb(
        &app_config.mqtt.url,
        &MqttClientConfiguration {
            protocol_version: Some(MqttProtocolVersion::V3_1_1),
            client_id: Some(app_config.mqtt.client_id.as_str()),
            username: app_config.mqtt.username.as_deref(),
            password: app_config.mqtt.password.as_deref(),
            reconnect_timeout: Some(Duration::from_secs(2)),
            network_timeout: Duration::from_secs(5),
            keep_alive_interval: Some(Duration::from_secs(30)),
            ..Default::default()
        },
        move |event| match event.payload() {
            EventPayload::Connected(_) => {
                info!("MQTT connected");
                let _ = connected_tx.try_send(());
                let _ = mqtt_status_tx.send(ConnectionStatus::Connected);
            }
            EventPayload::Disconnected => {
                let _ = mqtt_status_tx.send(ConnectionStatus::Connecting);
            }
            EventPayload::Received {
                topic: Some(topic),
                data,
                ..
            } if topic == callback_topic => {
                match core::str::from_utf8(data)
                    .ok()
                    .map(str::trim)
                    .and_then(|s| s.parse::<u64>().ok())
                {
                    Some(cups) => {
                        let _ = cup_counter_tx.send(cups);
                    }
                    None => {
                        warn!("Failed to parse cup counter payload: {:?}", data);
                    }
                }
            }
            payload => {
                info!("MQTT event: {}", payload);
            }
        },
    )
    .expect("Failed to create MQTT client");

    match connected_rx.recv_timeout(Duration::from_secs(10)) {
        Ok(_) => {
            if let Err(e) = mqtt_client.subscribe(&cup_counter_topic, QoS::AtMostOnce) {
                warn!("Failed to subscribe to {}: {:?}", cup_counter_topic, e);
            }
        }
        Err(_) => {
            warn!("MQTT connection timeout after 10s, skipping subscribe");
        }
    }

    info!("MQTT started: {}", app_config.mqtt.url);
    (mqtt_client, cup_counter_rx, mqtt_status_rx)
}

fn publish_mqtt_message(
    client: &mut Option<EspMqttClient<'_>>,
    cfg: &AppConfig,
    topic_suffix: &str,
    payload: &str,
) {
    let Some(client) = client.as_mut() else {
        return;
    };

    let topic = format!("{}/{}", cfg.mqtt.topic_prefix, topic_suffix);
    if let Err(e) = client.publish(&topic, QoS::AtMostOnce, false, payload.as_bytes()) {
        warn!("Failed to publish MQTT message: {:?}", e);
    }
}
