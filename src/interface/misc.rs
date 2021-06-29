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

pub use libc::mmap;
pub use std::ffi::c_void;

pub use serde::{Serialize as RustSerialize, Deserialize as RustDeserialize};

pub fn log_from_ptr(buf: *const u8) {
    if let Ok(s) = unsafe{std::ffi::CStr::from_ptr(buf as *const i8).to_str()} {
      log_to_stdout(s);
    }
}
// Print text to stdout
pub fn log_to_stdout(s: &str) {
    print!("{}", s);
}

// Print text to stderr
pub fn log_to_stderr(s: &str) {
    eprintln!("{}", s);
}

pub fn fillrandom(bufptr: *mut u8, count: usize) -> i32 {
    let slice = unsafe{std::slice::from_raw_parts_mut(bufptr, count)};
    let mut f = std::fs::OpenOptions::new().read(true).write(false).open("/dev/urandom").unwrap();
    f.read(slice).unwrap() as i32
}
pub fn fillzero(bufptr: *mut u8, count: usize) -> i32 {
    let slice = unsafe{std::slice::from_raw_parts_mut(bufptr, count)};
    for i in 0..count {slice[i] = 0u8;}
    count as i32
}

// Wrapper to return a dictionary (hashmap)
pub fn new_hashmap<K, V>() -> RustHashMap<K, V> {
    RustHashMap::new()
}

pub unsafe fn charstar_to_ruststr<'a>(cstr: *const i8) -> &'a str {
    std::ffi::CStr::from_ptr(cstr).to_str().unwrap()
}

pub fn libc_mmap(addr: *mut u8, len: usize, prot: i32, flags: i32, fildes: i32, off: i64) -> i32 {
    return ((unsafe{mmap(addr as *mut c_void, len, prot, flags, fildes, off)} as i64) & 0xffffffff) as i32;
}
