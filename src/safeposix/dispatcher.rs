#![allow(dead_code)]
#![allow(unused_variables)]
// retreive cage table

const ACCESS_SYSCALL: i32 = 2;
const UNLINK_SYSCALL: i32 = 4;
const LINK_SYSCALL: i32 = 5;
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

const BIND_SYSCALL: i32 = 33;
const SEND_SYSCALL: i32 = 34;
const SENDTO_SYSCALL: i32 = 35;
const RECV_SYSCALL: i32 = 36;
const RECVFROM_SYSCALL: i32 = 37;
const CONNECT_SYSCALL: i32 = 38;
const LISTEN_SYSCALL: i32 = 39;
const ACCEPT_SYSCALL: i32 = 40;

const GETSOCKOPT_SYSCALL: i32 = 43;
const SETSOCKOPT_SYSCALL: i32 = 44;
const SHUTDOWN_SYSCALL: i32 = 45;
const SELECT_SYSCALL: i32 = 46;
const GETCWD_SYSCALL: i32 = 47;
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
const CHDIR_SYSCALL: i32 = 130;
const MKDIR_SYSCALL: i32 = 131;
const RMDIR_SYSCALL: i32 = 132;
const SOCKET_SYSCALL: i32 = 136;

const GETSOCKNAME_SYSCALL: i32 = 144;
const GETPEERNAME_SYSCALL: i32 = 145;


use crate::interface;
use super::cage::{Arg, CAGE_TABLE, Cage, FSData, StatData};
use super::filesystem::{FS_METADATA, load_fs, incref_root, persist_metadata};
use crate::interface::errnos::*;
use super::syscalls::sys_constants::*;

macro_rules! get_onearg {
    ($arg: expr) => {
        match (move || Ok($arg?))() {
            Ok(okval) => okval,
            Err(e) => return e
        }
    };
}

//this macro takes in a syscall invocation name (i.e. cage.fork_syscall), and all of the arguments
//to the syscall. Then it unwraps the arguments, returning the error if any one of them is an error
//value, and returning the value of the function if not. It does this by using the ? operator in
//the body of a closure within the variadic macro
macro_rules! check_and_dispatch {
    ( $cage:ident . $func:ident, $($arg:expr),* ) => {
        (|| Ok($cage.$func( $($arg?),* )))().into_ok_or_err()
    };
}

macro_rules! check_and_dispatch_socketpair {
    ( $func:expr, $cage:ident, $($arg:expr),* ) => {
        (|| Ok($func( $cage, $($arg?),* )))().into_ok_or_err()
    };
}

