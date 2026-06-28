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
                #[cfg(feature = "simulator")]
                { "maratui-sim".to_string() }
                #[cfg(not(feature = "simulator"))]
                { "maratui-esp32".to_string() }
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
