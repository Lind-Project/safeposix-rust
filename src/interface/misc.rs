// Author: Nicholas Renner
//
// Misc functions for interface
// Random, locks, etc.

use std::fs::File;
use std::io::Read;

pub use std::collections::HashMap as RustHashMap;
pub use std::sync::{Arc as RustArc, Mutex as RustMutex};

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
