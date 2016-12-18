use std::sync::mpsc::Sender;
use std::time::Duration;
use std::thread;

pub struct Timer<T> {
  tx: Sender<T>,
  signal: T,
  interval_ms: u64
}

impl<T> Timer<T> {
  pub fn new(interval_ms: u64, tx: Sender<T>, signal: T) -> Timer<T> {
    Timer { tx: tx, signal: signal, interval_ms: interval_ms }
  }

  pub fn tick_forever(self) where T: 'static + Send + Clone {
    thread::spawn(move || {
      loop {
        match self.tx.send(self.signal.clone()) {
          Ok(_) => thread::sleep(Duration::from_millis(self.interval_ms)),
          Err(_) => break
        }
      }
    });
  }
}

