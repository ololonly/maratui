# MaraTUI - Coffee Machine Telemetry UI

UI для отображения телеметрии кофемашины на ESP32 с T-Display экраном.

## Режимы сборки

### Preview режим (для разработки на Windows)

Запускает симулятор экрана на вашем компьютере для тестирования UI без реального устройства.

```bash
cargo run --features preview --no-default-features
```

**Управление в preview режиме:**
- `ESC` или `q` - выход
- `a` или `←` - предыдущая вкладка
- `d` или `→` - следующая вкладка
- `w` - переключить debug режим
- `+` - увеличить счетчик чашек
- `-` - уменьшить счетчик чашек

### Release режим (для прошивки на ESP32)

Собирается для реального ESP32 модуля. Требует WSL и настройки ESP toolchain.

```bash
# В WSL, после настройки espup и toolchain
cargo build --release
cargo espflash flash --release
```

## Структура проекта

- `src/main.rs` - точка входа, разделяет preview и release версии
- `src/setup.rs` - инициализация ESP32 и дисплея (только release)
- `src/ui_app.rs` - основной UI код (общий для обеих версий)
- `src/telemetry.rs` - парсинг телеметрии из UART
- `src/rat_art.rs` - отрисовка изображений (только preview)
- `src/button.rs` - обработка кнопок

## Известные проблемы

1. **Интеграция дисплея**: В `src/setup.rs` есть TODO для исправления совместимости типов между `mipidsi::Display` и `mousefood::EmbeddedBackend`. Это нужно исправить перед прошивкой на устройство.

## Размер экрана

Экран T-Display: 240x135 пикселей (landscape режим)
