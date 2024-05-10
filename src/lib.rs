#![feature(lazy_cell)]
#![feature(rustc_private)] //for private crate imports for tests
#![feature(vec_into_raw_parts)]
#![feature(thread_local)]
#![allow(unused_imports)]
#![allow(clippy::needless_return, clippy::explicit_auto_deref, clippy::redundant_field_names)]

// interface and safeposix are public because otherwise there isn't a great
// way to 'use' them for benchmarking.
pub mod interface;
pub mod safeposix;
pub mod tests;
mod lib_fs_utils;
