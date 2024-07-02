#![allow(dead_code)]
// Author: Nicholas Renner

//! In-Memory Pipe Imlpementation for the RustPOSIX interface
//!
//! ## Pipe Module
//!
//! This module provides a method for in memory iPC between cages and is able to
//! replicate both pipes and Unix domain sockets.
//!
//! Linux pipes are implemented as a circular buffer of 16 pages (4096 bytes
//! each). Instead for RustPOSIX we implement the buffer as a lock-free circular
//! buffer for the sum of bytes in those pages.
//!
//! This implementation is also used internally by RustPOSIX to approximate Unix
//! sockets by allocating two of these pipes bi-directionally.
//!
//! We expose an API allowing to read and write to the pipe as well as check if
//! pipe descriptors are reading for reading and writing via select/poll
///
/// To learn more about pipes
/// [pipe(7)](https://man7.org/linux/man-pages/man7/pipe.7.html)
use crate::interface;
use crate::interface::errnos::{syscall_error, Errno};

use parking_lot::Mutex;
use ringbuf::{Consumer, Producer, RingBuffer};
use std::cmp::min;
use std::fmt;
use std::slice;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

// lets define a few constants for permission flags and the standard size of a
// Linux page
const O_RDONLY: i32 = 0o0;
const O_WRONLY: i32 = 0o1;
const O_RDWRFLAGS: i32 = 0o3;
const PAGE_SIZE: usize = 4096;

// lets also define an interval to check for thread cancellations since we may
// be blocking by busy waiting in trusted space here we're going to define this
// as 2^20 which should be ~1 sec according to the standard CLOCKS_PER_SEC
// definition, though it should be quite shorter on modern CPUs
const CANCEL_CHECK_INTERVAL: usize = 1048576;

/// # Description
/// In-memory pipe struct of given size which contains references to read and
/// write ends of a lock-free ringbuffer, as well as reference counters to each
/// end
#[derive(Clone)]
pub struct EmulatedPipe {
    write_end: Arc<Mutex<Producer<u8>>>,
    read_end: Arc<Mutex<Consumer<u8>>>,
    refcount_write: Arc<AtomicU32>,
    refcount_read: Arc<AtomicU32>,
    size: usize,
}

impl EmulatedPipe {
    /// # Description
    /// Creates an in-memory pipe object of specified size.
    /// The size provided is either a constant for a pipe (65,536 bytes) or a
    /// domain socket (212,992 bytes)
    ///
    /// # Arguments
    ///
    /// * `size` - Size of the iPC construct in bytes (either pipe or Unix
    ///   socket)
    ///
    /// # Returns
    ///
    /// EmulatedPipe object
    pub fn new_with_capacity(size: usize) -> EmulatedPipe {
        let rb = RingBuffer::<u8>::new(size);
        let (prod, cons) = rb.split();
        EmulatedPipe {
            write_end: Arc::new(Mutex::new(prod)),
            read_end: Arc::new(Mutex::new(cons)),
            refcount_write: Arc::new(AtomicU32::new(1)),
            refcount_read: Arc::new(AtomicU32::new(1)),
            size: size,
        }
    }

    /// # Description
    /// Checks the references to each end of the pipe to determine if its closed
    /// Necessary for determining if Unix sockets are closed for each direction
    ///
    /// # Returns
    ///
    /// True if all references are closed, false if there are open references
    pub fn is_pipe_closed(&self) -> bool {
        self.get_write_ref() + self.get_read_ref() == 0
    }

    /// Internal getter for write references
    ///
    /// Returns number of references to write end of the pipe
    fn get_write_ref(&self) -> u32 {
        self.refcount_write.load(Ordering::Relaxed)
    }

    /// Internal getter for read references
    ///
    /// Returns number of references to read end of the pipe
    fn get_read_ref(&self) -> u32 {
        self.refcount_read.load(Ordering::Relaxed)
    }

