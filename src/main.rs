#![feature(once_cell)] //for synclazy
#![feature(rustc_private)] //for private crate imports for tests
#![feature(vec_into_raw_parts)]
#![allow(unused_imports)]

mod interface;
mod safeposix;

fn main() {
    println!("Hello, world!");
}
