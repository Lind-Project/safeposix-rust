// Author: Nicholas Renner
//
// Timer functions for Rust interface. 

use std::{thread, time};

// Get program start time as reference
static now: time::Instant  = time::Instant::now();


// Return time since program has started
pub fn getruntime() -> time::Duration {
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
      let onesec = time::Duration::new(1, 0);
      sleep_ms(onesec);
      println!("{:?}", getruntime());
  }
}
