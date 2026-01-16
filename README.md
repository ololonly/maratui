# MaraTUI

## Сборка

**Preview (Windows):**
```bash
cargo run --features preview --no-default-features
```

**Release (WSL):**
```bash
source $HOME/export-esp.sh
cargo build --release
cargo espflash flash --release
```
