// System related constants
#![allow(dead_code)]
#![allow(unused_variables)]

use crate::interface;

// Define constants using static or const
// Imported into fs_calls file


//GID AND UID DEFAULT VALUES

pub const DEFAULT_UID : u64 = 1000;
pub const DEFAULT_GID : u64 = 1000;


// RESOURCE LIMITS

pub const NOFILE_CUR : u64 = 1024;
pub const NOFILE_MAX : u64 = 4*1024;

pub const STACK_CUR : u64 = 8192*1024;
pub const STACK_MAX : u64 = 1 << 32;

pub const RLIMIT_STACK: u64 = 0;
pub const RLIMIT_NOFILE: u64 = 1;
