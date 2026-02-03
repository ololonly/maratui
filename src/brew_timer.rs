use std::time::{Duration, Instant};

#[derive(Clone, Copy, Default)]
pub struct BrewTimer {
    started_at: Option<Instant>,
    secs: u64,
}

impl BrewTimer {
    pub const fn new() -> Self {
        Self {
            started_at: None,
            secs: 0,
        }
    }

    pub fn is_running(&self) -> bool {
        self.started_at != None
    }

    pub fn start(&mut self) {
        self.started_at = Some(Instant::now());
    }

    pub fn stop(&mut self) {
        self.secs = self.elapsed_secs();
        self.started_at = None;
    }

    pub fn elapsed(&self) -> Duration {
        self.started_at
            .map(|t| t.elapsed())
            .unwrap_or(Duration::ZERO)
    }

    pub fn elapsed_secs(&self) -> u64 {
        if self.started_at == None {
            self.secs
        } else {
            self.elapsed().as_secs()
        }
    }
}
