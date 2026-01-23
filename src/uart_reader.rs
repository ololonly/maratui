use crate::telemetry::{TelemetryFrame, parse_uart_line};
use esp_idf_svc::hal::delay::FreeRtos;
use esp_idf_svc::hal::gpio::AnyIOPin;
use esp_idf_svc::hal::peripherals::Peripherals;
use esp_idf_svc::hal::prelude::*;
use esp_idf_svc::hal::uart::config::{Config, DataBits, Parity, StopBits};
use esp_idf_svc::hal::uart::{UART1, UART2, UartDriver};
use esp_idf_svc::sys::EspError;
use log::{info, warn};
use std::sync::mpsc::Sender;
use std::thread;

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
        uart: UART1,
        tx_pin: impl Into<AnyIOPin>,
        rx_pin: impl Into<AnyIOPin>,
    ) -> Result<Self, EspError> {
        let mut config = Config::new();
        config.baudrate = Hertz(9600);
        config.data_bits = DataBits::DataBits8;
        config.parity = Parity::ParityNone;
        config.stop_bits = StopBits::STOP1;
        // Enable event queue for non-blocking operations
        config.queue_size = 10;

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
        let uart = self._uart;
        info!("About to spawn UART thread - thread::spawn called");

        thread::spawn(move || {
            let mut uart = uart;
            let mut line_buffer = String::new();
            let mut byte_buffer = [0u8; 1];

            // Force immediate log output
            info!("UART task started - entering loop");

            // Small delay to ensure log is written and task is scheduled
            FreeRtos::delay_ms(100);

            loop {
                // Check if data is available before reading to avoid blocking
                // This is a workaround for uart.read() blocking forever
                match uart.remaining_read() {
                    Ok(available) => {
                        if available > 0 {
                            // Data available - try to read
                            let read_result = uart.read(&mut byte_buffer, 1000); // 1ms timeout

                            match read_result {
                                Ok(1) => {
                                    let byte = byte_buffer[0];

                                    // Check for line endings
                                    if byte == b'\n' || byte == b'\r' {
                                        if !line_buffer.is_empty() {
                                            // Send line through channel (clone because we need to clear buffer after)
                                            let telemetry = parse_uart_line(&line_buffer).unwrap();
                                            info!("UART line received: '{}'", &line_buffer);
                                            if tx.send(telemetry).is_err() {
                                                // Receiver dropped, exit task
                                                warn!("UART channel closed, exiting");
                                                break;
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
                                            warn!("UART buffer overflow, clearing");
                                            line_buffer.clear();
                                        }
                                    } else {
                                        // Log non-ASCII bytes
                                        info!("UART non-ASCII byte: 0x{:02x}", byte);
                                    }
                                }
                                Ok(0) => {
                                    // Timeout - no data available (shouldn't happen if remaining_read > 0)
                                    // Continue to next iteration
                                }
                                Ok(_) => {
                                    // Should not happen with 1-byte buffer
                                }
                                Err(e) => {
                                    // Read error - log and wait a bit
                                    warn!("UART read error: {:?}", e);
                                    FreeRtos::delay_ms(10);
                                }
                            }
                        } else {
                            // No data available - delay to prevent busy-waiting
                            FreeRtos::delay_ms(10);
                        }
                    }
                    Err(e) => {
                        // Error checking remaining_read
                        warn!("UART remaining_read error: {:?}", e);
                        FreeRtos::delay_ms(10);
                    }
                }
            }

            warn!("UART task exited (should not happen)");
        });

        info!("UART thread spawn returned successfully");
        // Give thread time to start and log
        FreeRtos::delay_ms(200);
        info!("UART task spawn command completed");
        Ok(())
    }
}
