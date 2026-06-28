use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::text::{Line, Text};
use ratatui::widgets::Paragraph;
use ratatui::{Frame, widgets::Widget};
use std::time::{Duration, Instant};

use crate::screens::screen::Board;
use crate::state::GlobalAppState;

const UART_ACTIVITY_FLASH: Duration = Duration::from_millis(250);

fn format_uptime(secs: u64) -> String {
    let days = secs / 86400;
    let hours = (secs % 86400) / 3600;
    let mins = (secs % 3600) / 60;
    if days > 0 {
        format!("{}d{}h{}m", days, hours, mins)
    } else if hours > 0 {
        format!("{}h{}m", hours, mins)
    } else {
        format!("{}m", mins)
    }
}

/// Debug screen showing raw UART telemetry data
#[derive(Default)]
pub struct Debug;

impl Board for Debug {
    fn render(state: &GlobalAppState, area: Rect, frame: &mut Frame) {
        let buf = frame.buffer_mut();

        let [
            net_line,
            dev_line1,
            dev_line2,
            uart_line,
            log_area,
            hint_line,
        ] = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(1),
            Constraint::Length(1),
        ])
        .areas(area);

        let net_text = format!(
            "NET: Wi-Fi={:?} MQTT={:?}",
            state.wifi_status, state.mqtt_status
        );

        let dev = &state.device_info;
        let rssi_str = dev
            .wifi_rssi
            .map(|v| format!("{}dBm", v))
            .unwrap_or_else(|| "?".to_string());
        let ip_str = dev.ip.as_deref().unwrap_or("?");
        let heap_str = dev
            .free_heap_b
            .map(|v| format!("{}kB", v / 1024))
            .unwrap_or_else(|| "N/A".to_string());
        // ~45 chars per line at 7px/char on 320px display
        let dev_text1 = format!("DEV: ssid={} rssi={}", dev.wifi_ssid, rssi_str);

        let now = Instant::now();
        let tele_age_str = state
            .last_uart_frame_at
            .map(|t| format!("{}s", now.saturating_duration_since(t).as_secs()))
            .unwrap_or_else(|| "?".to_string());
        let dev_text2 = format!(
            "ip={} up={} heap={} rx={}",
            ip_str,
            format_uptime(dev.uptime_s),
            heap_str,
            tele_age_str,
        );

        let activity_marker = match state.last_uart_frame_at {
            Some(t) if now.saturating_duration_since(t) < UART_ACTIVITY_FLASH => "●",
            _ => " ",
        };

        let text = match &state.machine_state.last_frame {
            Some(telemetry) => format!("UART{} {}", activity_marker, telemetry.raw_string),
            None => format!("UART{}No data (waiting for connection...)", activity_marker),
        };

        let lines = state.events_log.iter().map(|s| Line::from(s.to_string()));

        Paragraph::new(vec![Line::from(net_text)]).render(net_line, buf);
        Paragraph::new(vec![Line::from(dev_text1)]).render(dev_line1, buf);
        Paragraph::new(vec![Line::from(dev_text2)]).render(dev_line2, buf);
        Paragraph::new(vec![Line::from(text)]).render(uart_line, buf);
        Paragraph::new(Text::from_iter(lines)).render(log_area, buf);
        Paragraph::new(vec![Line::from("Long press to exit debug screen")]).render(hint_line, buf);
    }
}
