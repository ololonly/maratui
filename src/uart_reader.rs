use crate::telemetry::{TelemetryFrame, parse_uart_line};
use esp_idf_svc::hal::gpio::AnyIOPin;
use esp_idf_svc::hal::peripherals::Peripherals;
use esp_idf_svc::hal::prelude::*;
use esp_idf_svc::hal::uart::config::{Config, DataBits, Parity, StopBits};
use esp_idf_svc::hal::uart::{UART2, UartDriver};
use esp_idf_svc::sys::EspError;
use std::sync::mpsc::Sender;
use std::thread;
use std::time::Duration;

/// UART reader that continuously reads from UART2 and sends parsed telemetry frames
/// through a channel.
pub struct UartReader {
    _uart: UartDriver<'static>,
}

impl UartReader {
    /// Creates a new UART reader configured for UART2.
    ///
    /// # Parameters
    /// - `uart`: UART2 peripheral
    /// - `tx_pin`: GPIO pin for TX (GPIO17)
    /// - `rx_pin`: GPIO pin for RX (GPIO21, note: GPIO18 is used for SPI)
    ///
    /// # Configuration
    /// - Baudrate: 9600
    /// - Data bits: 8
    /// - Stop bits: 1
    /// - Parity: None
    pub fn new(
        uart: UART2,
        tx_pin: impl Into<AnyIOPin>,
        rx_pin: impl Into<AnyIOPin>,
    ) -> Result<Self, EspError> {
        let mut config = Config::new();
        config.baudrate = Hertz(9600);
        config.data_bits = DataBits::DataBits8;
        config.parity = Parity::ParityNone;
        config.stop_bits = StopBits::STOP1;

        let uart = UartDriver::new(
            uart,
            tx_pin.into(),
            rx_pin.into(),
            Option::<AnyIOPin>::None,
            Option::<AnyIOPin>::None,
            &config,
        )?;

        Ok(UartReader { _uart: uart })
    }

    /// Spawns a background task that continuously reads from UART, parses lines,
    /// and sends TelemetryFrame through the channel.
    ///
    /// The task will run until the program terminates. Invalid lines are logged
    /// and ignored.
    ///
    /// This method consumes the UartReader, moving the UART driver into the background thread.
    pub fn spawn_uart_task(self, tx: Sender<TelemetryFrame>) -> Result<(), EspError> {
        // Move UART driver into the thread
        let mut uart = self._uart;
        thread::spawn(move || {
            let mut line_buffer = String::new();
            let mut byte_buffer = [0u8; 1];

            loop {
                // Try to read a byte with a timeout (100ms = 100000 microseconds)
                match uart.read(&mut byte_buffer, 100_000) {
                    Ok(1) => {
                        let byte = byte_buffer[0];

                        // Check for line endings
                        if byte == b'\n' || byte == b'\r' {
                            if !line_buffer.is_empty() {
                                // Try to parse the line
                                match parse_uart_line(&line_buffer) {
                                    Ok(frame) => {
                                        // Send parsed frame through channel
                                        if tx.send(frame).is_err() {
                                            // Receiver dropped, exit task
                                            // Channel closed, exit gracefully
                                            break;
                                        }
                                    }
                                    Err(_e) => {
                                        // Parse error - ignore invalid lines and continue
                                        // Invalid lines are silently ignored
                                    }
                                }
                                line_buffer.clear();
                            }
                            // Skip \r if followed by \n
                            if byte == b'\r' {
                                continue;
                            }
                        } else if byte.is_ascii() || byte == b'\t' {
                            // Add printable ASCII or tab to buffer
                            line_buffer.push(byte as char);

                            // Prevent buffer overflow
                            if line_buffer.len() > 256 {
                                // Buffer overflow - clear to prevent memory issues
                                line_buffer.clear();
                            }
                        }
                        // Ignore non-ASCII bytes
                    }
                    Ok(0) => {
                        // Timeout - no data available, continue
                    }
                    Ok(_) => {
                        // Should not happen with 1-byte buffer
                    }
                    Err(_e) => {
                        // Read error - wait a bit and continue
                        thread::sleep(Duration::from_millis(10));
                    }
                }
            }
        });

        Ok(())
    }
}
