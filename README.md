<div align="center">

![Rat Chef](assets/rat_barista.png)

</div>

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
