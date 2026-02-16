use std::time::Duration;

use crossterm::event::{self, Event as CtEvent, KeyEvent};
use tokio::sync::mpsc;

#[derive(Debug)]
pub enum Event {
    Key(KeyEvent),
    Tick,
}

pub struct EventHandler {
    rx: mpsc::UnboundedReceiver<Event>,
    #[allow(dead_code, reason = "kept alive to prevent channel close")]
    keep_alive: mpsc::UnboundedSender<Event>,
}

impl EventHandler {
    #[must_use]
    pub fn new(tick_rate: Duration) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        let keep_alive = tx.clone();

        tokio::spawn(async move {
            loop {
                if event::poll(tick_rate).unwrap_or(false) {
                    if let Ok(CtEvent::Key(key)) = event::read()
                        && tx.send(Event::Key(key)).is_err()
                    {
                        return;
                    }
                } else if tx.send(Event::Tick).is_err() {
                    return;
                }
            }
        });

        Self { rx, keep_alive }
    }

    pub async fn next(&mut self) -> Option<Event> {
        self.rx.recv().await
    }
}
