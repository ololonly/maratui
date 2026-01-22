![Rat Chef](assets/rat_chef.png)

# MaraTUI

A TUI (Terminal User Interface) application for embedded systems, built with Rust and Ratatui. Designed to display telemetry data and control interface for the **Lelit Mara** coffee machine.

## What it does

MaraTUI provides a multi-screen interface for monitoring and controlling coffee machine parameters:
- **Main Screen**: System status with visual display
- **Dashboard**: Real-time telemetry (temperatures, heating, pump status)
- **Graphs**: Temperature charts and data visualization
- **Debug**: UART communication monitoring

## What's coming

- Real-time telemetry parsing from UART
- Interactive button controls
- Temperature graph visualization
- System status monitoring
- Embedded display support (ESP32)
- Configurable Home Assistant integration (cup count data)
- MQTT broker integration (telemetry data and events broadcasting)
