mod rat_art;
mod telemetry;
mod uiApp;

fn main() -> Result<(), std::io::Error> {
    preview::run()
}

mod preview {
    use crate::rat_art::{draw_image_from_file, draw_rat_chef};
    use crate::telemetry::{
        MachineState, TelemetryFrame, parse_uart_line, update_state_with_events,
    };
    use crate::uiApp::UiApp;
    use embedded_graphics::geometry::{Point, Size as EgSize};
    use embedded_graphics_simulator::SimulatorDisplay;
    use mousefood::prelude::*;
    use ratatui::crossterm::event::{self, Event, KeyCode, KeyEventKind};
    use ratatui::crossterm::terminal::{disable_raw_mode, enable_raw_mode};
    use ratatui::{Terminal, widgets::Clear};
    use std::cell::RefCell;
    use std::io::{self, Write};
    use std::rc::Rc;
    use std::time::{Duration, Instant};

    // Guard to restore terminal mode on exit (even if panic occurs)
    struct RawModeGuard;
    impl Drop for RawModeGuard {
        fn drop(&mut self) {
            let _ = disable_raw_mode();
        }
    }

    fn log_stdout(message: &str) {
        let mut out = io::stdout();
        let _ = write!(out, "\r{}\r\n", message);
        let _ = out.flush();
    }

    fn log_stderr(message: &str) {
        let mut out = io::stderr();
        let _ = write!(out, "\r{}\r\n", message);
        let _ = out.flush();
    }

    pub fn run() -> Result<(), std::io::Error> {
        // Enable raw mode for keyboard input
        enable_raw_mode()?;
        let _guard = RawModeGuard;

        // T-Display dimensions: 240x135 landscape
        let (width, height) = (240u32, 135u32);
        let mut display: SimulatorDisplay<Bgr565> =
            SimulatorDisplay::new(EgSize::new(width, height));

        // Shared state for rat animation frame
        let rat_frame = Rc::new(RefCell::new(0usize));
        let rat_frame_clone = rat_frame.clone();

        // Create backend with flush callback that draws rat chef
        let config = EmbeddedBackendConfig {
            flush_callback: Box::new(move |display: &mut SimulatorDisplay<Bgr565>| {
                // Try to load image from file first, fallback to drawing function
                let rat_pos = Point::new(10, 20);
                let frame = *rat_frame_clone.borrow();

                // Try loading from "rat_chef.png" or "rat_chef.bmp" in project root
                // If file not found, fallback to drawing function
                if !draw_image_from_file(display, "rat_chef.png", rat_pos) {
                    if !draw_image_from_file(display, "rat_chef.bmp", rat_pos) {
                        // Fallback to drawing function
                        let _ = draw_rat_chef(display, rat_pos, frame);
                    }
                }
            }),
            ..Default::default()
        };

        let backend: EmbeddedBackend<'_, SimulatorDisplay<Bgr565>, Bgr565> =
            EmbeddedBackend::new(&mut display, config);
        let mut terminal = Terminal::new(backend)?;

        // Application state
        let mut ui_app = UiApp::new();
        let mut machine_state = MachineState::default();

        // Timing
        let start_time = Instant::now();
        let mut last_tick = Instant::now();
        let tick_rate = Duration::from_millis(100); // Update UI every 100ms

        // Test data
        let mut cups: u32 = 123;
        let mut step: u64 = 0;

        log_stdout("=== MaraTUI Preview Started ===");
        log_stdout("Controls:");
        log_stdout("  ESC/q - Exit");
        log_stdout("  a/←    - Previous tab");
        log_stdout("  d/→    - Next tab");
        log_stdout("  w      - Toggle debug mode");
        log_stdout("  +      - Increment cups");
        log_stdout("  -      - Decrement cups");
        log_stdout("==============================");
        log_stdout("Make sure SDL window has focus!");

        'main_loop: loop {
            // === INPUT HANDLING ===
            // Poll for keyboard events from crossterm (non-blocking)
            // Note: SDL window may need focus, but crossterm should still work
            while event::poll(Duration::from_millis(0))? {
                if let Ok(Event::Key(key_event)) = event::read() {
                    // Only process key press events (ignore release/repeat)
                    if key_event.kind != KeyEventKind::Press {
                        continue;
                    }

                    // Debug: print key to console
                    log_stderr(&format!("[KEY] {:?}", key_event.code));

                    match key_event.code {
                        // Exit
                        KeyCode::Esc | KeyCode::Char('q') => {
                            log_stdout("Exiting...");
                            break 'main_loop;
                        }

                        // Navigation: previous screen
                        KeyCode::Char('a') | KeyCode::Left => {
                            ui_app.previous_tab();
                            log_stderr(&format!("[UI] Previous tab"));
                        }

                        // Navigation: next screen
                        KeyCode::Char('d') | KeyCode::Right => {
                            ui_app.next_tab();
                            log_stderr(&format!("[UI] Next tab"));
                        }

                        // Toggle debug mode
                        KeyCode::Char('w') => {
                            ui_app.toggle_debug();
                            log_stderr(&format!("[UI] Debug mode toggled"));
                        }

                        // Manual cup counter (for testing)
                        KeyCode::Char('+') => {
                            cups = cups.saturating_add(1);
                            log_stderr(&format!("[UI] Cups: {}", cups));
                        }
                        KeyCode::Char('-') => {
                            cups = cups.saturating_sub(1);
                            log_stderr(&format!("[UI] Cups: {}", cups));
                        }

                        // Ignore other keys
                        _ => {}
                    }
                }
            }

            // === UPDATE LOGIC ===
            // Update UI at fixed interval
            if last_tick.elapsed() >= tick_rate {
                last_tick = Instant::now();
                step += 1;

                // Generate dummy UART data
                let uart_line = dummy_uart_line(start_time, step);

                // Parse UART line into telemetry frame
                let frame: TelemetryFrame = parse_uart_line(&uart_line).unwrap_or_else(|_| {
                    log_stderr(&format!("[ERROR] Failed to parse UART line: {}", uart_line));
                    fallback_frame()
                });

                // Update machine state and get events
                let now = Instant::now();
                let (snapshot, events) = update_state_with_events(&mut machine_state, frame, now);

                // Update rat animation frame (before moving snapshot)
                *rat_frame.borrow_mut() = snapshot.frame.boost_countdown_s as usize;

                // Update UI app state
                ui_app.update_telemetry(Some(snapshot));
                ui_app.update_cups(Some(cups));

                // Store recent events
                for event in events {
                    ui_app.add_event(event.clone(), now);
                    log_stderr(&format!("[EVENT] {:?}", event));
                }

                // === RENDERING ===
                // Rat is drawn in flush_callback after ratatui renders
                terminal.draw(|frame| {
                    // Clear entire area to prevent ghosting
                    frame.render_widget(Clear, frame.area());

                    // Render UI app
                    frame.render_widget(&ui_app, frame.area());
                })?;
            }

            // Small sleep to prevent 100% CPU usage
            std::thread::sleep(Duration::from_millis(5));
        }

