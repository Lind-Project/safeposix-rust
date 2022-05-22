#![feature(once_cell)] //for synclazy
#![feature(rustc_private)] //for private crate imports for tests
#![feature(vec_into_raw_parts)]
#![feature(result_into_ok_or_err)]
#![feature(duration_constants)]
#![allow(unused_imports)]
#![feature(cstr_from_bytes_until_nul)]

mod interface;
mod safeposix;
mod tests;
mod lib_fs_utils;

fn main() {
    println!("Hello, world!");
}
