use anyhow::{Result, anyhow};
use log::warn;

#[derive(Clone, Debug)]
pub struct WifiConfig {
    pub ssid: String,
    pub password: String,
}

#[derive(Clone, Debug)]
pub struct MqttConfig {
    pub enabled: bool,
    pub url: String,
    pub client_id: String,
    pub username: Option<String>,
    pub password: Option<String>,
    pub topic_prefix: String,
}

#[derive(Clone, Debug)]
pub struct AppConfig {
    pub wifi: Option<WifiConfig>,
    pub mqtt: MqttConfig,
}

impl AppConfig {
    pub fn from_env() -> Result<Self> {
        let wifi_ssid = setting("MARATUI_WIFI_SSID");
        let wifi_password = setting("MARATUI_WIFI_PASSWORD");

        let wifi = if let (Some(ssid), Some(password)) = (wifi_ssid, wifi_password) {
            Some(WifiConfig { ssid, password })
        } else {
            #[cfg(not(feature = "simulator"))]
            {
                return Err(anyhow!(
                    "Missing Wi-Fi settings. Define MARATUI_WIFI_SSID and MARATUI_WIFI_PASSWORD"
                ));
            }

            #[cfg(feature = "simulator")]
            {
                None
            }
        };

        let enabled = match setting("MARATUI_MQTT_ENABLED")
            .unwrap_or_else(|| "true".to_string())
            .to_ascii_lowercase()
            .as_str()
        {
            "1" | "true" | "yes" | "on" => true,
            "0" | "false" | "no" | "off" => false,
            other => return Err(anyhow!("Invalid MARATUI_MQTT_ENABLED value: {other}")),
        };

        let mqtt = MqttConfig {
            enabled,
            url: setting("MARATUI_MQTT_URL")
                .unwrap_or_else(|| "mqtt://broker.emqx.io:1883".to_string()),
            client_id: setting("MARATUI_MQTT_CLIENT_ID").unwrap_or_else(|| {
                if cfg!(feature = "simulator") {
                    "maratui-sim".to_string()
                } else {
                    "maratui-esp32".to_string()
                }
            }),
            username: setting("MARATUI_MQTT_USERNAME"),
            password: setting("MARATUI_MQTT_PASSWORD"),
            topic_prefix: setting("MARATUI_MQTT_TOPIC_PREFIX")
                .unwrap_or_else(|| "maratui".to_string()),
        };

        let cfg = Self { wifi, mqtt };
        cfg.warn_insecure();
        Ok(cfg)
    }

    fn warn_insecure(&self) {
        // S1: default public broker leaks telemetry to anyone who knows the topic
        if self.mqtt.url.contains("broker.emqx.io") {
            warn!(
                "MQTT is using the default public broker ({}). \
                 All telemetry is publicly readable. Set MARATUI_MQTT_URL to a private broker.",
                self.mqtt.url
            );
        }

        // S2: credentials sent over plaintext MQTT
        let is_plaintext = self.mqtt.url.starts_with("mqtt://")
            || self.mqtt.url.starts_with("tcp://")
            || self.mqtt.url.starts_with("ws://");
        let has_credentials = self.mqtt.username.is_some() || self.mqtt.password.is_some();
        if self.mqtt.enabled && is_plaintext && has_credentials {
            warn!(
                "MQTT credentials are being sent over a plaintext connection ({}). \
                 Switch to mqtts:// to encrypt credentials in transit.",
                self.mqtt.url
            );
        }
    }
}

fn setting(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .filter(|v| !v.is_empty())
        .or_else(|| build_time_setting(name).map(|v| v.to_string()))
}

