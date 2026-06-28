pub mod parser;
pub use parser::{
    AppEvent, MachineMode, MachineState, Snapshot, TelemetryFrame, parse_uart_line,
    update_state_with_events,
};
