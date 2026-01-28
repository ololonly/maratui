use std::fs::OpenOptions;
use std::io::Write;
use std::pin::Pin;

use crate::button::{Button, ButtonState};
use crate::telemetry::TelemetryFrame;
use crate::uart_reader::{self, UartReader};
use display_interface_spi::SPIInterface;
use esp_idf_svc::hal::delay::{Delay, Ets};
use esp_idf_svc::hal::gpio::{AnyIOPin, InterruptType, PinDriver};
use esp_idf_svc::hal::prelude::*;
use esp_idf_svc::hal::spi::config::MODE_3;
use esp_idf_svc::hal::spi::{SpiConfig, SpiDeviceDriver, SpiDriverConfig};
use ili9341::{DisplaySize240x320, Ili9341, Orientation};
use log::{info, warn};
use mousefood::embedded_graphics::prelude::{DrawTarget, Point, RgbColor, Size};
use mousefood::embedded_graphics::primitives::Rectangle;
// use mipidsi::models::{ILI9341Rgb565, ST7789};
// use mipidsi::options::{ColorInversion, Orientation, Rotation};
// use mousefood::embedded_graphics::draw_target::DrawTarget;
// use mousefood::embedded_graphics::prelude::*;
// use mousefood::fonts;
use mousefood::prelude::*;

/// Offset to align the display correctly.
const DISPLAY_OFFSET: (u16, u16) = (0, 0);

/// Display size in pixels.
const DISPLAY_SIZE: (u16, u16) = (240, 320);

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

