// Author: Nicholas Renner
//
// Timer functions for Rust interface. 

use std::{thread, time};


// Create a new timer
pub fn starttimer() -> time::Instant {
    time::Instant::now()
}

// Return time since timer was started
pub fn readtimer(now: time::Instant) -> time::Duration {
    now.elapsed()
}

// Sleep function to sleep for x milliseconds
pub fn sleep_ms(dur: time::Duration) {
    thread::sleep(dur);
}

#[cfg(test)]
mod tests {
  use super::*;
  #[test]
  pub fn naptime() {
      let starttime = starttimer();
      let onesec = time::Duration::new(1, 0);
      sleep_ms(onesec);
      println!("{:?}", readtimer(starttime));
  }
}
