#[allow(clippy::module_inception)]
pub mod telemetry;
pub use telemetry::{
    AppEvent, MachineMode, MachineState, Snapshot, TelemetryFrame, parse_uart_line,
    update_state_with_events,
};
