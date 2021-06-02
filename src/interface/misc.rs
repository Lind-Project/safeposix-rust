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
pub fn new_dict() -> HashMap<K, V, RandomState> {
    return HashMap::new();
}

// Wrapped Lock
pub struct emulated_lock<T> {
    lock: Arc<Mutex<T>>
}

// Lock constructor
pub fn createlock<T>(data: T) -> emulated_lock {
    let new_lock = emulated_lock{lock: Arc::new(Mutex::new(T))}
    
    return new_lock;
}

// Lock methods
impl<T> emulated_lock<T> {
    pub fn acquire(&self) -> &T {
        return &self.lock.lock().unwrap();
    }

    pub fn release(data: T) {
        drop(data);
    }
}