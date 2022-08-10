#![feature(once_cell)] //for synclazy
#![feature(rustc_private)] //for private crate imports for tests
#![feature(vec_into_raw_parts)]
#![feature(result_into_ok_or_err)]
#![feature(duration_constants)]
#![allow(unused_imports)]

mod interface;
mod safeposix;
mod tests;
mod lib_fs_utils;

use crate::tests::pipe_tests::pipe_tests::test_pipe;

fn main() {
    println!("Hello, world!");
    test_pipe();
}
