use crate::safeposix::{dispatcher::*, syscalls::errnos::*, cage::*};
use crate::interface;

pub fn get_int(union_argument: Arg) -> Result<i32, i32> {
    let data = unsafe{union_argument.dispatch_int};
    let mut typeChecker = Arg{dispatch_long: 0};
    typeChecker.dispatch_int = -1;
    if (data as i64 & !unsafe{typeChecker.dispatch_long}) == 0 {
        return Ok(data);
    }
    return Err(syscall_error(Errno::EINVAL, "dispatcher", "input data not valid"));
}

pub fn get_uint(union_argument: Arg) -> Result<u32, i32> {
    let data = unsafe{union_argument.dispatch_uint};
    let mut typeChecker = Arg{dispatch_ulong: 0};
    typeChecker.dispatch_uint = 0xffffffff;
    if (data as u64 & !unsafe{typeChecker.dispatch_ulong}) == 0 {
        return Ok(data);
    }
    return Err(syscall_error(Errno::EINVAL, "dispatcher", "input data not valid"));
}

pub fn get_long(union_argument: Arg) -> Result<i64, i32> {
    if let data = unsafe{union_argument.dispatch_long} { //this should not return error 
        return Ok(data);
    }
    return Err(syscall_error(Errno::EINVAL, "dispatcher", "input data not valid"));
}

pub fn get_ulong(union_argument: Arg) -> Result<u64, i32> {
    if let data = unsafe{union_argument.dispatch_ulong} {   //this should not return error 
        return Ok(data);
    }
    return Err(syscall_error(Errno::EINVAL, "dispatcher", "input data not valid"));
}

pub fn get_isize(union_argument: Arg) -> Result<isize, i32> { // also should not return error
    if let data = unsafe{union_argument.dispatch_isize} {
        return Ok(data);
    }
    return Err(syscall_error(Errno::EINVAL, "dispatcher", "input data not valid"));
}

pub fn get_usize(union_argument: Arg) -> Result<usize, i32> { //should not return an error
    if let data = unsafe{union_argument.dispatch_usize} {
        return Ok(data);
    }
    return Err(syscall_error(Errno::EINVAL, "dispatcher", "input data not valid"));
}

pub fn get_cbuf(union_argument: Arg) -> Result<*const u8, i32> {
    let data = unsafe{union_argument.dispatch_cbuf};
    if !data.is_null() {
        return Ok(data);
    }
    return Err(syscall_error(Errno::EILSEQ, "dispatcher", "input data not valid"));
}

pub fn get_mutcbuf(union_argument: Arg) -> Result<*mut u8, i32> {
    let data = unsafe{union_argument.dispatch_mutcbuf};
    if !data.is_null() {
        return Ok(data);
    }
    return Err(syscall_error(Errno::EILSEQ, "dispatcher", "input data not valid"));
}

pub fn get_cstrarr(union_argument: Arg) -> Result<*const *const i8, i32> {
    let data = unsafe{union_argument.dispatch_cstrarr};
    if !data.is_null() {
        return Ok(data);
    }
    return Err(syscall_error(Errno::EILSEQ, "dispatcher", "input data not valid"));
}

pub fn get_cstr<'a>(union_argument: Arg) -> Result<&'a str, i32> {
    let data = unsafe{interface::charstar_to_ruststr(union_argument.dispatch_cstr)};
    if let ret_data = Some(data) {
        return Ok(ret_data.unwrap());
    }
    return Err(syscall_error(Errno::EILSEQ, "dispatcher", "input data not valid"));
}

pub fn get_statdatastruct<'a>(union_argument: Arg) -> Result<&'a mut StatData, i32> { 
    let data = unsafe{&mut *union_argument.dispatch_statdatastruct};
    if let ret_data = Some(data) {
        return Ok(ret_data.unwrap());
    }
    return Err(syscall_error(Errno::EILSEQ, "dispatcher", "input data not valid"));
}

pub fn get_fsdatastruct<'a>(union_argument: Arg) -> Result<&'a mut FSData, i32> {
    let data = unsafe{&mut *union_argument.dispatch_fsdatastruct};
    if let ret_data = Some(data) {
        return Ok(ret_data.unwrap());
    }
    return Err(syscall_error(Errno::EILSEQ, "dispatcher", "input data not valid"));
}