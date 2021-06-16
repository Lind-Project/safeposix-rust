// System related constants

use crate::interface;

// Define constants using static or const
// Imported into fs_calls file


//GID AND UID DEFAULT VALUES

static DEFAULT_UID : u64 = 1000;
static DEFAULT_GID : u64 = 1000;


// RESOURCE LIMITS

NOFILE_CUR : u64 = 1024;
NOFILE_MAX : u64 = 4*1024;

STACK_CUR : u64 = 8192*1024;
STACK_MAX : u64 = 2**32;

RLIMIT_STACK = 0;
RLIMIT_NOFILE = 1;


//R Limit for getrlimit system call
#[repr(C)]
pub struct Rlimit {
  rlim_cur: u64,
  rlim_max: u64,
}
