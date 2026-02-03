/// Application setup.
pub mod app;
#[cfg(not(feature = "simulator"))]
mod setup;
#[cfg(not(feature = "simulator"))]
pub use setup::run_app;
#[cfg(feature = "simulator")]
mod setup_simulator;
#[cfg(feature = "simulator")]
pub use setup_simulator::run_app;

/// Button handling.
pub mod button;

/// QOI image widget for Ratatui.
pub mod qoi_widget;

pub mod telemetry;

pub mod screens;

/// State management (FSM).
pub mod state;

#[cfg(not(feature = "simulator"))]
pub mod uart_reader;
