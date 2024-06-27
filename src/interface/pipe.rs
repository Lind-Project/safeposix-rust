#![allow(dead_code)]
// Author: Nicholas Renner

//! In-Memory Pipe Imlpementation for the RustPOSIX interface
//!
//! ## Pipe Module
//!
//! This module provides a method for in memory iPC between cages and is able to replicate both pipes and Unix domain sockets

/// To learn more about pipes
/// [pipe(7)](https://man7.org/linux/man-pages/man7/pipe.7.html)
use crate::interface;
use crate::interface::errnos::{syscall_error, Errno};

use parking_lot::Mutex;
use ringbuf::{Consumer, Producer, RingBuffer};
use std::cmp::min;
use std::fmt;
use std::slice;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;

// lets define a few constants for permission flags and the standard size of a Linux page
const O_RDONLY: i32 = 0o0;
const O_WRONLY: i32 = 0o1;
const O_RDWRFLAGS: i32 = 0o3;
const PAGE_SIZE: usize = 4096;

// lets also define an interval to check for thread cancellations since we may be blocking in trusted space here
// we're going to define this as 2^20 which should be ~1 sec according to the standard CLOCKS_PER_SEC definition, though it should be quite shorter on modern CPUs
const CANCEL_CHECK_INTERVAL: usize = 1048576;

/// # Description
/// Helper function to create pipe objects
///
/// # Arguments
///
/// * `size` - Size of the iPC construct (either pipe or Unix socket)
///
/// # Returns
///
/// EmulatedPipe object
///
pub fn new_pipe(size: usize) -> EmulatedPipe {
    EmulatedPipe::new_with_capacity(size)
}

/// # Description
/// In-memory pipe struct
///
/// # Fields
///
/// * `write_end` - Reference to the write end of the pipe protected by a RWLock.
/// * `read_end` - Reference to the read end of the pipe protected by a RWLock.
/// * `refcount_write` - Count of open write references.
/// * `refcount_read` - Count of open read references.
/// * `eof` - Flag signifying the pipe has finished being written to.
/// * `size` - Size of pipe buffer in bytes.
#[derive(Clone)]
pub struct EmulatedPipe {
    write_end: Arc<Mutex<Producer<u8>>>,
    read_end: Arc<Mutex<Consumer<u8>>>,
    pub refcount_write: Arc<AtomicU32>,
    pub refcount_read: Arc<AtomicU32>,
    eof: Arc<AtomicBool>,
    size: usize,
}

impl EmulatedPipe {
    /// # Description
    /// Creates an in-memory pipe object
    ///
    /// # Arguments
    ///
    /// * `size` - Size of the iPC construct (either pipe or Unix socket)
    ///
    /// # Returns
    ///
    /// EmulatedPipe object
    ///
    pub fn new_with_capacity(size: usize) -> EmulatedPipe {
        let rb = RingBuffer::<u8>::new(size);
        let (prod, cons) = rb.split();
        EmulatedPipe {
            write_end: Arc::new(Mutex::new(prod)),
            read_end: Arc::new(Mutex::new(cons)),
            refcount_write: Arc::new(AtomicU32::new(1)),
            refcount_read: Arc::new(AtomicU32::new(1)),
            eof: Arc::new(AtomicBool::new(false)),
            size: size,
        }
    }

    /// # Description
    /// Setter for EOF flag
    pub fn set_eof(&self) {
        self.eof.store(true, Ordering::Relaxed);
    }

    /// # Description
    /// Getter for write references
    ///
    /// # Returns
    ///
    /// Number of references to write end of the pipe
    ///
    pub fn get_write_ref(&self) -> u32 {
        self.refcount_write.load(Ordering::Relaxed)
    }

    /// # Description
    /// Getter for read references
    ///
    /// # Returns
    ///
    /// Number of references to read end of the pipe
    ///
    pub fn get_read_ref(&self) -> u32 {
        self.refcount_read.load(Ordering::Relaxed)
    }