    /// # Description
    /// Increase references to write or read end.
    /// This is called when a reference to the pipe end is duplicated in cases
    /// such as fork() and dup/dup2().
    ///
    /// # Arguments
    ///
    /// * `flags` - Flags set on pipe descriptor, used to determine if its the
    ///   read or write end
    pub fn incr_ref(&self, flags: i32) {
        if (flags & O_RDWRFLAGS) == O_RDONLY {
            self.refcount_read.fetch_add(1, Ordering::Relaxed);
        }
        if (flags & O_RDWRFLAGS) == O_WRONLY {
            self.refcount_write.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// # Description
    /// Decrease references to write or read end.
    /// This is called when a reference to the pipe end is removed in cases such
    /// as close().
    ///
    /// # Arguments
    ///
    /// * `flags` - Flags set on pipe descriptor, used to determine if its the
    ///   read or write end
    pub fn decr_ref(&self, flags: i32) {
        if (flags & O_RDWRFLAGS) == O_RDONLY {
            self.refcount_read.fetch_sub(1, Ordering::Relaxed);
        }
        if (flags & O_RDWRFLAGS) == O_WRONLY {
            self.refcount_write.fetch_sub(1, Ordering::Relaxed);
        }
    }

    /// # Description
    /// Checks if pipe is currently ready for reading, used by select/poll etc.
    /// A pipe descriptor is ready if there is anything in the pipe or there are
    /// 0 remaining write references.
    ///
    /// # Returns
    ///
    /// True if descriptor is ready for reading, false if it will block
    pub fn check_select_read(&self) -> bool {
        let read_end = self.read_end.lock();
        let pipe_space = read_end.len();

        return (pipe_space > 0) || self.get_write_ref() == 0;
    }

    /// # Description
    /// Checks if pipe is currently ready for writing, used by select/poll etc.
    /// A pipe descriptor is ready for writing if there is more than a page
    /// (4096 bytes) of room in the pipe or if there are 0 remaining read
    /// references.
    ///
    /// # Returns
    ///
    /// True if descriptor is ready for writing, false if it will block
    pub fn check_select_write(&self) -> bool {
        let write_end = self.write_end.lock();
        let pipe_space = write_end.remaining();

        // Linux considers a pipe writeable if there is at least PAGE_SIZE (PIPE_BUF)
        // remaining space (4096 bytes)
        return pipe_space > PAGE_SIZE || self.get_read_ref() == 0;
    }

    /// ### Description
    ///
    /// write_to_pipe writes a specified number of bytes starting at the given
    /// pointer to a circular buffer.
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
    /// * `EAGAIN` - Non-blocking is enabled and the write has failed to fully
    ///   complete.
    /// * `EPIPE` - An attempt to write to a pipe with all read references have
    ///   been closed.
    ///
    /// ### Panics
    ///
    /// A panic occurs if the provided pointer is null
    ///
    /// To learn more about pipes and the write syscall
    /// [pipe(7)](https://man7.org/linux/man-pages/man7/pipe.7.html)
    /// [write(2)](https://man7.org/linux/man-pages/man2/write.2.html)
    pub fn write_to_pipe(&self, ptr: *const u8, length: usize, nonblocking: bool) -> i32 {
        // unlikely but if we attempt to write nothing, return 0
        if length == 0 {
            return 0;
        }

        // convert the raw pointer into a slice to interface with the circular buffer
        let buf = unsafe {
            assert!(!ptr.is_null());
            slice::from_raw_parts(ptr, length)
        };

        let mut write_end = self.write_end.lock();
        let mut bytes_written = 0;

        // Here we attempt to write the data to the pipe, looping until all bytes are
        // written or in non-blocking scenarios error with EAGAIN
        //
        // Here are the four different scenarios we encounter (via the pipe(7) manpage):
        //
        // O_NONBLOCK disabled, n <= PAGE_SIZE
        // All n bytes are written, write may block if
        // there is not room for n bytes to be written immediately

        // O_NONBLOCK enabled, n <= PAGE_SIZE
        // If there is room to write n bytes to the pipe, then
        // write succeeds immediately, writing all n bytes;
        // otherwise write fails, with errno set to EAGAIN.

        // O_NONBLOCK disabled, n > PAGE_SIZE
        // The write blocks until n bytes have been written.
        // Because Linux implements pipes as a buffer of pages, we need to wait until
        // a page worth of bytes is free in our buffer until we can write

        // O_NONBLOCK enabled, n > PAGE_SIZE
        // If the pipe is full, then write fails, with errno set to EAGAIN.
        // Otherwise, a "partial write" may occur returning the number of bytes written

        while bytes_written < length {
            if self.get_read_ref() == 0 {
                // we send EPIPE here since all read ends are closed
                return syscall_error(Errno::EPIPE, "write", "broken pipe");
            }

            let remaining = write_end.remaining();

            // we loop until either more than a page of bytes is free in the pipe
            // Linux technically writes a page per iteration here but its more efficient and
            // should be semantically equivalent to write more for why we wait
            if remaining < PAGE_SIZE {
                if nonblocking {
                    // for non-blocking if we have written a bit lets return how much we've written,
                    // otherwise we return EAGAIN
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
                    // we yield here on a non-writable pipe to let other threads continue more
                    // quickly
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
    /// write_vectored_to_pipe translates iovecs into a singular buffer so that
    /// write_to_pipe can write that data to a circular buffer.
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
    /// Upon successful completion, the amount of bytes written is returned via
    /// write_to_pipe. In case of a failure, an error is returned to the
    /// calling syscall.
    ///
    /// ### Errors
    ///
    /// * `EAGAIN` - Non-blocking is enabled and the write has failed to fully
    ///   complete (via write_to_pipe).
    /// * `EPIPE` - An attempt to write to a pipe when all read references have
    ///   been closed (via write_to_pipe).
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
        // unlikely but if we attempt to write 0 iovecs, return 0
        if iovcnt == 0 {
            return 0;
        }

        let mut buf = Vec::new();
        let mut length = 0;

        // we're going to loop through the iovec array and combine the buffers into one
        // Rust slice so that we can use the write_to_pipe function, recording the
        // length this is hacky but is the best way to do this for now
        for _iov in 0..iovcnt {
            unsafe {
                assert!(!ptr.is_null());
                // lets convert this iovec into a Rust slice,
                // and then extend our combined buffer
                let iovec = *ptr;
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
    /// read_from_pipe reads a specified number of bytes from the circular
    /// buffer to a given pointer.
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
    /// * `EAGAIN` - A non-blocking  read is attempted without EOF being reached
    ///   but there is no data in the pipe.
    ///
    /// ### Panics
    ///
    /// A panic occurs if the provided pointer is null
    ///
    /// To learn more about pipes and the read syscall
    /// [read(2))](https://man7.org/linux/man-pages/man2/read.2.html)
    pub fn read_from_pipe(&self, ptr: *mut u8, length: usize, nonblocking: bool) -> i32 {
        // unlikely but if we attempt to read nothing, return 0
        if length == 0 {
            return 0;
        }

        // convert the raw pointer into a slice to interface with the circular buffer
        let buf = unsafe {
            assert!(!ptr.is_null());
            slice::from_raw_parts_mut(ptr, length)
        };

        let mut read_end = self.read_end.lock();
        let mut pipe_space = read_end.len();
        if nonblocking && (pipe_space == 0) {
            // if the descriptor is non-blocking and theres nothing in the pipe we either:
            // return 0 if the EOF is reached (zero write references)
            // or return EAGAIN due to the O_NONBLOCK flag
            if self.get_write_ref() == 0 {
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
            // If write references are 0, we've reached EOF so return 0
            if self.get_write_ref() == 0 {
                return 0;
            }

            // we return EAGAIN here so we can go back to check if this cage has been sent a
            // cancel notification in the calling syscall if the calling
            // descriptor is blocking we then attempt to read again
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
            .finish()
    }
}