#[no_mangle]
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
        IOCTL_SYSCALL => {
            check_and_dispatch!(cage.ioctl_syscall, interface::get_int(arg1), interface::get_uint(arg2), interface::get_ioctlunion(arg3))
        }
        GETPPID_SYSCALL => {
            check_and_dispatch!(cage.getppid_syscall,)
        }
        GETPID_SYSCALL => {
            check_and_dispatch!(cage.getpid_syscall,)
        }
        SOCKET_SYSCALL => {
            check_and_dispatch!(cage.socket_syscall, interface::get_int(arg1), interface::get_int(arg2), interface::get_int(arg3))
        }

        BIND_SYSCALL => {
            let addrlen = get_onearg!(interface::get_uint(arg3));
            let addr = get_onearg!(interface::get_sockaddr(arg2, addrlen));
            check_and_dispatch!(cage.bind_syscall, interface::get_int(arg1), Ok::<&interface::GenSockaddr, i32>(&addr))
        }

        SEND_SYSCALL => {
            check_and_dispatch!(cage.send_syscall, interface::get_int(arg1), interface::get_cbuf(arg2), interface::get_usize(arg3), interface::get_int(arg4))
        }
        SENDTO_SYSCALL => {
            let addrlen = get_onearg!(interface::get_uint(arg6));
            let addr = get_onearg!(interface::get_sockaddr(arg5, addrlen));
            check_and_dispatch!(cage.sendto_syscall, interface::get_int(arg1), interface::get_cbuf(arg2), interface::get_usize(arg3), interface::get_int(arg4), Ok::<&interface::GenSockaddr, i32>(&addr))
        }

        RECV_SYSCALL => {
            check_and_dispatch!(cage.recv_syscall, interface::get_int(arg1), interface::get_mutcbuf(arg2), interface::get_usize(arg3), interface::get_int(arg4))
        }
        RECVFROM_SYSCALL => {
            let nullity1 = interface::arg_nullity(&arg5);
            let nullity2 = interface::arg_nullity(&arg6);

            if nullity1 && nullity2 {
                check_and_dispatch!(cage.recvfrom_syscall, interface::get_int(arg1), interface::get_mutcbuf(arg2), interface::get_usize(arg3), interface::get_int(arg4), Ok::<&mut Option<&mut interface::GenSockaddr>, i32>(&mut None))
            } else if !(nullity1 || nullity2) {
                let addrlen = get_onearg!(interface::get_socklen_t_ptr(arg6));
                let mut addr = get_onearg!(interface::get_sockaddr(arg5, addrlen));
                let rv = check_and_dispatch!(cage.recvfrom_syscall, interface::get_int(arg1), interface::get_mutcbuf(arg2), interface::get_usize(arg3), interface::get_int(arg4), Ok::<&mut Option<&mut interface::GenSockaddr>, i32>(&mut Some(&mut addr)));

                if rv >= 0 {
                    interface::copy_out_sockaddr(arg5, arg6, addr);
                }
                rv
            } else {
                syscall_error(Errno::EINVAL, "recvfrom", "exactly one of the last two arguments was zero")
            }
        }

        CONNECT_SYSCALL => {
            let addrlen = get_onearg!(interface::get_uint(arg3));
            let addr = get_onearg!(interface::get_sockaddr(arg2, addrlen));
            check_and_dispatch!(cage.connect_syscall, interface::get_int(arg1), Ok::<&interface::GenSockaddr, i32>(&addr))
        }
        LISTEN_SYSCALL => {
            check_and_dispatch!(cage.listen_syscall, interface::get_int(arg1), interface::get_int(arg2))
        }
        ACCEPT_SYSCALL => {
            let mut addr = interface::GenSockaddr::V4(interface::SockaddrV4::default()); //value doesn't matter
            let nullity1 = interface::arg_nullity(&arg2);
            let nullity2 = interface::arg_nullity(&arg3);
            
            if nullity1 && nullity2 {
                check_and_dispatch!(cage.accept_syscall, interface::get_int(arg1), Ok::<&mut interface::GenSockaddr, i32>(&mut addr))
            } else if !(nullity1 || nullity2) {
                let rv = check_and_dispatch!(cage.accept_syscall, interface::get_int(arg1), Ok::<&mut interface::GenSockaddr, i32>(&mut addr));
                if rv >= 0 {
                    interface::copy_out_sockaddr(arg2, arg3, addr);
                }
                rv
            } else {
                syscall_error(Errno::EINVAL, "accept", "exactly one of the last two arguments was zero")
            }
        }

        GETPEERNAME_SYSCALL => {
            let mut addr = interface::GenSockaddr::V4(interface::SockaddrV4::default()); //value doesn't matter
            if interface::arg_nullity(&arg2) || interface::arg_nullity(&arg3) {
                return syscall_error(Errno::EINVAL, "getpeername", "Either the address or the length were null");
            }
            let rv = check_and_dispatch!(cage.getpeername_syscall, interface::get_int(arg1), Ok::<&mut interface::GenSockaddr, i32>(&mut addr));

            if rv >= 0 {
                interface::copy_out_sockaddr(arg2, arg3, addr);
            }
            rv
        }
        GETSOCKNAME_SYSCALL => {
            let mut addr = interface::GenSockaddr::V4(interface::SockaddrV4::default()); //value doesn't matter
            if interface::arg_nullity(&arg2) || interface::arg_nullity(&arg3) {
                return syscall_error(Errno::EINVAL, "getsockname", "Either the address or the length were null");
            }
            let rv = check_and_dispatch!(cage.getsockname_syscall, interface::get_int(arg1), Ok::<&mut interface::GenSockaddr, i32>(&mut addr));

            if rv >= 0 {
                interface::copy_out_sockaddr(arg2, arg3, addr);
            }
            rv
        }

        GETSOCKOPT_SYSCALL => {
            let mut sockval = 0;
            if interface::arg_nullity(&arg4) || interface::arg_nullity(&arg5) {
                return syscall_error(Errno::EFAULT, "getsockopt", "Optval or optlen passed as null");
            }
            if get_onearg!(interface::get_socklen_t_ptr(arg5)) != 4 {
                return syscall_error(Errno::EINVAL, "setsockopt", "Invalid optlen passed");
            }
            let rv = check_and_dispatch!(cage.getsockopt_syscall, interface::get_int(arg1), interface::get_int(arg2), interface::get_int(arg3), Ok::<&mut i32, i32>(&mut sockval));

            if rv >= 0 {
                interface::copy_out_intptr(arg4, sockval);
            }
            //we take it as a given that the length is 4 both in and out
            rv
        }

        SETSOCKOPT_SYSCALL => {
            let sockval;
            if !interface::arg_nullity(&arg4) {
                if get_onearg!(interface::get_uint(arg5)) != 4 {
                    return syscall_error(Errno::EINVAL, "setsockopt", "Invalid optlen passed");
                }
                sockval = interface::get_int_from_intptr(arg4);
            } else {
                sockval = 0;
            }
            check_and_dispatch!(cage.setsockopt_syscall, interface::get_int(arg1), interface::get_int(arg2), interface::get_int(arg3), Ok::<i32, i32>(sockval))
        }

        SHUTDOWN_SYSCALL => {
            check_and_dispatch!(cage.netshutdown_syscall, interface::get_int(arg1), interface::get_int(arg2))
        }

        SELECT_SYSCALL => {
            let nfds = get_onearg!(interface::get_int(arg1));
            if nfds < 0 { //RLIMIT_NOFILE check as well?
                return syscall_error(Errno::EINVAL, "select", "The number of fds passed was invalid");
            }
            let mut readfds = get_onearg!(interface::fd_set_to_hashset(arg2, nfds));
            let mut writefds = get_onearg!(interface::fd_set_to_hashset(arg3, nfds));
            let mut exceptfds = get_onearg!(interface::fd_set_to_hashset(arg4, nfds));

            let rv = check_and_dispatch!(cage.select_syscall, Ok::<i32, i32>(nfds), Ok::<&mut interface::RustHashSet<i32>, i32>(&mut readfds), Ok::<&mut interface::RustHashSet<i32>, i32>(&mut writefds), Ok::<&mut interface::RustHashSet<i32>, i32>(&mut exceptfds), interface::duration_fromtimeval(arg5));

            interface::copy_out_to_fd_set(arg2, nfds, readfds);
            interface::copy_out_to_fd_set(arg3, nfds, writefds);
            interface::copy_out_to_fd_set(arg4, nfds, exceptfds);

            rv
        }
        POLL_SYSCALL => {
            let nfds = get_onearg!(interface::get_usize(arg2));
            check_and_dispatch!(cage.poll_syscall, interface::get_pollstruct_slice(arg1, nfds), interface::get_duration_from_millis(arg3))
        }

        SOCKETPAIR_SYSCALL => {
            check_and_dispatch_socketpair!(Cage::socketpair_syscall, cage, interface::get_int(arg1), interface::get_int(arg2), interface::get_int(arg3), interface::get_sockpair(arg4))
        }

        EXIT_SYSCALL => {
            check_and_dispatch!(cage.exit_syscall, interface::get_int(arg1))
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
        // CHMOD_SYSCALL => { //doesn't have a number value yet
        //     check_and_dispatch!(cage.chmod_syscall, interface::get_cstr(arg1), interface::get_uint(arg2))
        // }
        RMDIR_SYSCALL => {
            check_and_dispatch!(cage.rmdir_syscall, interface::get_cstr(arg1))
        }
        RENAME_SYSCALL => {
            check_and_dispatch!(cage.rename_syscall, interface::get_cstr(arg1), interface::get_cstr(arg2))
        }
        EPOLL_CREATE_SYSCALL => {
            check_and_dispatch!(cage.epoll_create_syscall, interface::get_int(arg1))
        }
        EPOLL_CTL_SYSCALL => {
            check_and_dispatch!(cage.epoll_ctl_syscall, interface::get_int(arg1), interface::get_int(arg2), interface::get_int(arg3), interface::get_epollevent(arg4))
        }
        EPOLL_WAIT_SYSCALL => {
            let nfds = get_onearg!(interface::get_int(arg3));

            if nfds < 0 { //RLIMIT_NOFILE check as well?
                return syscall_error(Errno::EINVAL, "select", "The number of fds passed was invalid");
            }

            check_and_dispatch!(cage.epoll_wait_syscall, interface::get_int(arg1), interface::get_epollevent_slice(arg2, nfds), Ok::<i32, i32>(nfds), interface::get_duration_from_millis(arg4))
        }
        GETDENTS_SYSCALL => {
            check_and_dispatch!(cage.getdents_syscall, interface::get_int(arg1), interface::get_mutcbuf(arg2), interface::get_uint(arg3))
        }
        PIPE_SYSCALL => {
            check_and_dispatch!(cage.pipe_syscall, interface::get_pipearray(arg1))
        }
        GETCWD_SYSCALL => {
            check_and_dispatch!(cage.getcwd_syscall, interface::get_mutcbuf(arg1), interface::get_uint(arg2))
        }
        GETHOSTNAME_SYSCALL => {
            check_and_dispatch!(cage.gethostname_syscall, interface::get_mutcbuf(arg1), interface::get_isize(arg2))
        }
        MKDIR_SYSCALL => {
            check_and_dispatch!(cage.mkdir_syscall, interface::get_cstr(arg1), interface::get_uint(arg2))
        }
        _ => {//unknown syscall
            -1
        }
    }
}

