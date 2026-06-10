use std::time::Duration;

use color_eyre::eyre::Result;
use crossterm::event::{Event, EventStream, KeyEvent};
use futures::StreamExt;
use tokio::select;
use tokio::time::{Interval, interval};

#[allow(dead_code)]
pub enum AppEvent {
    Tick,
    Key(KeyEvent),
    Resize(u16, u16),
}

pub struct EventHandler {
    events: EventStream,
    tick: Interval,
}

impl EventHandler {
    pub fn new(tick_rate: Duration) -> Self {
        // Create the interval once: a fresh interval's first tick completes
        // immediately, so rebuilding it per call would starve keyboard input.
        Self {
            events: EventStream::new(),
            tick: interval(tick_rate),
        }
    }

    pub async fn next(&mut self) -> Result<AppEvent> {
        select! {
            Some(Ok(event)) = self.events.next() => {
                match event {
                    Event::Key(key) => Ok(AppEvent::Key(key)),
                    Event::Resize(w, h) => Ok(AppEvent::Resize(w, h)),
                    _ => Ok(AppEvent::Tick),
                }
            }
            _ = self.tick.tick() => {
                Ok(AppEvent::Tick)
            }
        }
    }
}
