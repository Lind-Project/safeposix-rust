#![allow(dead_code)]
#![allow(unused_variables)]
// retreive cage table
const ACCESS_SYSCALL: i32 = 2;
const UNLINK_SYSCALL: i32 = 4;
const LINK_SYSCALL: i32 = 5;
const CHDIR_SYSCALL: i32 = 6;
const MKDIR_SYSCALL: i32 = 7;
const RMDIR_SYSCALL: i32 = 8;
const XSTAT_SYSCALL: i32 = 9;
const OPEN_SYSCALL: i32 = 10;
const CLOSE_SYSCALL: i32 = 11;
const READ_SYSCALL: i32 = 12;
const WRITE_SYSCALL: i32 = 13;
const LSEEK_SYSCALL: i32 = 14;
const IOCTL_SYSCALL: i32 = 15;
const FXSTAT_SYSCALL: i32 = 17;
const FSTATFS_SYSCALL: i32 = 19;
const MMAP_SYSCALL: i32 = 21;
const MUNMAP_SYSCALL: i32 = 22;
const GETDENTS_SYSCALL: i32 = 23;
const DUP_SYSCALL: i32 = 24;
const DUP2_SYSCALL: i32 = 25;
const STATFS_SYSCALL: i32 = 26;
const FCNTL_SYSCALL: i32 = 28;
const GETPPID_SYSCALL: i32 = 29;
const EXIT_SYSCALL: i32 = 30;
const GETPID_SYSCALL: i32 = 31;
const SOCKET_SYSCALL: i32 = 32;
const BIND_SYSCALL: i32 = 33;
const SEND_SYSCALL: i32 = 34;
const SENDTO_SYSCALL: i32 = 35;
const RECV_SYSCALL: i32 = 36;
const RECVFROM_SYSCALL: i32 = 37;
const CONNECT_SYSCALL: i32 = 38;
const LISTEN_SYSCALL: i32 = 39;
const ACCEPT_SYSCALL: i32 = 40;
const GETPEERNAME_SYSCALL: i32 = 41;
const GETSOCKNAME_SYSCALL: i32 = 42;
const GETSOCKOPT_SYSCALL: i32 = 43;
const SETSOCKOPT_SYSCALL: i32 = 44;
const SHUTDOWN_SYSCALL: i32 = 45;
const SELECT_SYSCALL: i32 = 46;
const GETIFADDRS_SYSCALL: i32 = 47;
const POLL_SYSCALL: i32 = 48;
const SOCKETPAIR_SYSCALL: i32 = 49;
const GETUID_SYSCALL: i32 = 50;
const GETEUID_SYSCALL: i32 = 51;
const GETGID_SYSCALL: i32 = 52;
const GETEGID_SYSCALL: i32 = 53;
const FLOCK_SYSCALL: i32 = 54;
const RENAME_SYSCALL: i32 = 55;
const EPOLL_CREATE_SYSCALL: i32 = 56;
const EPOLL_CTL_SYSCALL: i32 = 57;
const EPOLL_WAIT_SYSCALL: i32 = 58;

const PIPE_SYSCALL: i32 = 66;
const PIPE2_SYSCALL: i32 = 67;
const FORK_SYSCALL: i32 = 68;
const EXEC_SYSCALL: i32 = 69;

const GETHOSTNAME_SYSCALL: i32 = 125;
const PREAD_SYSCALL: i32 = 126;
const PWRITE_SYSCALL: i32 = 127;


use crate::interface;
use super::cage::{CAGE_TABLE, Cage};
use super::syscalls::{sys_constants::*, fs_constants::*};
use super::filesystem::{FS_METADATA, load_fs, incref_root};


#[repr(C)]
pub union Arg {
  dispatch_int: i32,
  dispatch_uint: u32,
  dispatch_ulong: u64,
  dispatch_long: i64,
  dispatch_usize: usize, //For types not specified to be a given length, but often set to word size (i.e. size_t)
  dispatch_isize: isize, //For types not specified to be a given length, but often set to word size (i.e. off_t)
  dispatch_cbuf: *const u8, //Typically corresponds to an immutable void* pointer as in write
  dispatch_mutcbuf: *mut u8, //Typically corresponds to a mutable void* pointer as in read
  dispatch_cstr: *const i8, //Typically corresponds to a passed in string of type char*, as in open
  dispatch_cstrarr: *const *const i8, //Typically corresponds to a passed in string array of type char* const[] as in execve
  dispatch_rlimitstruct: *mut Rlimit,
  dispatch_statdatastruct: *mut StatData
}

