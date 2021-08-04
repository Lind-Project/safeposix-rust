#![feature(once_cell)] //for synclazy
#![feature(rustc_private)] //for private crate imports for tests
#![feature(vec_into_raw_parts)]
#![feature(duration_constants)]
#![allow(unused_imports)]

mod interface;
mod safeposix;
mod tests;

fn main() {
    println!("Hello, world!");
}
