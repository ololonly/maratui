fn main() {
    export_dotenv_to_rustc_env();

    let target = std::env::var("TARGET").unwrap_or_default();
    let is_esp32 = target.contains("xtensa") || target.contains("espidf");
    let has_simulator = std::env::var("CARGO_FEATURE_SIMULATOR").is_ok();

    if has_simulator && is_esp32 {
        panic!(
            "Simulator is for host only. Use: cargo sim --target <host>\n\
             Example: cargo sim --target x86_64-unknown-linux-gnu"
        );
    }

    #[cfg(feature = "device")]
    {
        embuild::espidf::sysenv::output();
    }
}

fn export_dotenv_to_rustc_env() {
    let dotenv_path = std::path::Path::new(".env");
    println!("cargo:rerun-if-changed={}", dotenv_path.display());

    let Ok(content) = std::fs::read_to_string(dotenv_path) else {
        return;
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
    }
}
