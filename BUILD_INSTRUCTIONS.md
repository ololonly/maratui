# Инструкции по сборке и прошивке

## Подготовка окружения в WSL

### 1. Установка espup и toolchain

```bash
# Установите espup
cargo install espup

# Установите ESP toolchain
espup install

# Добавьте в ~/.bashrc или ~/.zshrc:
source $HOME/export-esp.sh
```

### 2. Определение target для вашего ESP модуля

Для ESP32 (XTENSA):
```bash
rustup target add xtensa-esp32-espidf
```

Для ESP32-C3 (RISC-V):
```bash
rustup target add riscv32imc-esp-espidf
```

### 3. Настройка проекта

Убедитесь, что проект находится **внутри WSL файловой системы** (не в `/mnt/c/...`), чтобы избежать проблем с путями.

## Сборка

### Preview версия (для тестирования на Windows)

```bash
cargo build --features preview
cargo run --features preview
```

### Release версия (для прошивки на ESP)

```bash
# Сначала загрузите переменные окружения
source $HOME/export-esp.sh

# Соберите для вашего target (замените на ваш target)
cargo build --release --target=xtensa-esp32-espidf
# или
cargo build --release --target=riscv32imc-esp-espidf
```

## Прошивка

### Установка espflash

```bash
cargo install espflash
```

### Прошивка модуля

```bash
# Убедитесь, что USB устройство доступно в WSL (может потребоваться usbipd-win на Windows)
cargo espflash flash --release --target=xtensa-esp32-espidf
```

## Известные проблемы

1. **Тип дисплея**: В `src/setup.rs` есть TODO для исправления совместимости типов между `mipidsi::Display` и `mousefood::EmbeddedBackend`. Возможно, потребуется использовать адаптер или другой подход.

2. **Размер экрана**: Экран T-Display имеет размер 240x135 пикселей в landscape режиме. Убедитесь, что UI правильно масштабируется.

## Структура проекта

- `src/main.rs` - точка входа, разделяет preview и release версии
- `src/setup.rs` - инициализация ESP32 и дисплея (только release)
- `src/ui_app.rs` - основной UI код (общий для обеих версий)
- `src/telemetry.rs` - парсинг телеметрии
- `src/rat_art.rs` - отрисовка изображений (только preview)

## Следующие шаги

1. Исправить интеграцию mousefood с mipidsi Display
2. Добавить чтение UART для телеметрии в release версии
3. Протестировать на реальном устройстве
4. Оптимизировать размер прошивки