    /// # Description
    /// Increase references to write or read descriptor
    ///
    /// # Arguments
    ///
    /// * `flags` - Flags set on pipe descriptor, used to determine if its the read or write end
    ///
    pub fn incr_ref(&self, flags: i32) {
        if (flags & O_RDWRFLAGS) == O_RDONLY {
            self.refcount_read.fetch_add(1, Ordering::Relaxed);
        }
        if (flags & O_RDWRFLAGS) == O_WRONLY {
            self.refcount_write.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// # Description
    /// Decrease references to write or read descriptor
    ///
    /// # Arguments
    ///
    /// * `flags` - Flags set on pipe descriptor, used to determine if its the read or write end
    ///
    pub fn decr_ref(&self, flags: i32) {
        if (flags & O_RDWRFLAGS) == O_RDONLY {
            self.refcount_read.fetch_sub(1, Ordering::Relaxed);
        }
        if (flags & O_RDWRFLAGS) == O_WRONLY {
            self.refcount_write.fetch_sub(1, Ordering::Relaxed);
        }
    }

    /// # Description
    /// Checks if pipe is currently ready for reading, used by select/poll etc
    ///
    /// # Returns
    ///
    /// True if descriptor is ready for reading, false if it will block
    ///
    pub fn check_select_read(&self) -> bool {
        let read_end = self.read_end.lock();
        let pipe_space = read_end.len();

        if (pipe_space > 0) || self.eof.load(Ordering::SeqCst) {
            return true;
        } else {
            return false;
        }
    }

    /// # Description
    /// Checks if pipe is currently ready for writing, used by select/poll etc
    ///
    /// # Returns
    ///
    /// True if descriptor is ready for writing, false if it will block
    ///
    pub fn check_select_write(&self) -> bool {
        let write_end = self.write_end.lock();
        let pipe_space = write_end.remaining();

        // Linux considers a pipe writeable if there is at least PAGE_SIZE (PIPE_BUF) remaining space (4096 bytes)
        return (self.size - pipe_space) > PAGE_SIZE;
    }

    /// ### Description
    ///
    /// write_to_pipe writes a specified number of bytes starting at the given pointer to a circular buffer.
    ///
    /// ### Arguments
    ///
    /// write_to_pipe accepts three arguments:
    /// * `ptr` - a pointer to the data being written.
    /// * `length` - the amount of bytes to attempt to write
    /// * `nonblocking` - if this attempt to write is nonblocking
    ///
    /// ### Returns
    ///
    /// Upon successful completion, the amount of bytes written is returned.
    /// In case of a failure, an error is returned to the calling syscall.
    ///
    /// ### Errors
    ///
    /// * `EAGAIN` - Non-blocking is enabled and the write has failed to fully complete.
    /// * `EPIPE` - An attempt to write to a pipe with all read references have been closed.
    ///
    /// ### Panics
    ///
    /// A panic occurs if the provided pointer is null
    ///
    /// To learn more about pipes and the write syscall
    /// [pipe(7)](https://man7.org/linux/man-pages/man7/pipe.7.html)
    /// [write(2)](https://man7.org/linux/man-pages/man2/write.2.html)
    pub fn write_to_pipe(&self, ptr: *const u8, length: usize, nonblocking: bool) -> i32 {
        // convert the raw pointer into a slice to interface with the circular buffer
        let buf = unsafe {
            assert!(!ptr.is_null());
            slice::from_raw_parts(ptr, length)
        };

        let mut write_end = self.write_end.lock();
        let mut bytes_written = 0;

        while bytes_written < length {
            if self.get_read_ref() == 0 {
                // we send EPIPE here since all read ends are closed
                return syscall_error(Errno::EPIPE, "write", "broken pipe");
            }

            let remaining = write_end.remaining();

            // we loop until either more than a page of bytes is free in the pipe
            // for why we wait for a free page of bytes, refer to the pipe man page about atomicity of writes or the pipe_write kernel implementation
            if remaining < PAGE_SIZE {
                if nonblocking {
                    // for non-blocking if we have written a bit lets return how much we've written, otherwise we return EAGAIN
                    if bytes_written > 0 {
                        return bytes_written as i32;
                    } else {
                        return syscall_error(
                            Errno::EAGAIN,
                            "write",
                            "there is no space available right now, try again later",
                        );
                    }
                } else {
                    // we yield here on a non-writable pipe to let other threads continue more quickly
                    interface::lind_yield();
                    continue;
                }
            };

            // lets read the minimum of the specified amount or whatever space we have
            let bytes_to_write = min(length, bytes_written as usize + remaining);
            write_end.push_slice(&buf[bytes_written..bytes_to_write]);
            bytes_written = bytes_to_write;
        }

        // lets return the amount we've written to the pipe
        bytes_written as i32
    }

    /// ### Description
    ///
    /// write_vectored_to_pipe translates iovecs into a singular buffer so that write_to_pipe can write that data to a circular buffer.
    ///
    /// ### Arguments
    ///
    /// write_to_pipe accepts three arguments:
    /// * `ptr` - a pointer to an Iovec array.
    /// * `iovcnt` - the number of iovec indexes in the array
    /// * `nonblocking` - if this attempt to write is nonblocking
    ///
    /// ### Returns
    ///
    /// Upon successful completion, the amount of bytes written is returned via write_to_pipe.
    /// In case of a failure, an error is returned to the calling syscall.
    ///
    /// ### Errors
    ///
    /// * `EAGAIN` - Non-blocking is enabled and the write has failed to fully complete (via write_to_pipe).
    /// * `EPIPE` - An attempt to write to a pipe with all read references have been closed (via write_to_pipe).
    ///
    /// ### Panics
    ///
    /// A panic occurs if the provided pointer is null
    ///
    /// To learn more about pipes and the writev syscall
    /// [pipe(7)](https://man7.org/linux/man-pages/man7/pipe.7.html)
    /// [pipe(7)](https://man7.org/linux/man-pages/man2/writev.2.html)
    pub fn write_vectored_to_pipe(
        &self,
        ptr: *const interface::IovecStruct,
        iovcnt: i32,
        nonblocking: bool,
    ) -> i32 {
        let mut buf = Vec::new();
        let mut length = 0;

        // we're going to loop through the iovec array and combine the buffers into one slice, recording the length
        // this is hacky but is the best way to do this for now
        for _iov in 0..iovcnt {
            unsafe {
                assert!(!ptr.is_null());
                let iovec = *ptr;
                // lets conevrt this iovec into a slice and then extend our concatenated buffer
                let iovbuf = slice::from_raw_parts(iovec.iov_base as *const u8, iovec.iov_len);
                buf.extend_from_slice(iovbuf);
                length = length + iovec.iov_len
            };
        }

        // now that we have a single buffer we can use the usual write to pipe function
        self.write_to_pipe(buf.as_ptr(), length, nonblocking)
    }

    /// ### Description
    ///
    /// read_from_pipe reads a specified number of bytes from the circular buffer to a given pointer.
    ///
    /// ### Arguments
    ///
    /// write_to_pipe accepts three arguments:
    /// * `ptr` - a pointer to the buffer being read to.
    /// * `length` - the amount of bytes to attempt to read
    /// * `nonblocking` - if this attempt to read is nonblocking
    ///
    /// ### Returns
    ///
    /// Upon successful completion, the amount of bytes read is returned.
    /// In case of a failure, an error is returned to the calling syscall.
    ///
    /// ### Errors
    ///
    /// * `EAGAIN` - Non-blocking is enabled and there is no data in the pipe.
    ///
    /// ### Panics
    ///
    /// A panic occurs if the provided pointer is null
    ///
    /// To learn more about pipes and the read syscall
    /// [read(2))](https://man7.org/linux/man-pages/man2/read.2.html)
    pub fn read_from_pipe(&self, ptr: *mut u8, length: usize, nonblocking: bool) -> i32 {
        // convert the raw pointer into a slice to interface with the circular buffer
        let buf = unsafe {
            assert!(!ptr.is_null());
            slice::from_raw_parts_mut(ptr, length)
        };

        let mut read_end = self.read_end.lock();
        let mut pipe_space = read_end.len();
        if nonblocking && (pipe_space == 0) {
            // if the descriptor is non-blocking and theres nothing in the pipe we either:
            // return 0 if the EOF is reached
            // or return EAGAIN due to the O_NONBLOCK flag
            if self.eof.load(Ordering::SeqCst) {
                return 0;
            }
            return syscall_error(
                Errno::EAGAIN,
                "read",
                "there is no data available right now, try again later",
            );
        }

        // wait for something to be in the pipe, but break on eof
        let mut count = 0;
        while pipe_space == 0 {
            // check for EOF first, if its set return 0
            if self.eof.load(Ordering::SeqCst) {
                return 0;
            }

            // we return EAGAIN here so we can go back to check if this cage has been sent a cancel notification in the calling syscall
            // if the calling descriptor is blocking we then attempt to read again
            if count == CANCEL_CHECK_INTERVAL {
                return -(Errno::EAGAIN as i32);
            }

            // lets check again if were empty
            pipe_space = read_end.len();
            count = count + 1;

            if pipe_space == 0 {
                // we yield here on an empty pipe to let other threads continue more quickly
                interface::lind_yield();
            }
        }

        // we've found something int the pipe
        // lets read the minimum of the specified amount or whatever is in the pipe
        let bytes_to_read = min(length, pipe_space);
        read_end.pop_slice(&mut buf[0..bytes_to_read]);

        // return the amount we read
        bytes_to_read as i32
    }
}

impl fmt::Debug for EmulatedPipe {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EmulatedPipe")
            .field("refcount read", &self.refcount_read)
            .field("refcount write", &self.refcount_write)
            .field("eof", &self.eof)
            .finish()
    }
}