        Ok(())
    }

    /// Generate dummy UART line for testing
    /// Simulates coffee machine telemetry with varying states
    fn dummy_uart_line(start: Instant, step: u64) -> String {
        let elapsed_secs = start.elapsed().as_secs_f32();

        // Simulate temperature fluctuations
        let boiler_temp = (92.0_f32 + (elapsed_secs * 0.7_f32).sin() * 4.0_f32).round() as i32;
        let boiler_target = if (elapsed_secs as u64 / 10) % 2 == 0 {
            93
        } else {
            94
        };
        let hx_temp = boiler_temp.saturating_sub(8);

        // Simulate boost countdown
        let boost = ((step / 5) % 30) as i32;

        // Heating is on when below target
        let heating = if boiler_temp < boiler_target { 1 } else { 0 };

        // Simulate pump cycles (on for ~7 seconds every 20 seconds)
        let pump = {
            let cycle = (elapsed_secs as u64) % 20;
            if (3..=10).contains(&cycle) { 1 } else { 0 }
        };

        // Simulate water refill warning (every 60 seconds, for 5 seconds)
        let no_water = {
            let cycle = (elapsed_secs as u64) % 60;
            (50..=55).contains(&cycle)
        };

        // Simulate mode changes
        let mode = if (elapsed_secs as u64) % 45 == 40 {
            'X' // Offline
        } else {
            'C' // Steam mode
        };

        // Format UART line
        if no_water {
            // Format with water warning (L65)
            format!(
                "{mode}1.10,{:03},L65,{:03},{:04},{heating},{pump}",
                boiler_temp, hx_temp, boost
            )
        } else {
            // Normal format
            format!(
                "{mode}1.10,{:03},{:03},{:03},{:04},{heating},{pump}",
                boiler_temp, boiler_target, hx_temp, boost
            )
        }
    }

    /// Create fallback frame when parsing fails
    fn fallback_frame() -> TelemetryFrame {
        TelemetryFrame {
            mode: crate::telemetry::MachineMode::SteamC,
            sw_version: "1.10".to_string(),
            boiler_now_c: 0,
            boiler_target_c: None,
            no_water_code: None,
            hx_now_c: 0,
            boost_countdown_s: 0,
            heating_on: false,
            pump_on: false,
        }
    }
}
