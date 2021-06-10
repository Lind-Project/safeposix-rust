// Author: Nicholas Renner
//
// Misc functions for interface
// Random, locks, etc.
#![allow(dead_code)]

use std::fs::File;
use std::io::Read;
pub use std::collections::HashMap as RustHashMap;

pub use std::sync::RwLock as RustLock;
pub use std::sync::Arc as RustRfc;
pub use std::thread::current as rust_gettid;

// Print text to stdout
pub fn log_to_stdout(s: &str) {
    print!("{}", s);
}

// Print text to stderr
pub fn log_to_stderr(s: &str) {
    eprintln!("{}", s);
}

// Return a string of random bytes with length 1024
pub fn randombytes() -> Vec<u8> {
    let mut f = File::open("/dev/urandom").unwrap();
    let mut buf = vec![0u8; 1024];
    f.read_exact(buf.as_mut_slice()).unwrap();

    return buf;
}

// Wrapper to return a dictionary (hashmap)
pub fn new_hashmap<K, V>() -> RustHashMap<K, V> {
    return RustHashMap::new()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    pub fn misctester() {
        //we clamp the ascii values so that from_utf8 does not fail
        log_to_stdout(std::str::from_utf8(&randombytes().into_iter().map(|x| if x < 128 {x} else {72}).collect::<Vec<u8>>().as_slice()).unwrap());
        let fd_table = RustHashMap::<&str, u32>::new();

    }
}