pub extern "C" fn dispatcher(cageid: u64, callnum: i32, arg1: Arg, arg2: Arg, arg3: Arg, arg4: Arg, arg5: Arg, arg6: Arg) -> i32 {

    // need to match based on if cage exists
    let cage = { CAGE_TABLE.read().unwrap().get(&cageid).unwrap().clone() };

    //implement syscall method calling using matching

    match callnum {
        ACCESS_SYSCALL => {
            cage.access_syscall(unsafe{interface::charstar_to_ruststr(arg1.dispatch_cstr)}, unsafe{arg2.dispatch_uint})
        }
        UNLINK_SYSCALL => {
            cage.unlink_syscall(unsafe{interface::charstar_to_ruststr(arg1.dispatch_cstr)})
        }
        LINK_SYSCALL => {
            cage.link_syscall(unsafe{interface::charstar_to_ruststr(arg1.dispatch_cstr)}, unsafe{interface::charstar_to_ruststr(arg1.dispatch_cstr)})
        }
        CHDIR_SYSCALL => {
            cage.chdir_syscall(unsafe{interface::charstar_to_ruststr(arg1.dispatch_cstr)})
        }
        XSTAT_SYSCALL => {
            cage.stat_syscall(unsafe{interface::charstar_to_ruststr(arg1.dispatch_cstr)}, unsafe{&mut *arg2.dispatch_statdatastruct})
        }
        OPEN_SYSCALL => {
            cage.open_syscall(unsafe{interface::charstar_to_ruststr(arg1.dispatch_cstr)}, unsafe{arg2.dispatch_int}, unsafe{arg3.dispatch_uint})
        }
        READ_SYSCALL => {
            cage.read_syscall(unsafe{arg1.dispatch_int}, unsafe{arg2.dispatch_mutcbuf}, unsafe{arg3.dispatch_usize})
        }
        WRITE_SYSCALL => {
            cage.write_syscall(unsafe{arg1.dispatch_int}, unsafe{arg2.dispatch_cbuf}, unsafe{arg3.dispatch_usize})
        }
        CLOSE_SYSCALL => {
            cage.close_syscall(unsafe{arg1.dispatch_int})
        }
        LSEEK_SYSCALL => {
            cage.lseek_syscall(unsafe{arg1.dispatch_int}, unsafe{arg2.dispatch_isize}, unsafe{arg3.dispatch_int})
        }
        FXSTAT_SYSCALL => {
            cage.fstat_syscall(unsafe{arg1.dispatch_int}, unsafe{&mut *arg2.dispatch_statdatastruct})
        }
        MMAP_SYSCALL => {
            cage.mmap_syscall(unsafe{arg1.dispatch_mutcbuf}, unsafe{arg2.dispatch_usize}, unsafe{arg3.dispatch_int}, 
                              unsafe{arg4.dispatch_int}, unsafe{arg5.dispatch_int}, unsafe{arg6.dispatch_long})
        }
        MUNMAP_SYSCALL => {
            cage.munmap_syscall(unsafe{arg1.dispatch_mutcbuf}, unsafe{arg2.dispatch_usize})
        }
        DUP_SYSCALL => {
            cage.dup_syscall(unsafe{arg1.dispatch_int}, None)
        }
        DUP2_SYSCALL => {
            cage.dup2_syscall(unsafe{arg1.dispatch_int}, unsafe{arg2.dispatch_int})
        }
        FCNTL_SYSCALL => {
            cage.fcntl_syscall(unsafe{arg1.dispatch_int}, unsafe{arg2.dispatch_int}, unsafe{arg3.dispatch_int})
        }
        GETPPID_SYSCALL => {
            cage.getppid_syscall()
        }
        GETPID_SYSCALL => {
            cage.getpid_syscall()
        }
        EXIT_SYSCALL => {
            cage.exit_syscall()
        }
        FLOCK_SYSCALL => {
            cage.flock_syscall(unsafe{arg1.dispatch_int}, unsafe{arg2.dispatch_int})
        }
        FORK_SYSCALL => {
            cage.fork_syscall(unsafe{arg1.dispatch_ulong})
        }
        EXEC_SYSCALL => {
            cage.exec_syscall(unsafe{arg1.dispatch_ulong})
        }
        GETUID_SYSCALL => {
            cage.getuid_syscall()
        }
        GETEUID_SYSCALL => {
            cage.geteuid_syscall()
        }
        GETGID_SYSCALL => {
            cage.getgid_syscall()
        }
        GETEGID_SYSCALL => {
            cage.getegid_syscall()
        }
        PREAD_SYSCALL => {
            cage.pread_syscall(unsafe{arg1.dispatch_int}, unsafe{arg2.dispatch_mutcbuf}, unsafe{arg3.dispatch_usize}, unsafe{arg4.dispatch_isize})
        }
        PWRITE_SYSCALL => {
            cage.pwrite_syscall(unsafe{arg1.dispatch_int}, unsafe{arg2.dispatch_cbuf}, unsafe{arg3.dispatch_usize}, unsafe{arg4.dispatch_isize})
        }
        CHMOD_SYSCALL => {
            cage.chmod_syscall(unsafe{interface::charstar_to_ruststr(arg1.dispatch_cstr)}, unsafe{arg2.dispatch_uint})
        }
        FCNTL_SYSCALL => {
            cage.fcntl_syscall(unsafe{arg1.dispatch_int}, unsafe{arg2.dispatch_int}, unsafe{arg3.dispatch_int})
        }
        DUP_SYSCALL => {
            cage.dup_syscall(unsafe{arg1.dispatch_int}, None)
        }
        DUP2_SYSCALL => {
            cage.dup2_syscall(unsafe{arg1.dispatch_int}, unsafe{arg2.dispatch_int})
        }
        GETDENTS_SYSCALL => {
            let mut dents_vec: Vec<(ClippedDirent, Vec<u8>)> = Vec::new();
            let res = cage.getdents_syscall(unsafe{arg1.dispatch_int}, unsafe{arg2.dispatch_usize}, &mut dents_vec);
            pack_dirents(dents_vec, unsafe{arg3.dispatch_mutcbuf});
            res
        }
        _ => {//unknown syscall
            -1
        }
    }
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

pub extern "C" fn lindrustinit() {
    load_fs();
    incref_root();
    let mut mutcagetable = CAGE_TABLE.write().unwrap();


    //init cage is its own parent
    let mut initcage = Cage{
        cageid: 1, cwd: interface::RustLock::new(interface::RustRfc::new(interface::RustPathBuf::from("/"))), 
        parent: 1, filedescriptortable: interface::RustLock::new(interface::RustHashMap::new())};
    initcage.load_lower_handle_stubs();
    mutcagetable.insert(1, interface::RustRfc::new(initcage));

}

pub extern "C" fn lindrustfinalize() {
    //wipe all keys from hashmap, i.e. free all cages
    let mut cagetable = CAGE_TABLE.write().unwrap();
    let drainedcages: Vec<(u64, interface::RustRfc<Cage>)> = cagetable.drain().collect();
    drop(cagetable);
    for (_cageid, cage) in drainedcages {
        cage.exit_syscall();
    }
}

#[cfg(test)]
pub mod dispatch_tests {
    use super::*;
    pub fn cagetest() {
        lindrustinit();

        {interface::RustRfc::get_mut(CAGE_TABLE.write().unwrap().get_mut(&1).unwrap()).unwrap().load_lower_handle_stubs();}
        {println!("{:?}", CAGE_TABLE.read().unwrap());};
        {println!("{:?}", FS_METADATA.read().unwrap().inodetable);};
        dispatcher(1, FORK_SYSCALL, Arg {dispatch_ulong: 2_u64}, Arg {dispatch_int: 34132}, Arg {dispatch_int: 109384}, Arg {dispatch_int: -12341}, Arg {dispatch_int: -12341}, Arg {dispatch_int: 0});
        {println!("{:?}", CAGE_TABLE.read().unwrap());};
        dispatcher(2, EXEC_SYSCALL, Arg {dispatch_ulong: 7_u64}, Arg {dispatch_int: 34132}, Arg {dispatch_int: 109384}, Arg {dispatch_int: -12341}, Arg {dispatch_int: -12341}, Arg {dispatch_int: 0});
        {println!("{:?}", CAGE_TABLE.read().unwrap());};
        dispatcher(7, EXIT_SYSCALL, Arg {dispatch_ulong: 61_u64}, Arg {dispatch_int: 33987}, Arg {dispatch_int: 123452}, Arg {dispatch_int: -98493}, Arg {dispatch_int: -1}, Arg {dispatch_int: 0});
        {println!("{:?}", CAGE_TABLE.read().unwrap());};

        lindrustfinalize();
    }
}
