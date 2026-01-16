use crate::button::{Button, ButtonState};
use esp_idf_svc::hal::delay::Ets;
use esp_idf_svc::hal::gpio::{AnyIOPin, InterruptType, PinDriver};
use esp_idf_svc::hal::prelude::*;
use esp_idf_svc::hal::spi::config::MODE_3;
use esp_idf_svc::hal::spi::{SpiConfig, SpiDeviceDriver, SpiDriverConfig};
use mipidsi::Builder;
use mipidsi::interface::SpiInterface;
use mipidsi::models::ST7789;
use mipidsi::options::{ColorInversion, Orientation, Rotation};
use mousefood::embedded_graphics::draw_target::DrawTarget;
use mousefood::embedded_graphics::prelude::*;
use mousefood::prelude::*;

/// Offset to align the display correctly.
const DISPLAY_OFFSET: (u16, u16) = (52, 40);

/// Display size in pixels.
const DISPLAY_SIZE: (u16, u16) = (135, 240);

/// Application trait to be implemented by the user.
pub trait App {
    /// Draw the UI frame.
    fn draw(&self, frame: &mut Frame);

    /// Handle button press events.
    fn handle_press(&mut self, button: Button);

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

/// Run the application with the provided [`App`] implementation.
///
/// It initializes the hardware, sets up the display and buttons,
/// and enters the main event loop.
///
/// Please note that this function is blocking and will not return.
/// It is meant to be called once at the start of the program (e.g., in `main`).
///
/// Errors are not handled and will cause a panic if they occur.
fn run_app(mut app: impl App) {
    esp_idf_svc::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    let peripherals = Peripherals::take().unwrap();

    // Turn on display backlight
    let mut backlight = PinDriver::output(peripherals.pins.gpio4).unwrap();
    backlight.set_high().unwrap();

    // Configure SPI
    let config = SpiConfig::new()
        .write_only(true)
        .baudrate(80u32.MHz().into())
        .data_mode(MODE_3);
    let spi_device = SpiDeviceDriver::new_single(
        peripherals.spi2,
        peripherals.pins.gpio18,
        peripherals.pins.gpio19,
        Option::<AnyIOPin>::None,
        Some(peripherals.pins.gpio5),
        &SpiDriverConfig::new(),
        &config,
    )
    .unwrap();
    let buffer = Box::leak(Box::new([0_u8; 4096]));
    let spi_interface = SpiInterface::new(
        spi_device,
        PinDriver::output(peripherals.pins.gpio16).unwrap(),
        buffer,
    );

    // Configure display
    let mut delay = Ets;
    let mut display = Builder::new(ST7789, spi_interface)
        .invert_colors(ColorInversion::Inverted)
        .reset_pin(PinDriver::output(peripherals.pins.gpio23).unwrap())
        .display_offset(DISPLAY_OFFSET.0, DISPLAY_OFFSET.1)
        .display_size(DISPLAY_SIZE.0, DISPLAY_SIZE.1)
        .orientation(Orientation::new().rotate(Rotation::Deg90))
        .init(&mut delay)
        .expect("Failed to init display");

    display
        .clear(Rgb565::BLACK)
        .expect("Failed to clear display");

    // Configure buttons
    let mut button1 = PinDriver::input(peripherals.pins.gpio35).unwrap();
    button1.set_interrupt_type(InterruptType::NegEdge).unwrap();
    let mut button1_state = ButtonState::default();

    let mut button2 = PinDriver::input(peripherals.pins.gpio0).unwrap();
    button2.set_interrupt_type(InterruptType::NegEdge).unwrap();
    let mut button2_state = ButtonState::default();

    // Setup Mousefood and Ratatui
    let backend = EmbeddedBackend::new(&mut display, Default::default());
    let mut terminal = Terminal::new(backend).unwrap();

    // Enter main event loop
    loop {
        // Handle button states
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

        // Draw the UI
        terminal
            .draw(|f| {
                app.draw(f);
            })
            .unwrap();
    }
}
