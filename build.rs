use std::collections::HashMap;

fn main() {
    let dotenv_vars = export_dotenv_to_rustc_env();

    let target = std::env::var("TARGET").unwrap_or_default();
    let is_esp32 = target.contains("xtensa") || target.contains("espidf");
    let has_simulator = std::env::var("CARGO_FEATURE_SIMULATOR").is_ok();

    if has_simulator && is_esp32 {
        panic!(
            "Simulator is for host only. Use: cargo sim --target <host>\n\
             Example: cargo sim --target x86_64-unknown-linux-gnu"
        );
    }

    if !has_simulator {
        validate_device_env(&dotenv_vars);
    }

    #[cfg(feature = "device")]
    {
        embuild::espidf::sysenv::output();
    }
}

// Require SSID (non-empty) and PASSWORD key presence (may be empty for open networks).
// This mirrors the runtime check in config.rs but fails at compile time instead.
fn validate_device_env(dotenv_vars: &HashMap<String, String>) {
    let from_process = |key: &str| std::env::var(key).ok().filter(|v| !v.is_empty());
    let from_dotenv = |key: &str| dotenv_vars.get(key).cloned().filter(|v| !v.is_empty());

    let ssid_present = from_process("MARATUI_WIFI_SSID")
        .or_else(|| from_dotenv("MARATUI_WIFI_SSID"))
        .is_some();

    // PASSWORD may be empty (open network) — we only require the key to exist.
    let password_present = std::env::var("MARATUI_WIFI_PASSWORD").is_ok()
        || dotenv_vars.contains_key("MARATUI_WIFI_PASSWORD");

    if !ssid_present || !password_present {
        let mut missing = Vec::new();
        if !ssid_present {
            missing.push("MARATUI_WIFI_SSID");
        }
        if !password_present {
            missing.push("MARATUI_WIFI_PASSWORD");
        }
        panic!(
            "\n\nMissing required Wi-Fi settings for device build: {}\n\
             Create a .env file in the project root:\n\n\
             MARATUI_WIFI_SSID=<your-network>\n\
             MARATUI_WIFI_PASSWORD=<your-password>\n",
            missing.join(", ")
        );
    }
}

fn export_dotenv_to_rustc_env() -> HashMap<String, String> {
    let dotenv_path = std::path::Path::new(".env");
    println!("cargo:rerun-if-changed={}", dotenv_path.display());

    let mut vars = HashMap::new();

    let Ok(content) = std::fs::read_to_string(dotenv_path) else {
        return vars;
    };

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        let Some((key, value)) = trimmed.split_once('=') else {
            continue;
        };

        let key = key.trim();
        if !key.starts_with("MARATUI_") {
            continue;
        }

        let mut value = value.trim().to_string();
        if (value.starts_with('"') && value.ends_with('"'))
            || (value.starts_with('\'') && value.ends_with('\''))
        {
            value = value[1..value.len() - 1].to_string();
        }

        println!("cargo:rustc-env={}={}", key, value);
        vars.insert(key.to_string(), value);
    }

    vars
}
