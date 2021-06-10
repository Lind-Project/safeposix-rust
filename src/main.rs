#![feature(once_cell)] //for synclazy
#![feature(rustc_private)] //for private crate imports for tests
#![feature(thread_id_value)]
#![allow(unused_imports)]

mod interface;
mod safeposix;

fn main() {
    println!("Hello, world!");
}
