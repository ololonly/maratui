use crate::telemetry::{TelemetryFrame, parse_uart_line};
use esp_idf_svc::hal::delay::FreeRtos;
use esp_idf_svc::hal::gpio::AnyIOPin;
use esp_idf_svc::hal::prelude::*;
use esp_idf_svc::hal::uart::config::{Config, DataBits, Parity, StopBits};
use esp_idf_svc::hal::uart::{UART1, UartDriver};
use esp_idf_svc::sys::EspError;
use log::{info, warn};
use std::sync::mpsc::Sender;
use std::thread;

const READ_BUF_SIZE: usize = 64;
const LINE_BUF_MAX: usize = 256;

pub struct UartReader {
    _uart: UartDriver<'static>,
}

impl UartReader {
    /// Creates a new UART reader configured for UART1 at 9600 8N1.
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

    /// Spawns a background task that reads UART in bursts, parses complete lines,
    /// and forwards `TelemetryFrame`s through `tx`.
    ///
    /// Reads up to `READ_BUF_SIZE` bytes per call instead of one byte at a time,
    /// reducing syscall overhead from ~27 calls per line to ~1.
    pub fn spawn_uart_task(self, tx: Sender<TelemetryFrame>) -> Result<(), EspError> {
        let uart = self._uart;
        info!("About to spawn UART thread - thread::spawn called");

        thread::spawn(move || {
            let mut line_buf = String::new();
            let mut read_buf = [0u8; READ_BUF_SIZE];

            info!("UART task started - entering loop");
            FreeRtos::delay_ms(100);

            'outer: loop {
                match uart.remaining_read() {
                    Ok(0) => {
                        FreeRtos::delay_ms(10);
                    }
                    Ok(available) => {
                        let to_read = available.min(READ_BUF_SIZE);
                        match uart.read(&mut read_buf[..to_read], 0) {
                            Ok(0) => {}
                            Ok(n) => {
                                for &byte in &read_buf[..n] {
                                    match byte {
                                        b'\n' | b'\r' => {
                                            if !line_buf.is_empty() {
                                                match parse_uart_line(&line_buf) {
                                                    Ok(telemetry) => {
                                                        info!("UART line: '{}'", &line_buf);
                                                        if tx.send(telemetry).is_err() {
                                                            warn!("UART channel closed, exiting");
                                                            break 'outer;
                                                        }
                                                    }
                                                    Err(_) => {
                                                        warn!("UART parse error: {:?}", line_buf);
                                                    }
                                                }
                                                line_buf.clear();
                                            }
                                        }
                                        _ if byte.is_ascii_graphic()
                                            || byte == b' '
                                            || byte == b'\t' =>
                                        {
                                            line_buf.push(byte as char);
                                            if line_buf.len() > LINE_BUF_MAX {
                                                warn!("UART buffer overflow, clearing");
                                                line_buf.clear();
                                            }
                                        }
                                        _ => {
                                            info!("UART non-ASCII byte: 0x{:02x}", byte);
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                warn!("UART read error: {:?}", e);
                                FreeRtos::delay_ms(10);
                            }
                        }
                    }
                    Err(e) => {
                        warn!("UART remaining_read error: {:?}", e);
                        FreeRtos::delay_ms(10);
                    }
                }
            }

            warn!("UART task exited (should not happen)");
        });

        info!("UART thread spawn returned successfully");
        FreeRtos::delay_ms(200);
        info!("UART task spawn command completed");
        Ok(())
    }
}
