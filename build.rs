fn main() {
    let target = std::env::var("TARGET").unwrap_or_default();
    let is_esp32 = target.contains("xtensa") || target.contains("espidf");
    let has_simulator = std::env::var("CARGO_FEATURE_SIMULATOR").is_ok();
    let has_device = std::env::var("CARGO_FEATURE_DEVICE").is_ok();

    if has_simulator && is_esp32 {
        panic!(
            "Simulator is for host only. Use: cargo sim --target <host>\n\
             Example: cargo sim --target x86_64-unknown-linux-gnu"
        );
    }

    if has_device {
        // ESP-IDF env for device build
        embuild::espidf::sysenv::output();
    }
}
