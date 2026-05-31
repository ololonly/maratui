# MaraTUI PCB

Custom ESP32 controller board for the Lelit Mara espresso machine.

## Components

| Component | Part | Notes |
|-----------|------|-------|
| MCU | ESP32 Type-C (custom) | Custom symbol/footprint — not in standard Espressif library |
| Display | ILI9341 with touch | Touch-enabled variant; touch is unused but this is the model available. Non-touch users can swap the footprint. |
| Backlight | GPIO14 | Driven HIGH = on |

## Custom Libraries

Non-standard components are bundled in `lib/` so the project opens without missing library errors on any machine.

| File | Description |
|------|-------------|
| `lib/symbols/esp32_30pin.kicad_sym` | ESP32 30-pin symbol |
| `lib/symbols/tft_320x240.kicad_sym` | ILI9341 320×240 with touch symbol |
| `lib/footprints/maratui.pretty/ESP32_30pin.kicad_mod` | ESP32 footprint |
| `lib/footprints/maratui.pretty/TFT-320x240.kicad_mod` | TFT display footprint |

Libraries are referenced via `${KIPRJMOD}` so paths resolve anywhere after cloning.

## Pin Assignment

| Function | GPIO |
|----------|------|
| SCK/CLK  | 18   |
| MOSI     | 21   |
| DC/RS    | 23   |
| CS       | 19   |
| RST      | 22   |
| Backlight | 14  |
| UART TX  | 17   |
| UART RX  | 16   |
| Button 1 | 12   |

## Manufacturing

Gerber files for fabrication: `gerbers/maratui-v1.0.zip`

Tested with JLCPCB default 2-layer settings.

## Back Silkscreen

> WORKS ON MY MACHINE  
> VOID WARRANTY? ALREADY DONE.
