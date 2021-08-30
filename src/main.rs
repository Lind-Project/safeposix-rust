#![feature(once_cell)] //for synclazy
#![feature(rustc_private)] //for private crate imports for tests
#![feature(vec_into_raw_parts)]
#![feature(result_into_ok_or_err)]
#![allow(unused_imports)]

mod interface;
mod safeposix;
mod tests;
mod lib_fs_utils;

use crate::tests::pipe_tests::pipe_tests::ut_lind_fs_pipe;

fn main() {
    ut_lind_fs_pipe();
    // println!("Hello, world!");
}
