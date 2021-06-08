
use crate::interface;

pub static cage_table: RustLazyGlobal<RustLock<RustRfc<RustHashMap>>> = RustLazyGlobal::new(|| rust_rfc::new(rust_lock::new(new_hashmap())));


pub struct cage {

}

impl cage {

}