#[no_mangle]
pub extern "C" fn lindrustinit() {
    load_fs();
    incref_root();
    incref_root();
    let mut mutcagetable = CAGE_TABLE.write().unwrap();

    let utilcage = Cage{
        cageid: 0, cwd: interface::RustLock::new(interface::RustRfc::new(interface::RustPathBuf::from("/"))),
        parent: 0, filedescriptortable: interface::RustLock::new(interface::RustHashMap::new())};
    mutcagetable.insert(0, interface::RustRfc::new(utilcage));

    //init cage is its own parent
    let mut initcage = Cage{
        cageid: 1, cwd: interface::RustLock::new(interface::RustRfc::new(interface::RustPathBuf::from("/"))),
        parent: 1, filedescriptortable: interface::RustLock::new(interface::RustHashMap::new())};
    initcage.load_lower_handle_stubs();
    mutcagetable.insert(1, interface::RustRfc::new(initcage));

}

#[no_mangle]
pub extern "C" fn lindrustfinalize() {
    //wipe all keys from hashmap, i.e. free all cages
    let mut cagetable = CAGE_TABLE.write().unwrap();
    let drainedcages: Vec<(u64, interface::RustRfc<Cage>)> = cagetable.drain().collect();
    drop(cagetable);
    for (_cageid, cage) in drainedcages {
        cage.exit_syscall(EXIT_SUCCESS);
    }
    persist_metadata(&*FS_METADATA.read().unwrap());
}
