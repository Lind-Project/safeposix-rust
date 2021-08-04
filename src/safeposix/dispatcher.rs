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
  pub dispatch_fsdatastruct: *mut FSData
}

//this macro takes in a syscall invocation name (i.e. cage.fork_syscall), and all of the arguments
//to the syscall. Then it unwraps the arguments, returning the error if any one of them is an error
//value, and returning the value of the function if not. It does this by using the ? operator in
//the body of a closure within the variadic macro
macro_rules! check_and_dispatch {
    ( $cage:ident . $func:ident, $($arg:expr),* ) => {
        (move || Ok($cage.$func( $($arg?),* )))().into_ok_or_err()
    };
}

pub extern "C" fn dispatcher(cageid: u64, callnum: i32, arg1: Arg, arg2: Arg, arg3: Arg, arg4: Arg, arg5: Arg, arg6: Arg) -> i32 {

    // need to match based on if cage exists
    let cage = { CAGE_TABLE.read().unwrap().get(&cageid).unwrap().clone() };

    match callnum {
        ACCESS_SYSCALL => {
            check_and_dispatch!(cage.access_syscall, interface::get_cstr(arg1), interface::get_uint(arg2))
        }
        UNLINK_SYSCALL => {
            check_and_dispatch!(cage.unlink_syscall, interface::get_cstr(arg1))
        }
        LINK_SYSCALL => {
            check_and_dispatch!(cage.link_syscall, interface::get_cstr(arg1), interface::get_cstr(arg2))
        }
        CHDIR_SYSCALL => {
            check_and_dispatch!(cage.chdir_syscall, interface::get_cstr(arg1))
        }
        XSTAT_SYSCALL => {
            check_and_dispatch!(cage.stat_syscall, interface::get_cstr(arg1), interface::get_statdatastruct(arg2))
        }
        OPEN_SYSCALL => {
            check_and_dispatch!(cage.open_syscall, interface::get_cstr(arg1), interface::get_int(arg2), interface::get_uint(arg3))
        }
        READ_SYSCALL => {
            check_and_dispatch!(cage.read_syscall, interface::get_int(arg1), interface::get_mutcbuf(arg2), interface::get_usize(arg3))
        }
        WRITE_SYSCALL => {
            check_and_dispatch!(cage.write_syscall, interface::get_int(arg1), interface::get_cbuf(arg2), interface::get_usize(arg3))
        }
        CLOSE_SYSCALL => {
            check_and_dispatch!(cage.close_syscall, interface::get_int(arg1))
        }
        LSEEK_SYSCALL => {
            check_and_dispatch!(cage.lseek_syscall, interface::get_int(arg1), interface::get_isize(arg2), interface::get_int(arg3))
        }
        FXSTAT_SYSCALL => {
            check_and_dispatch!(cage.fstat_syscall, interface::get_int(arg1), interface::get_statdatastruct(arg2))
        }
        FSTATFS_SYSCALL => {
            check_and_dispatch!(cage.fstatfs_syscall, interface::get_int(arg1), interface::get_fsdatastruct(arg2))
        }
        MMAP_SYSCALL => {
            check_and_dispatch!(cage.mmap_syscall, interface::get_mutcbuf(arg1), interface::get_usize(arg2), interface::get_int(arg3), interface::get_int(arg4), interface::get_int(arg5), interface::get_long(arg6))
        }
        MUNMAP_SYSCALL => {
            check_and_dispatch!(cage.munmap_syscall, interface::get_mutcbuf(arg1), interface::get_usize(arg2))
        }
        DUP_SYSCALL => {
            check_and_dispatch!(cage.dup_syscall, interface::get_int(arg1), Ok::<Option<i32>, i32>(None))
        }
        DUP2_SYSCALL => {
            check_and_dispatch!(cage.dup2_syscall, interface::get_int(arg1), interface::get_int(arg2))
        }
        STATFS_SYSCALL => {
            check_and_dispatch!(cage.statfs_syscall, interface::get_cstr(arg1), interface::get_fsdatastruct(arg2))
        }
        FCNTL_SYSCALL => {
            check_and_dispatch!(cage.fcntl_syscall, interface::get_int(arg1), interface::get_int(arg2), interface::get_int(arg3))
        }
        GETPPID_SYSCALL => {
            check_and_dispatch!(cage.getppid_syscall,)
        }
        GETPID_SYSCALL => {
            check_and_dispatch!(cage.getpid_syscall,)
        }
        EXIT_SYSCALL => {
            check_and_dispatch!(cage.exit_syscall,)
        }
        FLOCK_SYSCALL => {
            check_and_dispatch!(cage.flock_syscall, interface::get_int(arg1), interface::get_int(arg2))
        }
        FORK_SYSCALL => {
            check_and_dispatch!(cage.fork_syscall, interface::get_ulong(arg1))
        }
        EXEC_SYSCALL => {
            check_and_dispatch!(cage.exec_syscall, interface::get_ulong(arg1))
        }
        GETUID_SYSCALL => {
            check_and_dispatch!(cage.getuid_syscall,)
        }
        GETEUID_SYSCALL => {
            check_and_dispatch!(cage.geteuid_syscall,)
        }
        GETGID_SYSCALL => {
            check_and_dispatch!(cage.getgid_syscall,)
        }
        GETEGID_SYSCALL => {
            check_and_dispatch!(cage.getegid_syscall,)
        }
        PREAD_SYSCALL => {
            check_and_dispatch!(cage.pread_syscall, interface::get_int(arg1), interface::get_mutcbuf(arg2), interface::get_usize(arg3), interface::get_isize(arg4))
        }
        PWRITE_SYSCALL => {
            check_and_dispatch!(cage.pwrite_syscall, interface::get_int(arg1), interface::get_mutcbuf(arg2), interface::get_usize(arg3), interface::get_isize(arg4))
        }
        // CHMOD_SYSCALL => {
        //     check_and_dispatch!(cage.chmod_syscall, interface::get_cstr(arg1), interface::get_uint(arg2))
        // }
        RMDIR_SYSCALL => {
            check_and_dispatch!(cage.rmdir_syscall, interface::get_cstr(arg1))
        }
        RENAME_SYSCALL => {
            check_and_dispatch!(cage.rename_syscall, interface::get_cstr(arg1), interface::get_cstr(arg2))
        }
        FTRUNCATE_SYSCALL => {
            cage.ftruncate_syscall(unsafe{arg1.dispatch_int}, unsafe{arg2.dispatch_usize})
        }
        TRUNCATE_SYSCALL => {
            cage.truncate_syscall(unsafe{interface::charstar_to_ruststr(arg1.dispatch_cstr)}, unsafe{arg2.dispatch_usize})
        }
        _ => {//unknown syscall
            -1
        }
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
