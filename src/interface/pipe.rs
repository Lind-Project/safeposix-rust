// Author: Nicholas Renner
//
// Pipes for SafePOSIX based on Lock-Free Circular Buffer

#![allow(dead_code)]
use crate::interface;

use std::slice;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use ringbuf::{RingBuffer, Producer, Consumer};
use std::cmp::min;

const O_RDONLY: i32 = 0o0;
const O_WRONLY: i32 = 0o1;
const O_RDWRFLAGS: i32 = 0o3;

pub fn new_pipe(size: usize) -> EmulatedPipe {
    EmulatedPipe::new_with_capacity(size)
}

pub struct EmulatedPipe {
    write_end: Arc<Mutex<Producer<u8>>>,
    read_end: Arc<Mutex<Consumer<u8>>>,
    pub refcount_write: AtomicU32,
    pub refcount_read: AtomicU32,
    size: usize,
    eof: AtomicBool,
}

impl EmulatedPipe {
    pub fn new_with_capacity(size: usize) -> EmulatedPipe {
        let rb = RingBuffer::<u8>::new(size);
        let (prod, cons) = rb.split();
        EmulatedPipe { write_end: Arc::new(Mutex::new(prod)), read_end: Arc::new(Mutex::new(cons)), refcount_write: AtomicU32::new(1), refcount_read: AtomicU32::new(1), size: size, eof: AtomicBool::new(false)}
    }

    pub fn set_eof(&self) {
        self.eof.store(true, Ordering::Relaxed);
    }

    pub fn get_write_ref(&self) -> u32 {
        self.refcount_write.load(Ordering::Relaxed)
    }

    pub fn get_read_ref(&self) -> u32 {
        self.refcount_read.load(Ordering::Relaxed)
    }

    pub fn incr_ref(&self, flags: i32) {
        if (flags & O_RDWRFLAGS) == O_RDONLY {self.refcount_read.fetch_add(1, Ordering::Relaxed);}
        if (flags & O_RDWRFLAGS) == O_WRONLY {self.refcount_write.fetch_add(1, Ordering::Relaxed);}
    }

    pub fn decr_ref(&self, flags: i32) {
        if (flags & O_RDWRFLAGS) == O_RDONLY {self.refcount_read.fetch_sub(1, Ordering::Relaxed);}
        if (flags & O_RDWRFLAGS) == O_WRONLY {self.refcount_write.fetch_sub(1, Ordering::Relaxed);}
    }

    // Write length bytes from pointer into pipe
    // BUG: This only currently works as SPSC
    pub fn write_to_pipe(&self, ptr: *const u8, length: usize) -> usize {

        let mut bytes_written = 0;

        let buf = unsafe {
            assert!(!ptr.is_null());
            slice::from_raw_parts(ptr, length)
        };

        let mut write_end = self.write_end.lock().unwrap();

        while bytes_written < length {
            let bytes_to_write = min(length, bytes_written + write_end.remaining());
            write_end.push_slice(&buf[bytes_written..bytes_to_write]);
            bytes_written = bytes_to_write;
        }   

        bytes_written
    }

    // Read length bytes from the pipe into pointer
    // Will wait for bytes unless pipe is empty and eof is set.
    // BUG: This only currently works as SPSC
    pub fn read_from_pipe(&self, ptr: *mut u8, length: usize) -> usize {

        let mut bytes_read = 0;

        let buf = unsafe {
            assert!(!ptr.is_null());
            slice::from_raw_parts_mut(ptr, length)
        };

        let mut read_end = self.read_end.lock().unwrap();

        while bytes_read < length {
            if (read_end.len() == 0) & self.eof.load(Ordering::Relaxed) { break; }
            let bytes_to_read = min(length, bytes_read + read_end.len());
            read_end.pop_slice(&mut buf[bytes_read..bytes_to_read]);
            bytes_read = bytes_to_read;
        }

        bytes_read
    }

}

