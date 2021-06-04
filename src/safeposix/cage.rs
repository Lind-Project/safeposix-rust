
use interface::*;

pub static cage_table: rust_global<rust_rfc<rust_lock<rush_hashmap>>> = rust_global::new(|| rust_rfc::new(rust_lock::new(new_hashmap())));


pub struct cage {

}

impl cage {

}