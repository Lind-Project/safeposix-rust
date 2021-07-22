use crate::safeposix::dispatcher::*;
use crate::safeposix::cage::*;
use crate::interface;

pub fn get_int(union_argument: Arg) -> i32 {
    unsafe{union_argument.dispatch_int}
}

pub fn get_uint(union_argument: Arg) -> u32 {
    unsafe{union_argument.dispatch_uint}
}

pub fn get_long(union_argument: Arg) -> i64 {
    unsafe{union_argument.dispatch_long}
}

pub fn get_ulong(union_argument: Arg) -> u64 {
    unsafe{union_argument.dispatch_ulong}
}

pub fn get_isize(union_argument: Arg) -> isize {
    unsafe{union_argument.dispatch_isize}
}

pub fn get_usize(union_argument: Arg) -> usize {
    unsafe{union_argument.dispatch_usize}
}

pub fn get_cbuf(union_argument: Arg) -> *const u8 {
    unsafe{union_argument.dispatch_cbuf}
}

pub fn get_mutcbuf(union_argument: Arg) -> *mut u8 {
    unsafe{union_argument.dispatch_mutcbuf}
}

pub fn get_cstr(union_argument: Arg) -> &'static str {
    unsafe{interface::charstar_to_ruststr(union_argument.dispatch_cstr)}
}

pub fn get_cstrarr(union_argument: Arg) -> *const *const i8 {
    unsafe{union_argument.dispatch_cstrarr}
}

pub fn get_statdatastruct(union_argument: Arg) -> *mut StatData {
    unsafe{&mut *union_argument.dispatch_statdatastruct}
}

pub fn get_fsdatastruct(union_argument: Arg) -> *mut FSData {
    unsafe{&mut *union_argument.dispatch_fsdatastruct}
}