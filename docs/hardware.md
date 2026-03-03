# Hardware Wiring

Target board: **ESP32 Type-C** (generic) with external **ILI9341** 240x320 TFT display.

## Pinout

### SPI Display (ILI9341)

| Display Pin | ESP32 GPIO | Notes |
|-------------|------------|-------|
| SCK / CLK   | GPIO18     | VSPI CLK |
| SDA / MOSI  | GPIO23     | VSPI MOSI |
| DC / RS     | GPIO27     | Data/Command select |
| CS          | GPIO25     | Chip Select |
| RST / RESET | GPIO33     | Hardware reset |
| LED / BLK   | GPIO14     | Backlight (driven HIGH = on) |
| VCC         | 3.3V       | |
| GND         | GND        | |

### UART (Lelit Mara telemetry)

| Function | ESP32 GPIO | Notes |
|----------|------------|-------|
| TX       | GPIO17     | UART1 TX |
| RX       | GPIO16     | UART1 RX, 9600 baud 8N1 |

### Buttons

| Button   | ESP32 GPIO | Notes |
|----------|------------|-------|
| Button 1 | GPIO32     | Internal pull-up enabled |
| Button 2 | GPIO19     | Internal pull-up enabled |

Buttons connect GPIO to **GND** through a tactile switch (active LOW, NegEdge detection).
Short press: < 500 ms. Long press: 500-2000 ms. Both buttons simultaneously: special action.
