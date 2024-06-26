//! This module acts a wrapper for all system calls in the RustPOSIX environment. and has methods for each system call. divided into three categories: filesystem, system, and network
//!
//! ## System Calls
//!
//! Cage objects have methods for system calls, categorized as filesystem, system, or network-related. They return a code or an error from the `errno` enum.
//!

pub mod fs_calls;
pub mod fs_constants;
pub mod net_calls;
pub mod net_constants;
pub mod sys_calls;
pub mod sys_constants;
pub use fs_calls::*;
pub use fs_constants::*;
pub use net_calls::*;
pub use net_constants::*;
pub use sys_calls::*;
pub use sys_constants::*;
