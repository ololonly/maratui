<p align="center">
    <img src="assets/rat_barista.png" alt="Rat Chef">
    <br>
    <br>
    <a href="https://github.com/ratatui/ratatui">
        <img src="https://ratatui.rs/built-with-ratatui/badge.svg">
    </a>
</p>

# MaraTUI

A TUI (Terminal User Interface) application for embedded systems, built with Rust and Ratatui. Designed to display telemetry data and control interface for the **Lelit Mara** coffee machine.

## What it does

MaraTUI provides a multi-screen interface for monitoring and controlling coffee machine parameters:

- **Main Screen**: System status with visual display
- **Dashboard**: Real-time telemetry (temperatures, heating, pump status)
- **Graphs**: Temperature charts and data visualization
- **Debug**: UART communication monitoring
- Real-time telemetry parsing from UART
- Interactive button controls for navigation
- Embedded display support (ESP32)

## What's coming

- UI adjustments: lifetime graphs, design improvements, animations
- Configurable Home Assistant integration (cup count data)
- MQTT broker integration (telemetry data and events broadcasting)

## Development

### Simulator (debug)

Before running the simulator, install SDL2 development libraries for your OS. See the
rust-sdl2 installation instructions:
https://docs.rs/crate/sdl2/latest/source/README.md

Then run the simulator with auto-detected platform target:

```
make sim
```

Or use platform-specific cargo aliases directly:

| OS      | Command        |
|---------|----------------|
| Linux   | `cargo sim`    |
| macOS   | `cargo simmac` |
| Windows | `cargo simwin` |

### ESP32 (flash)

```
cargo run --release
```
