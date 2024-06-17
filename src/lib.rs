#![feature(lazy_cell)]
#![feature(rustc_private)] //for private crate imports for tests
#![feature(vec_into_raw_parts)]
#![feature(thread_local)]
#![allow(unused_imports)]

//! # RustPOSIX
//! Welcome to the RustPOSIX microvisor. 
//! This serves as the OS-in-a-process in the [Lind architecture](https://github.com/Lind-Project/lind-docs/tree/main). 
//! ## Overview
//!
//! A microvisor is a piece of software that lives within the same process as the cages. It provides a POSIX interface, allowing cages to believe they have their own operating system.
//!
//! With this crate, you can:
//! - Fork and execute other cages.
//! - Read and write files on disk.
//! - Communicate over the network.
//! - Manage shared memory.
//! 
//! Note this is a re-design of the original (now obsolete) [SafePOSIX](https://github.com/Lind-Project/nacl_repy), which was created using the [RepyV2](https://github.com/SeattleTestbed/docs/blob/master/Programming/RepyV2Tutorial.md) sandbox.
//! 
//! ## Building RustPOSIX
//!
//! RustPOSIX is constructed using the RustPOSIX interface. Importing external libraries via `use` statements is strictly prohibited in the files implementing RustPOSIX to maintain the restricted access to popular paths. Any attempts to import external libraries outside the specified paths will be rejected during code review.


// interface and safeposix are public because otherwise there isn't a great
// way to 'use' them for benchmarking.
pub mod interface;
mod lib_fs_utils;
pub mod safeposix;
pub mod tests;
