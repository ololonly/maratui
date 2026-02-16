// src/telemetry.rs
use std::{collections::VecDeque, time::Instant};
use strum::Display;

#[derive(Debug, Display, Default, Clone, Copy, PartialEq, Eq)]
pub enum MachineMode {
    #[strum(to_string = "Coffee")]
    Coffee, // '-' | '+'
    #[strum(to_string = "Steam")]
    SteamS, // 'S'
    #[strum(to_string = "Steam")]
    SteamV, // 'V'
    #[strum(to_string = "Steam")]
    SteamC, // 'C' (as you described)
    #[default]
    Offline, // 'X'
    Unknown(char),
}

impl MachineMode {
    pub fn from_char(c: char) -> Self {
        match c {
            '-' | '+' => MachineMode::Coffee,
            'S' => MachineMode::SteamS,
            'V' => MachineMode::SteamV,
            'C' => MachineMode::SteamC,
            'X' => MachineMode::Offline,
            other => MachineMode::Unknown(other),
        }
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct TelemetryFrame {
    pub raw_string: String,
    pub mode: MachineMode,
    pub sw_version: String,           // e.g. "1.10"
    pub boiler_now_c: u16,            // e.g. 37
    pub boiler_target_c: Option<u16>, // Some(138) or None if Lxx
    pub no_water_code: Option<u16>,   // Some(65) if "L65"
    pub hx_now_c: u16,                // e.g. 35
    pub boost_countdown_s: u16,       // e.g. 0000 -> 0
    pub heating_on: bool,             // "1"/"0"
    pub pump_on: bool,                // "1"/"0"
}

impl TelemetryFrame {
    pub fn debug_frame() -> Self {
        Self {
            raw_string: "C1.10,120,138,090,0000,1,0".to_string(),
            mode: MachineMode::SteamC,
            sw_version: "1.10".to_string(),
            boiler_now_c: 120,
            boiler_target_c: Some(138),
            hx_now_c: 90,
            boost_countdown_s: 0,
            heating_on: true,
            pump_on: false,
            no_water_code: None,
        }
    }

    pub fn debug_pump_on_frame() -> Self {
        Self {
            raw_string: "C1.10,120,138,090,0000,1,1".to_string(),
            mode: MachineMode::SteamC,
            sw_version: "1.10".to_string(),
            boiler_now_c: 120,
            boiler_target_c: Some(138),
            hx_now_c: 90,
            boost_countdown_s: 0,
            heating_on: true,
            pump_on: true,
            no_water_code: None,
        }
    }

    pub fn debug_no_water_frame() -> Self {
        Self {
            raw_string: "C1.10,122,L65,085,0000,0,0".to_string(),
            mode: MachineMode::Coffee,
            sw_version: "1.10".to_string(),
            boiler_now_c: 122,
            boiler_target_c: None,
            hx_now_c: 85,
            boost_countdown_s: 0,
            heating_on: false,
            pump_on: false,
            no_water_code: Some(65),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError {
    Empty,
    BadPrefix,
    WrongFieldCount { got: usize, expected: usize },
    BadNumber { field: &'static str, value: String },
    BadBool { field: &'static str, value: String },
}

/// Parses UART line format:
/// <Mode><Version>,<boiler_now>,<boiler_target_or_Lxx>,<hx_now>,<boost>,<heating>,<pump>
/// Examples:
/// - C1.10,037,138,035,0000,0,1
/// - C1.10,037,L65,035,0000,0,0
pub fn parse_uart_line(line: &str) -> Result<TelemetryFrame, ParseError> {
    let s = line.trim();
    if s.is_empty() {
        return Err(ParseError::Empty);
    }

    // First char is the machine mode
    let mut chars = s.chars();
    let mode_char = chars.next().ok_or(ParseError::Empty)?;
    let mode = MachineMode::from_char(mode_char);

    // Until the first comma is a software version (e.g. "1.10")
    let rest = &s[mode_char.len_utf8()..];
    let comma_pos = rest.find(',').ok_or(ParseError::BadPrefix)?;
    let sw_version = rest[..comma_pos].to_string();

    let after = &rest[comma_pos + 1..];
    let parts: Vec<&str> = after.split(',').collect();

    // Expected 6 fields after version:
    // boiler_now, boiler_target_or_Lxx, hx_now, boost, heating, pump
    if parts.len() != 6 {
        return Err(ParseError::WrongFieldCount {
            got: parts.len(),
            expected: 6,
        });
    }

    let boiler_now_c = parse_u16(parts[0], "boiler_now_c")?;

    let (boiler_target_c, no_water_code) = {
        let t = parts[1].trim();
        if let Some(code) = t.strip_prefix('L') {
            let code = parse_u16(code, "no_water_code")?;
            (None, Some(code))
        } else {
            let target = parse_u16(t, "boiler_target_c")?;
            (Some(target), None)
        }
    };

    let hx_now_c = parse_u16(parts[2], "hx_now_c")?;
    let boost_countdown_s = parse_u16(parts[3], "boost_countdown_s")?;
    let heating_on = parse_bool01(parts[4], "heating_on")?;
    let pump_on = parse_bool01(parts[5], "pump_on")?;

    let raw_string = line.to_string();

    Ok(TelemetryFrame {
        raw_string,
        mode,
        sw_version,
        boiler_now_c,
        boiler_target_c,
        no_water_code,
        hx_now_c,
        boost_countdown_s,
        heating_on,
        pump_on,
    })
}

fn parse_u16(s: &str, field: &'static str) -> Result<u16, ParseError> {
    let t = s.trim();
    t.parse::<u16>().map_err(|_| ParseError::BadNumber {
        field,
        value: t.to_string(),
    })
}

fn parse_bool01(s: &str, field: &'static str) -> Result<bool, ParseError> {
    match s.trim() {
        "0" => Ok(false),
        "1" => Ok(true),
        other => Err(ParseError::BadBool {
            field,
            value: other.to_string(),
        }),
    }
}

/// High-level events derived from telemetry stream.
/// These are perfect triggers for MQTT "events" topics and for UI notifications.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppEvent {
    /// Pump transitioned from OFF -> ON.
    ShotStarted,

    /// Pump transitioned from ON -> OFF, with total ON duration.
    ShotEnded { duration: u64 },

    /// No-water condition appeared (None -> Some(code)) or code changed.
    WaterRefillNeeded { code: u16 },

    /// No-water condition cleared (Some -> None).
    WaterRefillCleared,

    /// Mode changed (e.g. Coffee -> Steam, etc.).
    ModeChanged { from: MachineMode, to: MachineMode },
}

/// Runtime state used to compute derived values (pump duration, transitions).
#[derive(Debug, Clone)]
pub struct MachineState {
    pub on: bool,
    pub last_frame: Option<TelemetryFrame>,
    pub last_update: Option<Instant>,
    pub shot_started_at: Option<Instant>,
    pub target_boiler_data: VecDeque<f64>,
    pub current_boiler_data: VecDeque<f64>,
    pub current_hx_data: VecDeque<f64>,
}

impl Default for MachineState {
    fn default() -> Self {
        Self {
            on: false,
            last_frame: None,
            last_update: None,
            shot_started_at: None,
            target_boiler_data: VecDeque::with_capacity(300),
            current_boiler_data: VecDeque::with_capacity(300),
            current_hx_data: VecDeque::with_capacity(300),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Snapshot {
    pub frame: TelemetryFrame,
    pub is_extracting: bool,
}

/// Updates state with a new telemetry frame and returns:
/// - a UI-friendly snapshot
/// - a list of derived events (shot start/end, refill needed, etc.)
pub fn update_state_with_events(
    state: &mut MachineState,
    frame: TelemetryFrame,
    now: Instant,
) -> (Snapshot, Vec<AppEvent>) {
    let mut events: Vec<AppEvent> = Vec::new();

    // Read previous values (if any) for transition detection
    let (prev_mode, prev_pump_on, prev_no_water) = match state.last_frame.as_ref() {
        Some(prev) => (Some(prev.mode), prev.pump_on, prev.no_water_code),
        None => (None, false, None),
    };

    // 1) Mode change event
    if let Some(pm) = prev_mode {
        if pm != frame.mode {
            events.push(AppEvent::ModeChanged {
                from: pm,
                to: frame.mode,
            });
        }
    }

    // 2) Pump timer + shot start/end events
    match (prev_pump_on, frame.pump_on) {
        (false, true) => {
            // Shot started
            state.shot_started_at = Some(now);
            events.push(AppEvent::ShotStarted);
        }
        (true, true) => {
            // Shot continues
        }
        (true, false) => {
            // Shot ended
            let duration = state
                .shot_started_at
                .map(|started_at| now.saturating_duration_since(started_at).as_secs() as u64)
                .unwrap_or(0);
            state.shot_started_at = None;
            events.push(AppEvent::ShotEnded { duration });
        }
        (false, false) => {
            // No shot
        }
    }

    // 3) Water refill events (Lxx)
    match (prev_no_water, frame.no_water_code) {
        (None, Some(code)) => events.push(AppEvent::WaterRefillNeeded { code }),
        (Some(_), None) => events.push(AppEvent::WaterRefillCleared),
        (Some(prev_code), Some(new_code)) if prev_code != new_code => {
            events.push(AppEvent::WaterRefillNeeded { code: new_code })
        }
        _ => {}
    }

    // Store the new frame/time
    state.last_update = Some(now);
    state.last_frame = Some(frame.clone());

    let is_extracting = frame.pump_on;

    let snapshot = Snapshot {
        frame,
        is_extracting,
    };

    (snapshot, events)
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    #[test]
    fn parse_normal() {
        let f = parse_uart_line("C1.10,037,138,035,0000,1,0").unwrap();
        assert_eq!(f.mode, MachineMode::SteamC);
        assert_eq!(f.sw_version, "1.10");
        assert_eq!(f.boiler_now_c, 37);
        assert_eq!(f.boiler_target_c, Some(138));
        assert_eq!(f.no_water_code, None);
        assert_eq!(f.hx_now_c, 35);
        assert_eq!(f.boost_countdown_s, 0);
        assert_eq!(f.heating_on, true);
        assert_eq!(f.pump_on, false);
    }

    #[test]
    fn parse_no_water() {
        let f = parse_uart_line("C1.10,037,L65,035,0000,0,0").unwrap();
        assert_eq!(f.boiler_target_c, None);
        assert_eq!(f.no_water_code, Some(65));
        assert_eq!(f.heating_on, false);
        assert_eq!(f.pump_on, false);
    }

    #[test]
    fn events_shot_start_and_end() {
        let mut st = MachineState::default();
        let t0 = Instant::now();

        let off = parse_uart_line("C1.10,037,138,035,0000,1,0").unwrap();
        let (_s0, e0) = update_state_with_events(&mut st, off, t0);
        assert!(e0.is_empty());

        let on = parse_uart_line("C1.10,037,138,035,0000,1,1").unwrap();
        let (_s1, e1) = update_state_with_events(&mut st, on, t0 + Duration::from_millis(200));
        assert_eq!(e1, vec![AppEvent::ShotStarted]);

        let on2 = parse_uart_line("C1.10,038,138,035,0000,1,1").unwrap();
        let (_s2, e2) = update_state_with_events(&mut st, on2, t0 + Duration::from_secs(2));
        assert!(e2.is_empty());

        let off2 = parse_uart_line("C1.10,039,138,035,0000,1,0").unwrap();
        let (_s3, e3) = update_state_with_events(&mut st, off2, t0 + Duration::from_secs(3));
        assert_eq!(e3, vec![AppEvent::ShotEnded { duration: 2_800 }]);
    }

    #[test]
    fn events_water_refill_needed_and_cleared() {
        let mut st = MachineState::default();
        let t0 = Instant::now();

        let ok = parse_uart_line("C1.10,037,138,035,0000,1,0").unwrap();
        let (_s0, e0) = update_state_with_events(&mut st, ok, t0);
        assert!(e0.is_empty());

        let nowater = parse_uart_line("C1.10,037,L65,035,0000,0,0").unwrap();
        let (_s1, e1) = update_state_with_events(&mut st, nowater, t0 + Duration::from_millis(100));
        assert_eq!(e1, vec![AppEvent::WaterRefillNeeded { code: 65 }]);

        let ok2 = parse_uart_line("C1.10,037,138,035,0000,1,0").unwrap();
        let (_s2, e2) = update_state_with_events(&mut st, ok2, t0 + Duration::from_millis(200));
        assert_eq!(e2, vec![AppEvent::WaterRefillCleared]);
    }

    #[test]
    fn events_mode_changed() {
        let mut st = MachineState::default();
        let t0 = Instant::now();

        let c = parse_uart_line("C1.10,037,138,035,0000,1,0").unwrap();
        let (_s0, e0) = update_state_with_events(&mut st, c, t0);
        assert!(e0.is_empty());

        let x = parse_uart_line("X1.10,037,138,035,0000,0,0").unwrap();
        let (_s1, e1) = update_state_with_events(&mut st, x, t0 + Duration::from_millis(50));
        assert_eq!(
            e1,
            vec![AppEvent::ModeChanged {
                from: MachineMode::SteamC,
                to: MachineMode::Offline
            }]
        );
    }
}
