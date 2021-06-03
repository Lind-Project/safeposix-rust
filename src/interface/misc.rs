// Author: Nicholas Renner
//
// Misc functions for interface
// Random, locks, etc.

#![feature(once_cell)]
pub use std::lazy::SyncLazy;

use std::fs::File;
use std::io::Read;
use std::collections::HashMap as dict;

pub use std::sync::Mutex as lock;
pub use std::sync::Arc as rct;


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
pub fn new_dict<K, V>() -> dict<K, V> {
    return dict::new()
}

#[cfg(test)]
mod tests {
  use super::*;
  #[test]
  pub fn misctester() {
      log_to_stdout(std::str::from_utf8(&randombytes()).unwrap());
      let fd_table = new_dict();

  }
}
