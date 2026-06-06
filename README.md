<p align="center">
    <img src="assets/docs/rat_barista_full.png" alt="MaraTUI loading screen">
    <br>
    <br>
    <a href="https://github.com/ratatui/ratatui">
        <img src="https://ratatui.rs/built-with-ratatui/badge.svg">
    </a>
</p>

# MaraTUI

A TUI (Terminal User Interface) application for embedded systems, built with Rust and Ratatui. Designed to display telemetry data and control interface for the **Lelit Mara** espresso machine.

## Screens

Navigation cycles between two main screens with a single button press. The Debug screen is a hidden overlay.

### Dashboard (default)

The primary screen. Shows all real-time machine data:

- **Mode banner** — COFFEE / STEAM / OFFLINE, color-coded
- **Boiler gauge** — current vs. target temperature with warm→ready color zones and target marker
- **HX column** — heat-exchanger temperature with a vertical mini-gauge; ideal zone (90–95°C) is always visible
- **HEAT / PUMP indicators** — live status dots
- **Extraction timer** — large countdown in seconds (BigText), color shifts white→green→yellow as the shot progresses
- **Shot quality label** — post-extraction assessment (UNDEREXTRACTED / GOOD / PERFECT / LONG SHOT / BLONDING) shown after the pump stops
- **Cup counter** — total shots brewed, received via MQTT

### Graphs

Temperature history chart over a **5-minute rolling window** sampled at 1 Hz:

- **Current** boiler temperature (yellow)
- **Target** boiler temperature (red)
- **HX** temperature (blue)

### Debug (hidden overlay)

Accessible only via long press. Shows:

- Wi-Fi and MQTT connection statuses
- Device info: SSID, RSSI, IP address, uptime, free heap
- Live raw UART frame with activity flash indicator (●)
- Last 10 telemetry events log
- Hint: *Long press to exit debug screen*

## Screenshots

| Loading | Dashboard | Debug |
|---------|-----------|-------|
| ![Loading](assets/docs/loading_screen.png) | ![Dashboard](assets/docs/extraction_screen.png) | ![Debug](assets/docs/debug_screen.png) |

## Navigation

There is a single physical button (Button1).

| Press | Action |
|-------|--------|
| Short | Toggle Dashboard ↔ Graphs |
| Short (during loading) | Toggle display backlight |
| Long | Enter / exit Debug overlay |

## What's coming

- Offline mode using NVS storage

## Development

### Configuration (`.env`)

Copy `.env.example` to `.env` and fill in your values (the file is git-ignored):

```bash
cp .env.example .env
```

Key variables:

| Variable | Description |
|---|---|
| `MARATUI_WIFI_SSID` / `_PASSWORD` | Wi-Fi credentials (device only) |
| `MARATUI_MQTT_ENABLED` | Set `true` to enable MQTT publishing |
| `MARATUI_MQTT_URL` | Broker URL, e.g. `mqtt://192.168.1.x:1883` |
| `MARATUI_MQTT_CLIENT_ID` | Unique client identifier |
| `MARATUI_MQTT_TOPIC_PREFIX` | Topic namespace prefix (default: `mara`) |

How it works:

- On ESP32 (`device` feature): `.env` is parsed in [`build.rs`](build.rs), values are embedded at compile time and available in firmware.
- On simulator (`simulator` feature): env values are read at runtime; Wi‑Fi is skipped and host network is used directly for MQTT.

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

Keyboard controls in the simulator:

| Key | Action |
|-----|--------|
| Right / Left | Toggle Dashboard ↔ Graphs |
| D | Toggle Debug overlay |
| Up | Inject pump-on debug frame |
| Down | Inject normal debug frame |
| Space | Inject no-water debug frame |
| M | Publish manual MQTT event |

### Hardware

See [docs/hardware.md](docs/hardware.md) for ESP32 pinout, display wiring.

### Home Assistant integration

Build with `--features home-assistant` to enable direct MQTT Discovery publishing from the firmware — no Node-RED or other bridge required. On every MQTT connect the firmware publishes retained discovery configs and entities appear in HA automatically.

See [docs/home-assistant.md](docs/home-assistant.md) for setup instructions, entity list, and cup counter configuration.

### ESP32 (flash)

```
cargo run --release
```

On device startup the app:

1. Reads config from `MARATUI_*` env values (embedded at build time)
2. Connects to Wi‑Fi
3. Starts MQTT client
4. Publishes telemetry frames to `<MARATUI_MQTT_TOPIC_PREFIX>/telemetry`
5. Publishes telemetry events (shot start/end, water refill, mode change) to `<MARATUI_MQTT_TOPIC_PREFIX>/events`
6. Publishes device status (IP, RSSI, uptime, heap) to `<MARATUI_MQTT_TOPIC_PREFIX>/status`

> Note: current `esp-idf-svc` MQTT API in this project exposes MQTT 3.1 / 3.1.1 protocol selection; the implementation uses 3.1.1.
