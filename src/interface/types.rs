use crate::safeposix::{dispatcher::*, syscalls::errnos::*, cage::*};
use crate::interface;

pub fn get_int(union_argument: Arg) -> Result<i32, i32> {
    let data = unsafe{union_argument.dispatch_int};
    let mut type_checker = Arg{dispatch_long: 0};
    //turn part of the union into 0xffffffff, but, Rust 
    //does not like just using the hex value so we are forced to use
    //a value of -1
    type_checker.dispatch_int = -1;
    if (data as i64 & !unsafe{type_checker.dispatch_long}) == 0 {
        return Ok(data);
    }
    return Err(syscall_error(Errno::EINVAL, "dispatcher", "input data not valid"));
}

pub fn get_uint(union_argument: Arg) -> Result<u32, i32> {
    let data = unsafe{union_argument.dispatch_uint};
    let mut type_checker = Arg{dispatch_ulong: 0};
    type_checker.dispatch_uint = 0xffffffff;
    if (data as u64 & !unsafe{type_checker.dispatch_ulong}) == 0 {
        return Ok(data);
    }
    return Err(syscall_error(Errno::EINVAL, "dispatcher", "input data not valid"));
}

pub fn get_long(union_argument: Arg) -> Result<i64, i32> {
    return Ok(unsafe{union_argument.dispatch_long})   //this should not return error 
}

pub fn get_ulong(union_argument: Arg) -> Result<u64, i32> {
    return Ok(unsafe{union_argument.dispatch_ulong})   //this should not return error 
}

pub fn get_isize(union_argument: Arg) -> Result<isize, i32> { // also should not return error
    return Ok(unsafe{union_argument.dispatch_isize})
}

pub fn get_usize(union_argument: Arg) -> Result<usize, i32> { //should not return an error
    return Ok(unsafe{union_argument.dispatch_usize})
}

pub fn get_cbuf(union_argument: Arg) -> Result<*const u8, i32> {
    let data = unsafe{union_argument.dispatch_cbuf};
    if !data.is_null() {
        return Ok(data);
    }
    return Err(syscall_error(Errno::EFAULT, "dispatcher", "input data not valid"));
}

pub fn get_mutcbuf(union_argument: Arg) -> Result<*mut u8, i32> {
    let data = unsafe{union_argument.dispatch_mutcbuf};
    if !data.is_null() {
        return Ok(data);
    }
    return Err(syscall_error(Errno::EFAULT, "dispatcher", "input data not valid"));
}

pub fn get_cstr<'a>(union_argument: Arg) -> Result<&'a str, i32> {
   
    //first we check that the pointer is not null 
    //and then we check so that we can get data from the memory
    
    let pointer = unsafe{union_argument.dispatch_cstr};
    if !pointer.is_null() {
        if let Ok(ret_data) = unsafe{interface::charstar_to_ruststr(pointer)} {
            return Ok(ret_data);
        } else {
            return Err(syscall_error(Errno::EILSEQ, "dispatcher", "could not parse input data to a string"));
        }
    }
    return Err(syscall_error(Errno::EFAULT, "dispatcher", "input data not valid"));
}

pub fn get_cstrarr<'a>(union_argument: Arg) -> Result<Vec<&'a str>, i32> {

    //iterate though the pointers in a function and:
    //  1: check that the pointer is not null
    //  2: push the data from that pointer onto the vector being returned
    //once we encounter a null pointer, we know that we have either hit the end of the array or another null pointer in the memory
    
    let mut pointer = unsafe{union_argument.dispatch_cstrarr};
    let mut data_vector: Vec<&str> = Vec::new();
   
    if !pointer.is_null(){
        while unsafe{!(*pointer).is_null()} {
            if let Ok(character_bytes) = unsafe{interface::charstar_to_ruststr(*pointer)} {
                data_vector.push(character_bytes);
                pointer = pointer.wrapping_offset(1);
            } else {
                return Err(syscall_error(Errno::EILSEQ, "dispatcher", "could not parse input data to string"));
            }
        }
        return Ok(data_vector);
    }
    return Err(syscall_error(Errno::EFAULT, "dispatcher", "input data not valid"));
}

pub fn get_statdatastruct<'a>(union_argument: Arg) -> Result<&'a mut StatData, i32> { 
    let pointer = unsafe{union_argument.dispatch_statdatastruct};
    if !pointer.is_null() {    
        return Ok(unsafe{&mut *pointer});
    }
    return Err(syscall_error(Errno::EFAULT, "dispatcher", "input data not valid"));
}

pub fn get_fsdatastruct<'a>(union_argument: Arg) -> Result<&'a mut FSData, i32> {
    let pointer = unsafe{union_argument.dispatch_fsdatastruct};
    if !pointer.is_null() {    
        return Ok(unsafe{&mut *pointer});
    }
    return Err(syscall_error(Errno::EFAULT, "dispatcher", "input data not valid"));
}

pub fn get_pipearray<'a>(union_argument: Arg) -> Result<&'a mut PipeArray, i32> {
    let pointer = unsafe{union_argument.dispatch_pipearray};
    if !pointer.is_null() {    
        return Ok(unsafe{&mut *pointer});
    }
    return Err(syscall_error(Errno::EFAULT, "dispatcher", "input data not valid"));
}