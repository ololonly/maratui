use crate::button::{Button, ButtonPressType, ButtonState};
use crate::telemetry::TelemetryFrame;
use mousefood::embedded_graphics::prelude::{DrawTarget, Point, RgbColor, Size};
use mousefood::embedded_graphics::primitives::Rectangle;
use mousefood::fonts::*;
use mousefood::prelude::*;
use ratatui::{Frame, Terminal};

#[cfg(feature = "simulator")]
use std::cell::RefCell;
#[cfg(feature = "simulator")]
use std::rc::Rc;

#[cfg(feature = "device")]
use crate::uart_reader::UartReader;
#[cfg(feature = "device")]
use display_interface_spi::SPIInterface;
#[cfg(feature = "device")]
use esp_idf_svc::hal::delay::Ets;
#[cfg(feature = "device")]
use esp_idf_svc::hal::gpio::{AnyIOPin, Gpio27, Gpio33, InterruptType, Output, PinDriver};
#[cfg(feature = "device")]
use esp_idf_svc::hal::prelude::*;
#[cfg(feature = "device")]
use esp_idf_svc::hal::spi::{SPI2, SpiConfig, SpiDeviceDriver, SpiDriver};
#[cfg(feature = "device")]
use ili9341::{DisplaySize240x320, Ili9341, Orientation};
#[cfg(feature = "device")]
use log::{info, warn};

#[cfg(feature = "device")]
type DisplayResult<'a> = anyhow::Result<
    Ili9341<
        SPIInterface<SpiDeviceDriver<'a, SpiDriver<'a>>, PinDriver<'a, Gpio27, Output>>,
        PinDriver<'a, Gpio33, Output>,
    >,
>;
#[cfg(feature = "simulator")]
use embedded_graphics_simulator::{
    OutputSettingsBuilder, SimulatorDisplay, SimulatorEvent, Window, sdl2::Keycode,
};

/// Application trait to be implemented by the user.
pub trait MaraUiApp {
    /// Draw the UI frame.
    fn draw(&self, frame: &mut Frame);

    /// Handle button press events.
    fn handle_press(&mut self, button: Button);

    fn next_tab(&mut self);
    fn previous_tab(&mut self);

    fn update_telemetry(&mut self, telemetry: TelemetryFrame);

    /// Run the application.
    ///
    /// Default implementation provided. Do not override unless necessary.
    fn run(self)
    where
        Self: Sized,
    {
        run_app(self);
    }
}

#[cfg(feature = "device")]
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
    let spi_config = SpiConfig::new().baudrate(20u32.MHz().into());
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

    display
        .fill_solid(
            &Rectangle::new(Point::new(0, 0), Size::new(320, 240)),
            Rgb565::BLACK,
        )
        .unwrap();
    Ok(display)
}

#[cfg(feature = "device")]
fn run_app(app: impl MaraUiApp) {
    esp_idf_svc::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();
    run_app_hardware(app);
}

#[cfg(feature = "simulator")]
fn run_app(app: impl MaraUiApp) {
    run_app_simulator(app);
}

#[cfg(feature = "device")]
fn run_app_hardware(mut app: impl MaraUiApp) {
    let peripherals = Peripherals::take().unwrap();
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

    let mut display =
        get_ili9341(spi_p, dc, mosi, sclk, cs, rst).expect("Failed to initialize display");

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

    Ets::delay_ms(100);

    loop {
        let button1_pressed = button1.is_low();
        let button2_pressed = button2.is_low();

        if button1_pressed && button2_pressed {
            app.handle_press(Button::Both);
            Ets::delay_ms(100);
        } else {
            button1_state.update(button1_pressed, |press_type| {
                app.handle_press(Button::Button1(press_type));
            });

            button2_state.update(button2_pressed, |press_type| {
                app.handle_press(Button::Button2(press_type));
            });
        }

        while let Ok(telemetry) = rx.try_recv() {
            app.update_telemetry(telemetry);
        }

        terminal
            .draw(|f| {
                app.draw(f);
            })
            .unwrap();
    }
}

#[cfg(feature = "simulator")]
fn run_app_simulator(mut app: impl MaraUiApp) {
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

    app.update_telemetry(TelemetryFrame::default());

    loop {
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
                            app.handle_press(Button::Button2(ButtonPressType::Short));
                        }
                        Keycode::Right => {
                            app.handle_press(Button::Button1(ButtonPressType::Short));
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        }
    }
}
