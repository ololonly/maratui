# Home Assistant Integration

MaraTUI can publish Home Assistant MQTT Discovery configs directly from the firmware — no Node-RED or any other bridge required.

## Architecture

```
ESP32 (MaraTUI)
    │  UART telemetry
    │  MQTT (homeassistant/# discovery + state)
    └─► MQTT broker ──────────────────────────────► Home Assistant
                                                          │
                                                    sensors / binary sensors
                                                    auto-registered via Discovery
```

On connect, the firmware publishes retained discovery configs to `homeassistant/sensor/maratui_*/config` and `homeassistant/binary_sensor/maratui_*/config`. Home Assistant picks them up automatically — no manual YAML needed.

## Prerequisites

- MQTT broker reachable by both the ESP32 and Home Assistant (e.g. Mosquitto on the same host)
- Home Assistant with the MQTT integration enabled (Settings → Devices & Services → MQTT)

## Enabling HA Discovery

Add `--features home-assistant` to your build command:

**Flash to device:**
```bash
cargo run --release --features home-assistant
# or
make flash-ha
```

**Run the simulator:**
```bash
cargo simha          # Linux
cargo simmacHA       # macOS
make sim-ha          # auto-detects OS
```

That's it — on first MQTT connect the firmware publishes all discovery configs, and entities appear in HA within seconds.

## MQTT Topics Published by MaraTUI

### `<prefix>/telemetry` — ~1 Hz JSON frame

```json
{
  "mode": "Coffee",
  "sw": "1.10",
  "boiler_now_c": 93,
  "boiler_target_c": 94,
  "hx_now_c": 91,
  "boost_countdown_s": 0,
  "heating_on": false,
  "pump_on": false,
  "no_water_code": null
}
```

### `<prefix>/events` — on-change JSON events

| `type` | Extra fields | Description |
|---|---|---|
| `shot_started` | — | Pump just turned on |
| `shot_ended` | `duration` (int, seconds) | Pump turned off; duration of the shot |
| `shot_aborted` | `duration` (int, seconds) | Pump ran < 10 s (rinse / pre-heat kick) |
| `water_refill_needed` | `code` (int) | Water low detected |
| `water_refill_cleared` | — | Water low cleared |
| `mode_changed` | `from`, `to` (strings) | Machine mode transition |

### `<prefix>/status` — periodic device info (~30 s)

```json
{
  "uptime_s": 3600,
  "wifi_ssid": "MyNetwork",
  "wifi_rssi": -62,
  "ip": "192.168.1.42",
  "free_heap_b": 180000,
  "last_telemetry_age_s": 1
}
```

The topic prefix defaults to `mara` and is configured via `MARATUI_MQTT_TOPIC_PREFIX` in `.env`.

## Home Assistant Entities

All entities appear under a single device named **Lelit Mara X**.

### Sensors

| Entity | Unit | Notes |
|---|---|---|
| Mode | — | Operating mode |
| Firmware Version | — | Diagnostic |
| Boiler Temperature | °C | `device_class: temperature` |
| Boiler Target | °C | `device_class: temperature` |
| HX Temperature | °C | `device_class: temperature` |
| Last Extraction Duration | s | Retained; updated on shot end |
| Extraction Timer | s | Live counter during shot |
| Time Since Last Shot | min | Minutes since pump last stopped |
| Cup Counter | — | See cup counter setup below |
| Uptime | min | Diagnostic |
| Wi-Fi RSSI | dBm | Diagnostic |
| Wi-Fi SSID | — | Diagnostic |
| IP Address | — | Diagnostic |
| Free Heap | kB | Diagnostic |
| Telemetry Age | s | Diagnostic |

### Binary Sensors

| Entity | Notes |
|---|---|
| Heating | `device_class: heat` |
| Pump Active | On while shot is in progress |
| Water Level Low | `device_class: problem` |

## Cup Counter Setup

The cup counter requires a short one-time setup in Home Assistant. The counter value is stored as an HA helper and published back to the broker so the MaraTUI display stays in sync.

### 1. Create a Counter helper

Settings → Devices & Services → Helpers → **+ Create helper** → **Counter**

- Name: `Mara Shots`
- Entity ID: `counter.coffee_counter` (default)
- Initial value: your current count (or 0)

### 2. Import the automation

Settings → Automations → **+ Create automation** → ⋮ menu → **Edit in YAML**

Paste the contents of `docs/ha-automation.yaml`, save, and enable.

The automation triggers on `mara/events` when `type == shot_ended` and `duration > 20 s`, increments the helper, and publishes the new value to `mara/cup_counter` (retained) so both the HA sensor and the ESP32 display update.

### Cup counter flow

```
MaraTUI → mara/events {"type":"shot_ended","duration":35}
               ↓
         HA automation (ha-automation.yaml)
         duration > 20 s → counter.increment(counter.coffee_counter)
                         → mqtt.publish(mara/cup_counter, retained)
               ↓
         ESP32 subscribes to mara/cup_counter → display updates
         HA sensor maratui_cup_counter reads mara/cup_counter → HA updates
```

## Troubleshooting

**Entities don't appear in HA**
- Check that the MQTT integration is enabled (Settings → Devices & Services → MQTT).
- Confirm the broker MaraTUI connects to is the same one HA uses.
- Subscribe to `homeassistant/#` with `mosquitto_sub -t 'homeassistant/#' -v` to verify discovery configs are arriving.
- Power-cycle or reconnect the ESP32 — discovery configs are re-sent on every MQTT connect.

**Values don't update**
- Confirm the ESP32 is connected to Wi-Fi and MQTT (`MARATUI_MQTT_ENABLED=true` in `.env`).
- Check the MaraTUI Debug screen for UART activity and connection status.
- Subscribe to `mara/#` to see raw messages from the device.

**Topic mismatch**
- If you set `MARATUI_MQTT_TOPIC_PREFIX` to something other than `mara`, update the topics in `docs/ha-automation.yaml` accordingly.

**Cup counter not incrementing**
- Verify the automation is enabled and the trigger topic matches your prefix.
- Check HA automation traces (Settings → Automations → Mara Shot Counter → ⋮ → Traces).
