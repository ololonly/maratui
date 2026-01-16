mod rat_art;
mod telemetry;
mod ui_app;

fn main() -> Result<(), std::io::Error> {
    #[cfg(feature = "preview")]
    {
        preview::run()?;
    }
    
    #[cfg(not(feature = "preview"))]
    {
        release::main();
    }
    
    Ok(())
}

#[cfg(feature = "esp")]
mod release {
    use crate::telemetry::{MachineState, TelemetryFrame, parse_uart_line, update_state_with_events};
    use crate::ui_app::{Screen, UiApp};
    use maratui::button::Button;
    use maratui::setup::App;
    use mousefood::prelude::*;
    use ratatui::widgets::Clear;
    use std::time::{Duration, Instant};

    /// Application state for release version.
    pub struct AppState {
        ui_app: UiApp,
        machine_state: MachineState,
        last_tick: Instant,
        tick_rate: Duration,
    }

    impl Default for AppState {
        fn default() -> Self {
            Self {
                ui_app: UiApp::new(),
                machine_state: MachineState::default(),
                last_tick: Instant::now(),
                tick_rate: Duration::from_millis(100),
            }
        }
    }

    impl App for AppState {
        /// Draw the UI frame.
        fn draw(&self, frame: &mut Frame) {
            // Clear to prevent ghosting
            frame.render_widget(Clear, frame.area());
            // Render UI app
            frame.render_widget(&self.ui_app, frame.area());
        }

        /// Handle button press events.
        fn handle_press(&mut self, button: Button) {
            match button {
                Button::Button1(_) | Button::Button2(_) if button.is_short_press() => {
                    // Short press: navigate tabs
                    if button.is_button1() {
                        self.ui_app.previous_tab();
                    } else {
                        self.ui_app.next_tab();
                    }
                }
                Button::Both => {
                    // Both buttons: toggle debug
                    self.ui_app.toggle_debug();
                }
                _ => {
                    // Long press or other: do nothing for now
                }
            }
        }
    }

    fn main() {
        // For now, just run the app with empty telemetry
        // In real implementation, you would read from UART here
        let mut app = AppState::default();
        
        // TODO: Add UART reading loop here
        // For now, just run the UI loop
        app.run();
    }
}

mod preview {
    use crate::rat_art::{GifAnimation, draw_gif_frame, draw_image_from_file};
    use crate::telemetry::{
        MachineState, TelemetryFrame, parse_uart_line, update_state_with_events,
    };
    use crate::ui_app::{Screen, UiApp};
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

        // Application state - wrapped in Rc<RefCell<>> so flush_callback can access it
        let ui_app = Rc::new(RefCell::new(UiApp::new()));
        let mut machine_state = MachineState::default();

        // Load GIF animation (try to load once at startup)
        let gif_animation = Rc::new(RefCell::new(
            GifAnimation::load_from_file("assets/rat_chef.gif").unwrap_or_else(|e| {
                log_stderr(&format!("Failed to load GIF: {}, using empty animation", e));
                GifAnimation::new()
            }),
        ));

        // Create backend with flush callback that draws rat chef
        let ui_app_clone = Rc::clone(&ui_app);
        let gif_animation_clone = Rc::clone(&gif_animation);
        let config = EmbeddedBackendConfig {
            flush_callback: Box::new(move |display: &mut SimulatorDisplay<Bgr565>| {
                let rat_pos = Point::new(1, 30);
                let screen = ui_app_clone.borrow().state.screen;
                let now = Instant::now();

                if screen == Screen::Main {
                    // Try to draw animated GIF first
                    let mut anim = gif_animation_clone.borrow_mut();
                    if anim.is_loaded() {
                        draw_gif_frame(display, &mut anim, rat_pos, now);
                    } else {
                        // Fallback to static PNG
                        if !draw_image_from_file(display, "assets/rat_chef.png", rat_pos) {
                            // Fallback to drawing function (if implemented)
                        }
                    }
                }
            }),
            ..Default::default()
        };

        let backend: EmbeddedBackend<'_, SimulatorDisplay<Bgr565>, Bgr565> =
            EmbeddedBackend::new(&mut display, config);
        let mut terminal = Terminal::new(backend)?;

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
                            ui_app.borrow_mut().previous_tab();
                            log_stderr(&format!("[UI] Previous tab"));
                        }

                        // Navigation: next screen
                        KeyCode::Char('d') | KeyCode::Right => {
                            ui_app.borrow_mut().next_tab();
                            log_stderr(&format!("[UI] Next tab"));
                        }

                        // Toggle debug mode
                        KeyCode::Char('w') => {
                            ui_app.borrow_mut().toggle_debug();
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

                // Update UI app state
                ui_app.borrow_mut().update_telemetry(Some(snapshot));
                ui_app.borrow_mut().update_cups(Some(cups));

                // Store recent events
                for event in events {
                    ui_app.borrow_mut().add_event(event.clone(), now);
                    log_stderr(&format!("[EVENT] {:?}", event));
                }

                // === RENDERING ===
                // Rat is drawn in flush_callback after ratatui renders
                terminal.draw(|frame| {
                    // Clear entire area to prevent ghosting
                    frame.render_widget(Clear, frame.area());

                    // Render UI app
                    frame.render_widget(&*ui_app.borrow(), frame.area());
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
