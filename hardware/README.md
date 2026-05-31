# MaraTUI PCB

Custom ESP32 controller board for the Lelit Mara espresso machine.

## Components

| Ref | Part | Link |
|-----|------|------|
| U1 | ILI9341 2.8" SPI TFT with touch | [Amazon TR](https://www.amazon.com.tr/2-8in%C3%A7-SPI-Dokunmatik-Ekran-Mod%C3%BCl%C3%BC/dp/B0GWFFJBZN) |
| U2 | ESP32-WROOM-32 | [Amazon TR](https://www.amazon.com.tr/ESP32-Wroom-32-Wifi-Bluetooth-Geli%C5%9Ftirme-Kart%C4%B1/dp/B0BT7SW1LF) |
| SW1 | Tactile switch | |
| J1 | Barrel jack (power) | |
| J2 | 3-pin connector (GND + UART) | |

## Custom Libraries

Non-standard components are bundled in `lib/` so the project opens without missing library errors on any machine.

| File | Description |
|------|-------------|
| `lib/symbols/esp32_30pin.kicad_sym` | ESP32-WROOM-32 symbol |
| `lib/symbols/tft_320x240.kicad_sym` | ILI9341 320×240 symbol |
| `lib/footprints/maratui.pretty/ESP32_30pin.kicad_mod` | ESP32 footprint |
| `lib/footprints/maratui.pretty/TFT-320x240.kicad_mod` | TFT display footprint |


## Manufacturing

Gerber files for fabrication: `gerbers/maratui-v1.0.zip`

Tested with JLCPCB default 2-layer settings.
