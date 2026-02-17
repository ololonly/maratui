use crate::app::MaraUiApp;
use crate::button::{Button, ButtonState};
use crate::config::AppConfig;
use crate::state::{AppEvent, ConnectionStatus};
use crate::telemetry::TelemetryFrame;
use mousefood::embedded_graphics::Drawable;
use mousefood::embedded_graphics::image::{Image, ImageRaw, ImageRawBE};
use mousefood::embedded_graphics::prelude::{DrawTarget, Point, RgbColor};
use mousefood::fonts::*;
use mousefood::prelude::*;
use ratatui::Terminal;

use crate::uart_reader::UartReader;
use display_interface_spi::SPIInterface;
use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::hal::delay::Ets;
use esp_idf_svc::hal::gpio::{AnyIOPin, Gpio27, Gpio33, InterruptType, Output, PinDriver};
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
use std::time::Duration;

type DisplayResult<'a> = anyhow::Result<
    Ili9341<
        SPIInterface<SpiDeviceDriver<'a, SpiDriver<'a>>, PinDriver<'a, Gpio27, Output>>,
        PinDriver<'a, Gpio33, Output>,
    >,
>;

fn get_ili9341<'a>(
    spi_p: SPI2,
    dc: esp_idf_svc::hal::gpio::Gpio27,
    mosi: esp_idf_svc::hal::gpio::Gpio13,
    sclk: esp_idf_svc::hal::gpio::Gpio15,
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
    let mosi = peripherals.pins.gpio13;
    let sclk = peripherals.pins.gpio15;
    let cs = Some(peripherals.pins.gpio25);
    let rst = peripherals.pins.gpio33;
    let uart1 = peripherals.uart1;
    let gpio21 = peripherals.pins.gpio21;
    let gpio22 = peripherals.pins.gpio22;
    let button1_pin = peripherals.pins.gpio35;
    let button2_pin = peripherals.pins.gpio0;

    //turn off backlight on idle
    //turn on when button pressed w/ timeout
    let mut display =
        get_ili9341(spi_p, dc, mosi, sclk, cs, rst).expect("Failed to initialize display");

    let loading_screen_data = include_bytes!("../assets/loading_screen.raw");
    let loading_image_raw = ImageRawBE::new(loading_screen_data, 320);

    let loading_image: Image<'_, ImageRaw<'_, Rgb565>> =
        Image::new(&loading_image_raw, Point::zero());

    loading_image.draw(&mut display).unwrap();

    let mut button1 = PinDriver::input(button1_pin).unwrap();
    button1.set_interrupt_type(InterruptType::NegEdge).unwrap();
    let mut button1_state = ButtonState::default();

    let mut button2 = PinDriver::input(button2_pin).unwrap();
    button2.set_interrupt_type(InterruptType::NegEdge).unwrap();
    let mut button2_state = ButtonState::default();

    let config = EmbeddedBackendConfig {
        font_regular: MONO_7X14,
        font_bold: Some(MONO_7X14_BOLD),
        font_italic: Some(MONO_7X14),
        ..Default::default()
    };

    let backend = EmbeddedBackend::new(&mut display, config);
    let mut terminal = Terminal::new(backend).unwrap();

    app.handle_event(AppEvent::WifiStatusChanged(ConnectionStatus::Connecting));
    app.handle_event(AppEvent::MqttStatusChanged(ConnectionStatus::Connecting));

    let (_wifi, mut mqtt_client, mut cup_counter_rx) = init_networking(modem, &app_config);

    app.handle_event(AppEvent::WifiStatusChanged(ConnectionStatus::Connected));
    if mqtt_client.is_some() {
        app.handle_event(AppEvent::MqttStatusChanged(ConnectionStatus::Connected));
    } else {
        app.handle_event(AppEvent::MqttStatusChanged(ConnectionStatus::Disabled));
    }

    let (tx, rx) = std::sync::mpsc::channel::<TelemetryFrame>();

    info!("Initializing UART1: TX=GPIO17, RX=GPIO22, baud=9600");
    let uart_reader = match UartReader::new(uart1, gpio21, gpio22) {
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

    //Display cleanup before main loop
    display.clear(Rgb565::BLACK).unwrap();

    Ets::delay_ms(100);

    loop {
        let button1_pressed = button1.is_low();
        let button2_pressed = button2.is_low();

        if button1_pressed && button2_pressed {
            terminal.clear().unwrap();
            app.handle_press(Button::Both);
            Ets::delay_ms(100);
        } else {
            button1_state.update(button1_pressed, |press_type| {
                terminal.clear().unwrap();
                app.handle_press(Button::Button1(press_type));
            });

            button2_state.update(button2_pressed, |press_type| {
                terminal.clear().unwrap();
                app.handle_press(Button::Button2(press_type));
            });
        }

        while let Ok(telemetry) = rx.try_recv() {
            app.update_telemetry(telemetry);
        }

        if let Some(cup_counter_rx) = cup_counter_rx.as_mut() {
            while let Ok(cups) = cup_counter_rx.try_recv() {
                app.handle_event(AppEvent::CupCounterUpdated { cups });
            }
        }

        for (topic_suffix, payload) in app.take_outbound_mqtt_messages() {
            publish_mqtt_message(&mut mqtt_client, &app_config, &topic_suffix, &payload);
        }

        app.render_image(terminal.backend_mut().display_mut());

        terminal
            .draw(|f| {
                app.draw(f);
            })
            .unwrap();
    }
}

fn init_networking(
    modem: esp_idf_svc::hal::modem::Modem,
    app_config: &AppConfig,
) -> (
    Option<EspWifi<'static>>,
    Option<EspMqttClient<'static>>,
    Option<Receiver<u64>>,
) {
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

    info!("Wi-Fi connected");

    if !app_config.mqtt.enabled {
        info!("MQTT is disabled by MARATUI_MQTT_ENABLED");
        return (Some(esp_wifi), None, None);
    }

    let cup_counter_topic = format!("{}/cup_counter", app_config.mqtt.topic_prefix);
    let callback_topic = cup_counter_topic.clone();
    let (cup_counter_tx, cup_counter_rx) = mpsc::channel::<u64>();
    let (connected_tx, connected_rx) = mpsc::sync_channel::<()>(1);

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
    (Some(esp_wifi), Some(mqtt_client), Some(cup_counter_rx))
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
