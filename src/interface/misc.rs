// Author: Nicholas Renner
//
// Misc functions for interface
// Random, locks, etc.

#![feature(once_cell)]
pub use std::lazy::SyncLazy as rust_global;

use std::fs::File;
use std::io::{self, Read, Write};
use std::collections::HashMap as rust_hashmap;

pub use std::sync::RwLock as rust_lock;
pub use std::sync::Arc as rust_rfc;
pub use std::thread::current::id as rust_gettid;


// Print text to stdout
pub fn log_to_stdout(s: &str) {
    print!("{}", s);
}

// Print text to stderr
pub fn log_to_stderr(s: &str) {
    io::stderr().write_all(s)?;
}

// Return a string of random bytes with length 1024
pub fn randombytes() -> Vec<u8> {
    let mut f = File::open("/dev/urandom").unwrap();
    let mut buf = vec![0u8; 1024];
    f.read_exact(buf.as_mut_slice()).unwrap();

    return buf;
}

// Wrapper to return a dictionary (hashmap)
pub fn new_hashmap<K, V>() -> rust_hashmap<K, V> {
    return rust_hashmap::new()
}

#[cfg(test)]
mod tests {
  use super::*;
  #[test]
  pub fn misctester() {
      log_to_stdout(std::str::from_utf8(&randombytes()).unwrap());
      let fd_table = rust_hashmap();

  }
}
