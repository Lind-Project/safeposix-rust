// Author: Nicholas Renner
//
// Pipes for SafePOSIX based on Lock-Free Circular Buffer

#![allow(dead_code)]
use crate::interface;
use crate::interface::errnos::{Errno, syscall_error};

use parking_lot::Mutex;
use std::slice;
use std::sync::Arc;
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
    pub fn check_select_read(&self) -> bool {
        let mut read_end = self.read_end.lock();
        let mut pipe_space = read_end.len();
        if !self.eof.load(Ordering::SeqCst) && (pipe_space == self.size) {
            return true;
        }
        else {
            return false;
        }
    }
    pub fn check_select_write(&self) -> bool {

        let mut write_end = self.write_end.lock();

        let pipe_space = write_end.remaining();
        if pipe_space == self.size {
            return false;
        }
        else {
            return true;
        }
    }

    // Write length bytes from pointer into pipe
    // BUG: This only currently works as SPSC
    pub fn write_to_pipe(&self, ptr: *const u8, length: usize, nonblocking: bool) -> i32 {

        let mut bytes_written = 0;

        let buf = unsafe {
            assert!(!ptr.is_null());
            slice::from_raw_parts(ptr, length)
        };

        let mut write_end = self.write_end.lock();

        let pipe_space = write_end.remaining();
        if nonblocking && (pipe_space == self.size) {
            return -1;
        }

        while bytes_written < length {
            let bytes_to_write = min(length, bytes_written as usize + write_end.remaining());
            write_end.push_slice(&buf[bytes_written..bytes_to_write]);
            bytes_written = bytes_to_write;
        }   

        bytes_written as i32
    }

    // Read length bytes from the pipe into pointer
    // Will wait for bytes unless pipe is empty and eof is set.
    // BUG: This only currently works as SPSC
    pub fn read_from_pipe(&self, ptr: *mut u8, length: usize, nonblocking: bool) -> i32 {

        let mut bytes_read = 0;

        let buf = unsafe {
            assert!(!ptr.is_null());
            slice::from_raw_parts_mut(ptr, length)
        };

        let mut read_end = self.read_end.lock();
        let mut pipe_space = read_end.len();
        if nonblocking && (pipe_space == 0) {
            return -1;
        }

        while bytes_read < length {
            pipe_space = read_end.len();
            if (pipe_space == 0) && self.eof.load(Ordering::SeqCst) { break; }
            let bytes_to_read = min(length, bytes_read + pipe_space);
            read_end.pop_slice(&mut buf[bytes_read..bytes_to_read]);
            bytes_read = bytes_to_read;
        }

        bytes_read as i32
    }

}

