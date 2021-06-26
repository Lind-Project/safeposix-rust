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
use super::filesystem::{FS_METADATA};


#[repr(C)]
pub union Arg {
  dispatch_int: i32,
  dispatch_uint: u32,
  dispatch_ulong: u64,
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
        LSEEK_SYSCALL => {
            cage.lseek_syscall(unsafe{arg1.dispatch_int}, unsafe{arg2.dispatch_isize}, unsafe{arg3.dispatch_int})
        }
        FXSTAT_SYSCALL => {
            cage.fstat_syscall(unsafe{arg1.dispatch_int}, unsafe{&mut *arg2.dispatch_statdatastruct})
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
        EXEC_SYSCALL => {
            cage.exec_syscall(unsafe{arg1.dispatch_ulong})
        }
        FORK_SYSCALL => {
            cage.fork_syscall(unsafe{arg1.dispatch_ulong})
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
        _ => {//unknown syscall
            -1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    pub fn cagetest() {

        {CAGE_TABLE.write().unwrap().insert(1, interface::RustRfc::new(Cage{cageid: 1, cwd: interface::RustLock::new(interface::RustRfc::new(interface::RustPathBuf::from("/"))), parent: 0, filedescriptortable: interface::RustLock::new(interface::RustHashMap::new())}));}
        {interface::RustRfc::get_mut(CAGE_TABLE.write().unwrap().get_mut(&1).unwrap()).unwrap().load_lower_handle_stubs();}
        {println!("{:?}", CAGE_TABLE.read().unwrap());};
        {println!("{:?}", FS_METADATA.read().unwrap().inodetable);};
        dispatcher(1, FORK_SYSCALL, Arg {dispatch_ulong: 2_u64}, Arg {dispatch_int: 34132}, Arg {dispatch_int: 109384}, Arg {dispatch_int: -12341}, Arg {dispatch_int: -12341}, Arg {dispatch_int: 0});
        {println!("{:?}", CAGE_TABLE.read().unwrap());};
        dispatcher(2, EXEC_SYSCALL, Arg {dispatch_ulong: 7_u64}, Arg {dispatch_int: 34132}, Arg {dispatch_int: 109384}, Arg {dispatch_int: -12341}, Arg {dispatch_int: -12341}, Arg {dispatch_int: 0});
        {println!("{:?}", CAGE_TABLE.read().unwrap());};
        dispatcher(7, EXIT_SYSCALL, Arg {dispatch_ulong: 61_u64}, Arg {dispatch_int: 33987}, Arg {dispatch_int: 123452}, Arg {dispatch_int: -98493}, Arg {dispatch_int: -1}, Arg {dispatch_int: 0});
        {println!("{:?}", CAGE_TABLE.read().unwrap());};

    }
}
