// Author: Nicholas Renner
//
// Timer functions for Rust interface. 

use std::{thread, time};

// Get program start time as reference
let now = time::Instant::now();


// Return time since program has started
pub fn getruntime() -> time::Duration {
    let new_now = Instant::now();

    return new_now.duration_since(now));
}

// Sleep function to sleep for x milliseconds
pub fn sleep_ms(dur: Duration) {
    thread::sleep(dur);
}