fn run_app(mut app: impl MaraUiApp) {
    esp_idf_svc::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    // let peripherals = Peripherals::take().unwrap();

    // let rst = PinDriver::output(peripherals.pins.gpio13).unwrap();
    // let dc = PinDriver::output(peripherals.pins.gpio2).unwrap();
    // let sclk = peripherals.pins.gpio22;
    // let sdo = peripherals.pins.gpio12;
    // // For display we only need MOSI (write-only), disable MISO to avoid full-duplex limits
    // let sdi = Option::<AnyIOPin>::None;
    // let cs = Some(peripherals.pins.gpio15);
    // let driver_config = Default::default();
    // // Lower initial SPI frequency to stay below full-duplex limit
    // let spi_config = SpiConfig::default().baudrate(20u32.MHz().into());
    // // #region agent log
    // agent_debug_log(
    //     "H1",
    //     "src/setup.rs:63",
    //     "before SpiDeviceDriver::new_single",
    //     r#"{"baud_mhz":20,"mode":"half_or_write_only"}"#,
    // );
    // // #endregion
    // let spi = match SpiDeviceDriver::new_single(
    //     peripherals.spi2,
    //     sclk,
    //     sdo,
    //     sdi,
    //     cs,
    //     &driver_config,
    //     &spi_config,
    // );

    let peripherals = Peripherals::take().unwrap();

    let spi_p = peripherals.spi2; // SPI2 is used for the display

    let dc = peripherals.pins.gpio27; // DC pin for display
    let mosi = peripherals.pins.gpio13; // MOSI pin for display
    let sclk = peripherals.pins.gpio15; // SCK pin for display
    let cs = Some(peripherals.pins.gpio25); // CS pin for display
    let rst = peripherals.pins.gpio33; // Reset pin for display
    let sdi = Option::<AnyIOPin>::None; // MISO not used for display

    let rst = PinDriver::output(rst).unwrap();
    let dc = PinDriver::output(dc).unwrap();
    let driver_config = Default::default();
    let spi_config = SpiConfig::new().baudrate(20u32.MHz().into());
    let spi = SpiDeviceDriver::new_single(spi_p, sclk, mosi, sdi, cs, &driver_config, &spi_config)
        .unwrap();

    // let di = SPIInterface::new(spi, dc);
    let di = SPIInterface::new(spi, dc);

    // let mut display = Ili9341::new(
    //     di,
    //     rst,
    //     &mut Delay::new(100),
    //     Orientation::Landscape,
    //     DisplaySize240x320,
    // )
    // .unwrap();

    let mut display = Ili9341::new(
        di,
        rst,
        &mut esp_idf_svc::hal::delay::FreeRtos,
        Orientation::Portrait,
        DisplaySize240x320,
    )
    .unwrap();

    display
        .fill_solid(
            &Rectangle::new(Point::new(0, 0), Size::new(240, 320)),
            Rgb565::BLUE,
        )
        .unwrap();
    // Turn on display backlight
    // let mut backlight = PinDriver::output(peripherals.pins.gpio21).unwrap();
    // backlight.set_high().unwrap();

    // // Configure SPI
    // let config = SpiConfig::new()
    //     .write_only(true)
    //     .baudrate(80u32.MHz().into())
    //     .data_mode(MODE_3);
    // let spi_device = SpiDeviceDriver::new_single(
    //     peripherals.spi2,
    //     peripherals.pins.gpio22,
    //     peripherals.pins.gpio12,
    //     Some(peripherals.pins.gpio17),
    //     Some(peripherals.pins.gpio15),
    //     &SpiDriverConfig::new(),
    //     &config,
    // )
    // .unwrap();
    // let buffer = Box::leak(Box::new([0_u8; 4096]));
    // let spi_interface = SpiInterface::new(
    //     spi_device,
    //     PinDriver::output(peripherals.pins.gpio2).unwrap(),
    //     buffer,
    // );

    // // Configure display
    // let mut delay = Ets;
    // let mut display = Builder::new(ILI9341Rgb565, spi_interface)
    //     .invert_colors(ColorInversion::Inverted)
    //     .reset_pin(PinDriver::output(peripherals.pins.gpio13).unwrap())
    //     .display_offset(DISPLAY_OFFSET.0, DISPLAY_OFFSET.1)
    //     .display_size(DISPLAY_SIZE.0, DISPLAY_SIZE.1)
    //     .orientation(Orientation::new().rotate(Rotation::Deg0))
    //     .init(&mut delay)
    //     .expect("Failed to init display");

    // display
    //     .clear(Rgb565::BLACK)
    //     .expect("Failed to clear display");

    // Draw the UI
    // Configure buttons
    let mut button1 = PinDriver::input(peripherals.pins.gpio35).unwrap();
    button1.set_interrupt_type(InterruptType::NegEdge).unwrap();
    let mut button1_state = ButtonState::default();

    let mut button2 = PinDriver::input(peripherals.pins.gpio0).unwrap();
    button2.set_interrupt_type(InterruptType::NegEdge).unwrap();
    let mut button2_state = ButtonState::default();

    let backend = EmbeddedBackend::new(&mut display, Default::default());
    let mut terminal = Terminal::new(backend).unwrap();

    // let (tx, rx) = std::sync::mpsc::channel::<TelemetryFrame>();

    // info!("Initializing UART1: TX=GPIO17, RX=GPIO22, baud=9600");
    // let uart_reader = match UartReader::new(
    //     peripherals.uart1,
    //     peripherals.pins.gpio21,
    //     peripherals.pins.gpio22,
    // ) {
    //     Ok(reader) => {
    //         info!("UART1 initialized successfully");
    //         reader
    //     }
    //     Err(e) => {
    //         warn!("Failed to initialize UART1: {:?}", e);
    //         panic!("UART initialization failed");
    //     }
    // };

    // info!("Spawning UART task");
    // match uart_reader.spawn_uart_task(tx) {
    //     Ok(()) => info!("UART task spawn command completed"),
    //     Err(e) => {
    //         warn!("Failed to spawn UART task: {:?}", e);
    //         panic!("UART task spawn failed");
    //     }
    // }

    // Give UART task time to start
    Ets::delay_ms(100);

    let mut frame_counter = 0u32;
    // Enter main event loop
    loop {
        frame_counter += 1;

        // Handle button states
        // let button1_pressed = button1.is_low();
        // let button2_pressed = button2.is_low();

        // if button1_pressed && button2_pressed {
        //     app.handle_press(Button::Both);
        //     Ets::delay_ms(100);
        // } else {
        //     button1_state.update(button1_pressed, |press_type| {
        //         app.handle_press(Button::Button1(press_type));
        //     });

        //     button2_state.update(button2_pressed, |press_type| {
        //         app.handle_press(Button::Button2(press_type));
        //     });
        // }

        // // Check for UART data (non-blocking)
        // while let Ok(telemetry) = rx.try_recv() {
        //     app.update_telemetry(telemetry);
        // }

        info!("Drawing frame");
        terminal
            .draw(|f| {
                app.draw(f);
            })
            .unwrap();
    }
}
