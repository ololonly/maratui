# Home Assistant Integration via Node-RED

This guide explains how to get Lelit Mara telemetry into Home Assistant using the Node-RED flow included in this repo.

## Architecture

```
ESP32 (MaraTUI)
    │  UART telemetry
    └─► MQTT broker  ──────────────────────────────────┐
            │                                          │
            │  mara/telemetry (JSON, ~1 Hz)            │
            │  mara/events    (JSON, on change)        │
            └─► Node-RED flow                          │
                    │                                  │
                    │  MQTT Discovery + state topics   │
                    └─► Home Assistant                 │
                              │                        │
                              └──────── sensors ───────┘
```

The Node-RED flow acts as a bridge: it subscribes to the raw MaraTUI topics, transforms the data, and publishes Home Assistant MQTT Discovery configs so entities appear automatically — no manual YAML required.

## Prerequisites

- MQTT broker reachable by both the ESP32 and the machine running Node-RED (e.g. Mosquitto on the same host as HA)
- Node-RED with the `node-red-contrib-mqtt-broker` palette (bundled in most HA Node-RED add-ons)
- Home Assistant with the MQTT integration enabled

## MQTT Topics Published by MaraTUI

### `<prefix>/telemetry` — ~1 Hz JSON frame

```json
{
  "mode": "E",
  "sw": "1.1",
  "boiler_now_c": 93.5,
  "boiler_target_c": 94.0,
  "hx_now_c": 91.2,
  "boost_countdown_s": 0,
  "heating_on": false,
  "pump_on": false,
  "no_water_code": null
}
```

| Field | Type | Description |
|---|---|---|
| `mode` | string | Machine operating mode (`E` = espresso, `S` = steam, etc.) |
| `sw` | string | Firmware version string from machine |
| `boiler_now_c` | float | Current boiler temperature, °C |
| `boiler_target_c` | float \| null | Boiler setpoint, °C; null when unavailable |
| `hx_now_c` | float | Heat exchanger temperature, °C |
| `boost_countdown_s` | int | Seconds remaining in boost mode (0 = inactive) |
| `heating_on` | bool | Heating element active |
| `pump_on` | bool | Pump active (shot in progress) |
| `no_water_code` | int \| null | Non-null when water reservoir is low |

The topic prefix defaults to `mara` and is set via `MARATUI_MQTT_TOPIC_PREFIX` in `.env`.

### `<prefix>/events` — on-change JSON events

| `type` | Extra fields | Description |
|---|---|---|
| `shot_started` | — | Pump just turned on |
| `shot_ended` | `duration` (int, seconds) | Pump turned off; duration of the shot |
| `water_refill_needed` | `code` (int) | Water low detected |
| `water_refill_cleared` | — | Water low cleared |
| `mode_changed` | `from`, `to` (strings) | Machine mode transition |

## Importing the Node-RED Flow

1. Open Node-RED (usually `http://<ha-host>:1880`).
2. Click the hamburger menu → **Import**.
3. Paste or upload `docs/node-red-ha-bridge.json`.
4. Click **Import**, then **Deploy**.

On deploy the flow immediately publishes MQTT Discovery configs (retained), so entities appear in HA within a few seconds.

## Configuring the MQTT Broker

The imported flow uses a broker node named **Local MQTT** pointing to `localhost:1883`. If your broker lives elsewhere:

1. Double-click any MQTT node in the flow.
2. Click the pencil icon next to the broker field.
3. Update **Server** and **Port**.
4. Click **Update** → **Done** → **Deploy**.

If your broker requires authentication, fill in **Username** / **Password** in the same broker config dialog.

## Home Assistant Entities

The flow registers the following entities under a single device named **Lelit Mara**:

### Sensors

| Entity | Unit | Notes |
|---|---|---|
| Mode | — | Operating mode character |
| Firmware Version | — | Diagnostic entity |
| Boiler Temperature | °C | `device_class: temperature` |
| Boiler Target | °C | `device_class: temperature` |
| HX Temperature | °C | `device_class: temperature` |
| Last Extraction Duration | s | Updated on shot end, retained |
| Extraction Timer | s | Live counter during shot |
| Time Since Last Shot | min | Minutes since pump last stopped |

### Binary Sensors

| Entity | Notes |
|---|---|
| Heating | `device_class: heat` |
| Pump Active | On while shot is in progress |
| Water Level Low | `device_class: problem` |

## How the Flow Works

```
[Inject on deploy] → [Build HA Discovery] → [MQTT out]
[mara/telemetry]   → [Parse Telemetry]   → [MQTT out]
[mara/events]      → [Handle Events]     → [MQTT out]
```

**Build HA Discovery** — runs once 2 s after deploy. Publishes retained config payloads to `homeassistant/sensor/maratui_*/config` and `homeassistant/binary_sensor/maratui_*/config`.

**Parse Telemetry** — runs on every telemetry frame. Maintains a node context to compute the live extraction timer (seconds since `pump_on` became true) and reads a flow-level `last_shot_end` timestamp set by the events handler to compute time-since-last-shot.

**Handle Events** — runs on every event. Persists `last_extraction_duration` as a retained state, resets the extraction timer to 0 on `shot_started`, and syncs the water-low binary sensor directly from events (more reliable than polling the telemetry field).

## Example HA Automation

Turn on a smart plug 20 minutes before the first shot each morning (requires the **Time Since Last Shot** sensor):

```yaml
alias: Mara warm-up reminder
trigger:
  - platform: time
    at: "07:40:00"
condition:
  - condition: state
    entity_id: binary_sensor.lelit_mara_pump_active
    state: "off"
action:
  - service: notify.mobile_app_your_phone
    data:
      message: "Start the Mara if you want espresso at 8."
```

## Troubleshooting

**Entities don't appear in HA**
- Confirm the MQTT integration is enabled in HA (Settings → Devices & Services → MQTT).
- Check that the broker Node-RED connects to is the same one HA listens on.
- Re-click the **Publish Discovery** inject node manually to resend configs.

**Values don't update**
- Confirm the ESP32 is connected to Wi-Fi and MQTT (`MARATUI_MQTT_ENABLED=true` in `.env`).
- Check the MaraTUI Debug screen for UART activity and connection status.
- In Node-RED, add a **debug** node after `mqtt in` to see raw messages.

**Topic mismatch**
- The flow subscribes to `mara/telemetry` and `mara/events`. If you changed `MARATUI_MQTT_TOPIC_PREFIX`, update the topic fields in both `mqtt in` nodes.
