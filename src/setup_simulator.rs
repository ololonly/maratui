use crate::app::MaraUiApp;
use crate::button::{Button, ButtonPressType};
use crate::telemetry::TelemetryFrame;
use mousefood::embedded_graphics::prelude::Size;
use mousefood::fonts::*;
use mousefood::prelude::*;
use ratatui::Terminal;

use std::cell::RefCell;
use std::rc::Rc;

use embedded_graphics_simulator::{
    OutputSettingsBuilder, SimulatorDisplay, SimulatorEvent, Window, sdl2::Keycode,
};

pub fn run_app(app: impl MaraUiApp) {
    run_app_simulator(app);
}

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
                        Keycode::Up => {
                            app.update_telemetry(TelemetryFrame::debug_pump_on_frame());
                        }
                        Keycode::Down => {
                            app.update_telemetry(TelemetryFrame::debug_frame());
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        }
    }
}
