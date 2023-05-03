// Author: Nicholas Renner
//
// Timer functions for Rust interface. 
#![allow(dead_code)]

use std::thread;
use std::time::SystemTime;
use std::sync::Mutex;
pub use std::time::Instant as RustInstant;
pub use std::time::Duration as RustDuration;

use super::lind_kill;

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

struct _AlarmClock {
    pub cageid: Option<u64>,
    pub start_instant: RustInstant,
    pub duration: RustDuration,
    pub is_ticking: bool,
}

pub struct AlarmClock {
    _ac: Mutex<_AlarmClock>,
}

impl AlarmClock {
    pub fn new() -> Self {
        Self {
            _ac: Mutex::new(
                _AlarmClock {
                    cageid: None,
                    start_instant: RustInstant::now(),
                    duration: RustDuration::ZERO,
                    is_ticking: false,
                }
            )
        }
    }

    pub fn lind_alarm(&self, seconds: u32, cageid: u64) -> u32 {
        let mut guard = self._ac.lock().unwrap();
        let remaining_seconds = 0;

        if guard.is_ticking {
            remaining_seconds = guard.duration.saturating_sub(guard.start_instant.elapsed()).as_secs() as u32;
        }

        guard.cageid = Some(cageid);
        guard.start_instant = RustInstant::now();
        guard.duration = RustDuration::from_secs(seconds as u64);
        guard.is_ticking = true;

        Mutex::unlock(guard);
        remaining_seconds
    }

    pub fn tick(&self) {
        while true {
            let mut guard = self._ac.lock().unwrap();

            if guard.is_ticking {
                let remaining_seconds = guard.duration.checked_sub(guard.start_instant.elapsed());

                match remaining_seconds {
                    Some(_) => (),
                    None    => {
                        if let Some(cageid) = guard.cageid {
                            lind_kill(cageid, 14);
                            guard.cageid = None;
                        }
                    },
                }

                guard.is_ticking = false;
            }

            Mutex::unlock(guard);
            thread::sleep(RustDuration::from_secs(1));
        }
    }
}
