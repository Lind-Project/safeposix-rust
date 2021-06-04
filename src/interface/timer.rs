// Author: Nicholas Renner
//
// Timer functions for Rust interface. 

use std::thread;
pub use std::time:Instant as rust_timer;
pub use std::time:Duration as rust_timeval;

// Create a new timer
pub fn starttimer() -> rust_timer {
    time::Instant::now()
}

// Return time since timer was started
pub fn readtimer(now: rust_timer) -> rust_timeval {
    now.elapsed()
}

// Sleep function to sleep for x milliseconds
pub fn sleep_ms(dur: rust_timeval) {
    thread::sleep(dur);
}

#[cfg(test)]
mod tests {
  use super::*;
  #[test]
  pub fn naptime() {
      let starttime = starttimer();
      let onesec = rust_timeval::new(1, 0);
      sleep_ms(onesec);
      println!("{:?}", readtimer(starttime));
  }
}