fn build_time_setting(name: &str) -> Option<&'static str> {
    match name {
        "MARATUI_WIFI_SSID" => option_env!("MARATUI_WIFI_SSID"),
        "MARATUI_WIFI_PASSWORD" => option_env!("MARATUI_WIFI_PASSWORD"),
        "MARATUI_MQTT_ENABLED" => option_env!("MARATUI_MQTT_ENABLED"),
        "MARATUI_MQTT_URL" => option_env!("MARATUI_MQTT_URL"),
        "MARATUI_MQTT_CLIENT_ID" => option_env!("MARATUI_MQTT_CLIENT_ID"),
        "MARATUI_MQTT_USERNAME" => option_env!("MARATUI_MQTT_USERNAME"),
        "MARATUI_MQTT_PASSWORD" => option_env!("MARATUI_MQTT_PASSWORD"),
        "MARATUI_MQTT_TOPIC_PREFIX" => option_env!("MARATUI_MQTT_TOPIC_PREFIX"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    // Serialize env-mutating tests so parallel test threads don't interfere.
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    /// Set the minimum env vars needed for a predictable config, overriding any build-time baked values.
    fn set_base_env() {
        unsafe {
            std::env::set_var("MARATUI_MQTT_ENABLED", "true");
            std::env::set_var("MARATUI_MQTT_URL", "mqtt://broker.emqx.io:1883");
            std::env::set_var("MARATUI_MQTT_CLIENT_ID", "test-client");
            std::env::set_var("MARATUI_MQTT_TOPIC_PREFIX", "maratui");
            std::env::remove_var("MARATUI_MQTT_USERNAME");
            std::env::remove_var("MARATUI_MQTT_PASSWORD");
            std::env::remove_var("MARATUI_WIFI_SSID");
            std::env::remove_var("MARATUI_WIFI_PASSWORD");
        }
    }

    fn lock() -> std::sync::MutexGuard<'static, ()> {
        ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner())
    }

    #[test]
    fn test_default_config_succeeds_in_simulator() {
        let _lock = lock();
        set_base_env();

        let cfg = AppConfig::from_env().expect("default config should succeed in simulator mode");
        // MQTT fields are fully controlled by set_base_env; wifi may be present if baked into
        // the binary at compile time from a local .env file, so we do not assert on it here.
        assert!(cfg.mqtt.enabled);
        assert_eq!(cfg.mqtt.topic_prefix, "maratui");
    }

    #[test]
    fn test_wifi_config_set_via_env() {
        let _lock = lock();
        set_base_env();
        unsafe {
            std::env::set_var("MARATUI_WIFI_SSID", "my_ssid");
            std::env::set_var("MARATUI_WIFI_PASSWORD", "secret");
        }

        let cfg = AppConfig::from_env().unwrap();
        let wifi = cfg.wifi.expect("wifi config should be present");
        assert_eq!(wifi.ssid, "my_ssid");
        assert_eq!(wifi.password, "secret");
    }

    #[test]
    fn test_mqtt_disabled_via_env() {
        let _lock = lock();
        set_base_env();
        unsafe { std::env::set_var("MARATUI_MQTT_ENABLED", "false") };

        let cfg = AppConfig::from_env().unwrap();
        assert!(!cfg.mqtt.enabled);
    }

    #[test]
    fn test_mqtt_enabled_accepts_all_truthy_values() {
        let _lock = lock();
        for val in &["1", "true", "yes", "on", "TRUE", "YES"] {
            set_base_env();
            unsafe { std::env::set_var("MARATUI_MQTT_ENABLED", val) };
            let cfg = AppConfig::from_env().unwrap();
            assert!(cfg.mqtt.enabled, "expected enabled for value '{val}'");
        }
    }

    #[test]
    fn test_mqtt_enabled_accepts_all_falsy_values() {
        let _lock = lock();
        for val in &["0", "false", "no", "off"] {
            set_base_env();
            unsafe { std::env::set_var("MARATUI_MQTT_ENABLED", val) };
            let cfg = AppConfig::from_env().unwrap();
            assert!(!cfg.mqtt.enabled, "expected disabled for value '{val}'");
        }
    }

    #[test]
    fn test_mqtt_enabled_invalid_value_returns_error() {
        let _lock = lock();
        set_base_env();
        unsafe { std::env::set_var("MARATUI_MQTT_ENABLED", "maybe") };

        let result = AppConfig::from_env();
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Invalid MARATUI_MQTT_ENABLED value"));
    }

    #[test]
    fn test_mqtt_custom_url_and_prefix() {
        let _lock = lock();
        set_base_env();
        unsafe {
            std::env::set_var("MARATUI_MQTT_URL", "mqtt://localhost:1883");
            std::env::set_var("MARATUI_MQTT_TOPIC_PREFIX", "home");
            std::env::set_var("MARATUI_MQTT_CLIENT_ID", "my-device");
        }

        let cfg = AppConfig::from_env().unwrap();
        assert_eq!(cfg.mqtt.url, "mqtt://localhost:1883");
        assert_eq!(cfg.mqtt.topic_prefix, "home");
        assert_eq!(cfg.mqtt.client_id, "my-device");
    }

    #[test]
    fn test_mqtt_credentials_set() {
        let _lock = lock();
        set_base_env();
        unsafe {
            std::env::set_var("MARATUI_MQTT_USERNAME", "user");
            std::env::set_var("MARATUI_MQTT_PASSWORD", "pass");
        }

        let cfg = AppConfig::from_env().unwrap();
        assert_eq!(cfg.mqtt.username.as_deref(), Some("user"));
        assert_eq!(cfg.mqtt.password.as_deref(), Some("pass"));
    }
}
