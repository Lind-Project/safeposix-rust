#![feature(lazy_cell)]
#![feature(rustc_private)] //for private crate imports for tests
#![feature(vec_into_raw_parts)]
#![allow(unused_imports)]

mod interface;
mod safeposix;
mod tests;
mod lib_fs_utils;
