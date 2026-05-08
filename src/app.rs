use crate::button::Button;
use crate::run_app;
use crate::screens::{Board, Connecting, Dashboard, Debug, Graphs, Rat, Screen};
use crate::state::{AppError, AppEvent, AppStateMachine, ConnectionStatus, GlobalAppState};
use crate::telemetry::TelemetryFrame;
use mousefood::embedded_graphics::Drawable;
use mousefood::embedded_graphics::image::{Image, ImageRaw, ImageRawBE};
use mousefood::embedded_graphics::prelude::{DrawTarget, OriginDimensions, Point};
use mousefood::prelude::Rgb565;
use ratatui::Frame;
use std::time::Instant;

/// Application trait to be implemented by the user
pub trait MaraUiApp {
    /// Draw the UI frame
    fn draw(&self, frame: &mut Frame);

    /// Handle button press events
    fn handle_press(&mut self, button: Button);

    /// Update application state with telemetry data
    fn update_telemetry(&mut self, telemetry: TelemetryFrame);

    /// Handle generic application event
    fn handle_event(&mut self, event: AppEvent);

    /// Process time-based transitions (auto-return after extraction)
    fn tick(&mut self) {}

    /// Returns `true` if the backlight should be on
    fn backlight_active(&self) -> bool {
        true
    }

    /// Drain app-generated MQTT outbound messages (`topic_suffix`, `payload`)
    fn take_outbound_mqtt_messages(&mut self) -> Vec<(String, String)>;

    /// Read current network status from app state
    fn connection_statuses(&self) -> (ConnectionStatus, ConnectionStatus);

    /// Current active screen
    fn current_screen(&self) -> Screen;

    fn render_image<D>(&mut self, display: &mut D)
    where
        D: DrawTarget<Color = Rgb565> + OriginDimensions,
        D::Error: core::fmt::Debug;

    /// Run the application
    ///
    /// Default implementation provided. Do not override unless necessary.
    fn run(self)
    where
        Self: Sized,
    {
        run_app(self);
    }
}

/// Main application structure
#[derive(Default)]
pub struct MaraUi {
    /// Global application state
    pub state: GlobalAppState,
}

/// The main application implementation
impl MaraUiApp for MaraUi {
    /// Draw the UI frame
    /// This is being called in the main loop to render the UI
    fn draw(&self, frame: &mut Frame) {
        let area = frame.area();

        // Show connecting screen until first UART frame arrives.
        // Debug screen is always accessible for diagnostics.
        if self.state.machine_state.last_frame.is_none()
            && self.state.current_screen != Screen::Debug
        {
            Connecting::render(&self.state, area, frame);
            return;
        }

        match self.state.current_screen {
            Screen::Main => Rat::render(&self.state, area, frame),
            Screen::Dashboard => Dashboard::render(&self.state, area, frame),
            Screen::Graphs => Graphs::render(&self.state, area, frame),
            Screen::Debug => Debug::render(&self.state, area, frame),
        }
    }

    /// Handle button press events
    fn handle_press(&mut self, button: Button) {
        AppStateMachine::handle_button_press(&mut self.state, button);
    }

    /// Update application state with telemetry data
    fn update_telemetry(&mut self, telemetry: TelemetryFrame) {
        let now = Instant::now();
        AppStateMachine::handle_telemetry_frame(&mut self.state, telemetry, now);
    }

    fn handle_event(&mut self, event: AppEvent) {
        AppStateMachine::handle_event(&mut self.state, event);
    }

    fn take_outbound_mqtt_messages(&mut self) -> Vec<(String, String)> {
        self.state
            .take_outbound_mqtt_messages()
            .into_iter()
            .map(|msg| (msg.topic_suffix, msg.payload))
            .collect()
    }

    fn tick(&mut self) {
        AppStateMachine::tick(&mut self.state, Instant::now());
    }

    fn backlight_active(&self) -> bool {
        self.state.current_screen == Screen::Debug
            || self.state.backlight_should_be_on(Instant::now())
    }

    fn current_screen(&self) -> Screen {
        self.state.current_screen
    }

    fn connection_statuses(&self) -> (ConnectionStatus, ConnectionStatus) {
        (
            self.state.wifi_status.clone(),
            self.state.mqtt_status.clone(),
        )
    }

    fn render_image<D>(&mut self, display: &mut D)
    where
        D: DrawTarget<Color = Rgb565> + OriginDimensions,
        D::Error: core::fmt::Debug,
    {
        // Render rat barista on Main screen (once data arrives) and during connecting/loading
        let is_main_active = self.state.current_screen == Screen::Main
            && self.state.machine_state.last_frame.is_some();
        let is_connecting = self.state.machine_state.last_frame.is_none()
            && self.state.current_screen != Screen::Debug;

        if !is_main_active && !is_connecting {
            return;
        }

        //ffmpeg -f lavfi -i color=black:s=180x180 -i rat_barista.png -filter_complex "[1:v]scale=180:180[scaled];[0:v][scaled]overlay" -f rawvideo -pix_fmt rgb565be -frames:v 1 rat_barista.raw
        let image_data: &[u8] = if is_main_active {
            if let Some(AppError::WaterRefillNeeded { .. }) = self.state.error {
                include_bytes!("../assets/rat_barista_thirsty.raw")
            } else {
                include_bytes!("../assets/rat_barista.raw")
            }
        } else {
            include_bytes!("../assets/rat_barista.raw")
        };

        let mut render_point = Point::new(5, 0);

        if is_connecting {
            render_point.y = -60;
        }

        let image = ImageRawBE::new(image_data, 180);
        let im: Image<'_, ImageRaw<'_, Rgb565>> = Image::new(&image, render_point);
        im.draw(display).unwrap();
    }
}
