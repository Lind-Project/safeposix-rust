// Author: Nicholas Renner
//
// Misc functions for interface
// Random, locks, etc.

use std::fs::File;
use std::io::Read;
use std::sync::{Arc, Mutex};
use std::collections::HashMap;

// Print text to stdout
pub fn log_to_stdout(s: &str) {
    print!("{}", s);
}

// Return a string of random bytes with length 1024
pub fn randombytes() -> Vec<u8> {
    let mut f = File::open("/dev/urandom").unwrap();
    let mut buf = vec![0u8; 1024];
    f.read_exact(buf.as_mut_slice()).unwrap();

    return buf;
}

// Wrapper to return a dictionary (hashmap)
pub fn new_dict<K, V>() -> HashMap<K, V> {
    return HashMap::new();
}

// Wrapped Lock
pub struct EmulatedLock<T> {
    lock: Arc<Mutex<T>>
}

// Lock constructor
pub fn createlock<T>(data: T) -> EmulatedLock<T> {
    let new_lock = EmulatedLock{lock: Arc::new(Mutex::new(data))};
    
    return new_lock;
}

// Lock methods
impl<T> EmulatedLock<T> {
    pub fn acquire(&mut self) -> &mut T {
        &mut self.lock.lock().unwrap()
    }

    pub fn release(data: T) {
        drop(data);
    }
}

#[cfg(test)]
mod tests {
  use super::*;
  #[test]
  pub fn misctester() {
      log_to_stdout(std::str::from_utf8(&randombytes()).unwrap());
      let mut locky: EmulatedLock<HashMap<u8, String>> = createlock(new_dict());
      let j = locky.acquire();
      j.insert(1, "foo".to_string());
      j.insert(2, "bar".to_string());
      j.insert(3, "fizz".to_string());
      j.insert(2, "buzz".to_string());
      log_to_stdout(&j.get(&2).unwrap().to_string());
      EmulatedLock::release(j);
  }
}
