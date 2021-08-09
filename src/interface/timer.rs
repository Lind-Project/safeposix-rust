// Author: Nicholas Renner
//
// Timer functions for Rust interface. 
#![allow(dead_code)]

use std::thread;
use std::time::SystemTime;
pub use std::time::Instant as RustInstant;
pub use std::time::Duration as RustDuration;

pub fn timestamp() -> u64 {
    SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs()
}

// Create a new timer
pub fn starttimer() -> RustInstant {
    RustInstant::now()
}

// Return time since timer was started
pub fn readtimer(now: RustInstant) -> RustDuration {
    now.elapsed()
}

// Sleep function to sleep for specified duration
pub fn sleep(dur: RustDuration) {
    thread::sleep(dur);
}

#[cfg(test)]
mod tests {
  use super::*;
  #[test]
  pub fn naptime() {
      let starttime = starttimer();
      let onesec = RustDuration::new(1, 0);
      sleep(onesec);
      println!("{:?}", readtimer(starttime));
  }
}
