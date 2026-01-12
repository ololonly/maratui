mod telemetry;
mod ui;

#[cfg(not(feature = "preview"))]
fn main() {
    println!("Desktop preview is disabled.");
    println!("Run preview window with:");
    println!("  cargo run --features preview");
}

#[cfg(feature = "preview")]
fn main() -> Result<(), std::io::Error> {
    preview::run()
}

#[cfg(feature = "preview")]
mod preview {
    use std::time::{Duration, Instant};
    use embedded_graphics::geometry::Size as EgSize;
    use embedded_graphics_simulator::SimulatorDisplay;
    use mousefood::prelude::*;
    use ratatui::{Terminal, widgets::Clear};
    use ratatui::crossterm::event::{self, Event, KeyCode, KeyEventKind};
    use ratatui::crossterm::terminal::{disable_raw_mode, enable_raw_mode};
    use crate::telemetry::{
        AppEvent, MachineState, TelemetryFrame, 
        parse_uart_line, update_state_with_events,
    };
    use crate::ui::{Screen, UiState, render_app};

    // Guard to restore terminal mode on exit (even if panic occurs)
    struct RawModeGuard;
    impl Drop for RawModeGuard {
        fn drop(&mut self) {
            let _ = disable_raw_mode();
        }
    }

    pub fn run() -> Result<(), std::io::Error> {
        // Enable raw mode for keyboard input
        enable_raw_mode()?;
        let _guard = RawModeGuard;

        // T-Display dimensions: 240x135 landscape
        let (width, height) = (240u32, 135u32);
        let mut display: SimulatorDisplay<Bgr565> = 
            SimulatorDisplay::new(EgSize::new(width, height));

        // Create backend + terminal. Must stay alive to keep SDL window open.
        let backend: EmbeddedBackend<'_, SimulatorDisplay<Bgr565>, Bgr565> =
            EmbeddedBackend::new(&mut display, EmbeddedBackendConfig::default());
        let mut terminal = Terminal::new(backend)?;

        // Application state
        let mut ui_state = UiState::default();
        let mut machine_state = MachineState::default();
        
        // For tracking events and debugging
        let mut recent_events: Vec<(Instant, AppEvent)> = Vec::new();
        let max_events = 5; // Keep last 5 events for display

        // Timing
        let start_time = Instant::now();
        let mut last_tick = Instant::now();
        let tick_rate = Duration::from_millis(100); // Update UI every 100ms
        
        // Test data
        let mut cups: u32 = 123;
        let mut step: u64 = 0;

        println!("=== MaraTUI Preview Started ===");
        println!("Controls:");
        println!("  ESC/q - Exit");
        println!("  a/←    - Previous screen");
        println!("  d/→    - Next screen");
        println!("  w      - Toggle debug mode");
        println!("  +      - Increment cups");
        println!("  -      - Decrement cups");
        println!("==============================");
        println!("Make sure SDL window has focus!");

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
                    eprintln!("[KEY] {:?}", key_event.code);

                    match key_event.code {
                        // Exit
                        KeyCode::Esc | KeyCode::Char('q') => {
                            println!("Exiting...");
                            break 'main_loop;
                        }

                        // Navigation: previous screen
                        KeyCode::Char('a') | KeyCode::Left => {
                            let old_screen = ui_state.screen;
                            ui_state.screen = prev_screen(ui_state.screen);
                            eprintln!("[UI] Screen: {:?} -> {:?}", old_screen, ui_state.screen);
                        }

                        // Navigation: next screen
                        KeyCode::Char('d') | KeyCode::Right => {
                            let old_screen = ui_state.screen;
                            ui_state.screen = next_screen(ui_state.screen);
                            eprintln!("[UI] Screen: {:?} -> {:?}", old_screen, ui_state.screen);
                        }

                        // Toggle debug mode
                        KeyCode::Char('w') => {
                            ui_state.show_debug = !ui_state.show_debug;
                            eprintln!("[UI] Debug mode: {}", ui_state.show_debug);
                        }

                        // Manual cup counter (for testing)
                        KeyCode::Char('+') => {
                            cups = cups.saturating_add(1);
                            eprintln!("[UI] Cups: {}", cups);
                        }
                        KeyCode::Char('-') => {
                            cups = cups.saturating_sub(1);
                            eprintln!("[UI] Cups: {}", cups);
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
                let frame: TelemetryFrame = 
                    parse_uart_line(&uart_line)
                        .unwrap_or_else(|_| {
                            eprintln!("[ERROR] Failed to parse UART line: {}", uart_line);
                            fallback_frame()
                        });

                // Update machine state and get events
                let now = Instant::now();
                let (snapshot, events) = 
                    update_state_with_events(&mut machine_state, frame, now);

                // Store recent events for debugging
                for event in events {
                    recent_events.push((now, event.clone()));
                    eprintln!("[EVENT] {:?}", event);
                    
                    // Keep only last N events
                    if recent_events.len() > max_events {
                        recent_events.remove(0);
                    }
                }

                // === RENDERING ===
                terminal.draw(|frame| {
                    // Clear entire area to prevent ghosting
                    frame.render_widget(Clear, frame.area());

                    // Render main UI
                    render_app(
                        frame, 
                        &ui_state, 
                        Some(&snapshot), 
                        Some(cups),
                        &recent_events, // Pass events for debug display
                    );
                })?;
            }

            // Small sleep to prevent 100% CPU usage
            std::thread::sleep(Duration::from_millis(5));
        }

        Ok(())
    }

    /// Get next screen in sequence
    fn next_screen(current: Screen) -> Screen {
        match current {
            Screen::Dashboard => Screen::Details,
            Screen::Details => Screen::History,
            Screen::History => Screen::Dashboard,
        }
    }

    /// Get previous screen in sequence
    fn prev_screen(current: Screen) -> Screen {
        match current {
            Screen::Dashboard => Screen::History,
            Screen::Details => Screen::Dashboard,
            Screen::History => Screen::Details,
        }
    }

    /// Generate dummy UART line for testing
    /// Simulates coffee machine telemetry with varying states
    fn dummy_uart_line(start: Instant, step: u64) -> String {
        let elapsed_secs = start.elapsed().as_secs_f32();

        // Simulate temperature fluctuations
        let boiler_temp = (92.0_f32 + (elapsed_secs * 0.7_f32).sin() * 4.0_f32).round() as i32;
        let boiler_target = if (elapsed_secs as u64 / 10) % 2 == 0 { 93 } else { 94 };
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
