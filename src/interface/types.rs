use crate::interface;
use crate::interface::errnos::{Errno, syscall_error};

//redefining the FSData struct in this file so that we maintain flow of program
//derive eq attributes for testing whether the structs equal other fsdata structs from stat/fstat
#[derive(Eq, PartialEq, Default)]
#[repr(C)]
pub struct FSData {
  pub f_type: u64,
  pub f_bsize: u64,
  pub f_blocks: u64,
  pub f_bfree: u64,
  pub f_bavail: u64,
  //total files in the file system -- should be infinite
  pub f_files: u64,
  //free files in the file system -- should be infinite
  pub f_ffiles: u64,
  pub f_fsid: u64,
  //not really a limit for naming, but 254 works
  pub f_namelen: u64,
  //arbitrary val for blocksize as well
  pub f_frsize: u64,
  pub f_spare: [u8; 32]
}

//redefining the StatData struct in this file so that we maintain flow of program
//derive eq attributes for testing whether the structs equal other statdata structs from stat/fstat
#[derive(Eq, PartialEq, Default)]
#[repr(C)]
pub struct StatData {
  pub st_dev: u64,
  pub st_ino: usize,
  pub st_mode: u32,
  pub st_nlink: u32,
  pub st_uid: u32,
  pub st_gid: u32,
  pub __pad0: i32,
  pub st_rdev: u64,
  pub st_size: usize,
  pub st_blksize: isize,
  pub st_blocks: usize,
  //currently we don't populate or care about the time bits here
  pub st_atim: (u64, u64),
  pub st_mtim: (u64, u64),
  pub st_ctim: (u64, u64)
}

//R Limit for getrlimit system call
#[repr(C)]
pub struct Rlimit {
  pub rlim_cur: u64,
  pub rlim_max: u64,
}

#[derive(Eq, PartialEq, Default, Copy, Clone)]
#[repr(C)]
pub struct PipeArray {
  pub readfd: i32,
  pub writefd: i32,
}


//redefining the Arg union to maintain the flow of the program
#[repr(C)]
pub union Arg {
  pub dispatch_int: i32,
  pub dispatch_uint: u32,
  pub dispatch_ulong: u64,
  pub dispatch_long: i64,
  pub dispatch_usize: usize, //For types not specified to be a given length, but often set to word size (i.e. size_t)
  pub dispatch_isize: isize, //For types not specified to be a given length, but often set to word size (i.e. off_t)
  pub dispatch_cbuf: *const u8, //Typically corresponds to an immutable void* pointer as in write
  pub dispatch_mutcbuf: *mut u8, //Typically corresponds to a mutable void* pointer as in read
  pub dispatch_cstr: *const i8, //Typically corresponds to a passed in string of type char*, as in open
  pub dispatch_cstrarr: *const *const i8, //Typically corresponds to a passed in string array of type char* const[] as in execve
  pub dispatch_rlimitstruct: *mut Rlimit,
  pub dispatch_statdatastruct: *mut StatData,
  pub dispatch_fsdatastruct: *mut FSData,
  pub dispatch_pipearray: *mut PipeArray
}


use std::mem::size_of;

// Represents a Dirent struct without the string, as rust has no flexible array member support
#[repr(C, packed(1))]
pub struct ClippedDirent {
    pub d_ino: u64,
    pub d_off: u64,
    pub d_reclen: u16
}

pub const CLIPPED_DIRENT_SIZE: u32 = size_of::<interface::ClippedDirent>() as u32;

pub fn get_int(union_argument: Arg) -> Result<i32, i32> {
    let data = unsafe{union_argument.dispatch_int};
    let mut type_checker = Arg{dispatch_long: 0};
    //turn part of the union into 0xffffffff, but, Rust 
    //does not like just using the hex value so we are forced to use
    //a value of -1
    type_checker.dispatch_int = -1;
    if (unsafe{union_argument.dispatch_long} & !unsafe{type_checker.dispatch_long}) == 0 {
        return Ok(data);
    }
    return Err(syscall_error(Errno::EINVAL, "dispatcher", "input data not valid"));
}

pub fn get_uint(union_argument: Arg) -> Result<u32, i32> {
    let data = unsafe{union_argument.dispatch_uint};
    let mut type_checker = Arg{dispatch_ulong: 0};
    type_checker.dispatch_uint = 0xffffffff;
    if (unsafe{union_argument.dispatch_ulong} & !unsafe{type_checker.dispatch_ulong}) == 0 {
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

/// Given the vector of tuples produced from getdents_syscall, each of which consists of 
/// a ClippedDirent struct and a u8 vector representing the name, and also given the 
/// pointer to the base of the buffer to which the getdents structs should be copied, 
/// populate said buffer with these getdents structs and the names at the requisite locations
///
/// We assume a number of things about the tuples that are input: 
///
/// 1. The name in the u8 vec is null terminated
/// 2. After being null terminated it is then padded to the next highest 8 byte boundary
/// 3. After being padded, the last byte of padding is populated with DT_UNKNOWN (0) for now, 
/// as the d_type field does not have to be fully implemented for getdents to be POSIX compliant
/// 4. All fields in the clipped dirent,  are correctly filled--i.e. d_off has the correct offset
/// of the next struct in the buffer and d_reclen has the length of the struct with the padded name
/// 5. The number of tuples in the vector is such that they all fit in the buffer
///
/// There is enough information to produce a tuple vector that can satisfy these assumptions well
/// in getdents syscall, and thus all the work to satisfy these assumptions should be done there
pub fn pack_dirents(dirtuplevec: Vec<(ClippedDirent, Vec<u8>)>, baseptr: *mut u8) {
    let mut curptr = baseptr;
  
    //for each tuple we write in the ClippedDirent struct, and then the padded name vec
    for dirtuple in dirtuplevec {
      //get pointer to start of next dirent in the buffer as a ClippedDirent pointer
      let curclippedptr = curptr as *mut ClippedDirent;
      //turn that pointer into a rust reference
      let curwrappedptr = unsafe{&mut *curclippedptr};
      //assign to the data that reference points to with the value of the ClippedDirent from the tuple
      *curwrappedptr = dirtuple.0;
  
      //advance pointer by the size of one ClippedDirent, std::mem::size_of should be added into the interface
      curptr = curptr.wrapping_offset(std::mem::size_of::<ClippedDirent>() as isize);
  
      //write, starting from this advanced location, the u8 vec representation of the name
      unsafe{curptr.copy_from(dirtuple.1.as_slice().as_ptr(), dirtuple.1.len())};
  
      //advance pointer by the size of name, which we assume to be null terminated and padded correctly
      //and thus we are finished with this struct
      curptr = curptr.wrapping_offset(dirtuple.1.len() as isize);
    }
}

pub fn get_pipearray<'a>(union_argument: Arg) -> Result<&'a mut PipeArray, i32> {
    let pointer = unsafe{union_argument.dispatch_pipearray};
    if !pointer.is_null() {    
        return Ok(unsafe{&mut *pointer});
    }
    return Err(syscall_error(Errno::EFAULT, "dispatcher", "input data not valid"));
}

#[derive(Debug)]
#[repr(C)]
pub struct PollStruct {
    pub events: u32,
    pub revents: u32,
    pub fd: i32 
}

//EPOLL
#[derive(Copy, Clone, Debug)]
#[repr(C)]
pub struct EpollEvent {
    pub events: u32,
    pub fd: i32 
    //in native this is a union which could be one of a number of things
    //however, we only support EPOLL_CTL subcommands which take the fd
}
