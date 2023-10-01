#![allow(dead_code)]
#![allow(unused_variables)]
// retreive cage table

const ACCESS_SYSCALL: i32 = 2;
const UNLINK_SYSCALL: i32 = 4;
const LINK_SYSCALL: i32 = 5;
const RENAME_SYSCALL: i32 = 6;

const XSTAT_SYSCALL: i32 = 9;
const OPEN_SYSCALL: i32 = 10;
const CLOSE_SYSCALL: i32 = 11;
const READ_SYSCALL: i32 = 12;
const WRITE_SYSCALL: i32 = 13;
const LSEEK_SYSCALL: i32 = 14;
const IOCTL_SYSCALL: i32 = 15;
const TRUNCATE_SYSCALL: i32 = 16;
const FXSTAT_SYSCALL: i32 = 17;
const FTRUNCATE_SYSCALL: i32 = 18;
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
const EPOLL_CREATE_SYSCALL: i32 = 56;
const EPOLL_CTL_SYSCALL: i32 = 57;
const EPOLL_WAIT_SYSCALL: i32 = 58;

const SHMGET_SYSCALL: i32 = 62;
const SHMAT_SYSCALL: i32 = 63;
const SHMDT_SYSCALL: i32 = 64;
const SHMCTL_SYSCALL: i32 = 65;

const PIPE_SYSCALL: i32 = 66;
const PIPE2_SYSCALL: i32 = 67;
const FORK_SYSCALL: i32 = 68;
const EXEC_SYSCALL: i32 = 69;

const MUTEX_CREATE_SYSCALL: i32 = 70;
const MUTEX_DESTROY_SYSCALL: i32 = 71;
const MUTEX_LOCK_SYSCALL: i32 = 72;
const MUTEX_TRYLOCK_SYSCALL: i32 = 73;
const MUTEX_UNLOCK_SYSCALL: i32 = 74;
const COND_CREATE_SYSCALL: i32 = 75;
const COND_DESTROY_SYSCALL: i32 = 76;
const COND_WAIT_SYSCALL: i32 = 77;
const COND_BROADCAST_SYSCALL: i32 = 78;
const COND_SIGNAL_SYSCALL: i32 = 79;
const COND_TIMEDWAIT_SYSCALL: i32 = 80;

const SEM_INIT_SYSCALL: i32 = 91;
const SEM_WAIT_SYSCALL: i32 = 92;
const SEM_TRYWAIT_SYSCALL: i32 = 93;
const SEM_TIMEDWAIT_SYSCALL: i32 = 94;
const SEM_POST_SYSCALL: i32 = 95;
const SEM_DESTROY_SYSCALL: i32 = 96;
const SEM_GETVALUE_SYSCALL: i32 = 97;

const GETHOSTNAME_SYSCALL: i32 = 125;
const PREAD_SYSCALL: i32 = 126;
const PWRITE_SYSCALL: i32 = 127;
const CHDIR_SYSCALL: i32 = 130;
const MKDIR_SYSCALL: i32 = 131;
const RMDIR_SYSCALL: i32 = 132;
const CHMOD_SYSCALL: i32 = 133;
const FCHMOD_SYSCALL: i32 = 134;

const SOCKET_SYSCALL: i32 = 136;

const GETSOCKNAME_SYSCALL: i32 = 144;
const GETPEERNAME_SYSCALL: i32 = 145;
const GETIFADDRS_SYSCALL: i32 = 146;

const FCHDIR_SYSCALL: i32 = 161;

use crate::interface;
use super::cage::*;
use super::filesystem::{FS_METADATA, load_fs, incref_root, remove_domain_sock, persist_metadata, LOGMAP, LOGFILENAME, FilesystemMetadata};
use super::shm::{SHM_METADATA};
use super::net::{NET_METADATA};
use crate::interface::errnos::*;
use super::syscalls::sys_constants::*;
use super::syscalls::fs_constants::IPC_STAT;

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
        match (|| Ok($cage.$func( $($arg?),* )))() {
            Ok(i) => i, Err(i) => i
        }
    };
}

macro_rules! check_and_dispatch_socketpair {
    ( $func:expr, $cage:ident, $($arg:expr),* ) => {
        match (|| Ok($func( $cage, $($arg?),* )))() {
            Ok(i) => i, Err(i) => i
        }
    };
}


// the following "quick" functions are implemented for research purposes
// to increase I/O performance by bypassing the dispatcher and type checker
#[no_mangle]
pub extern "C" fn quick_write(fd: i32, buf: *const u8, count: usize, cageid: u64) -> i32 {
  unsafe { CAGE_TABLE[cageid as usize].as_ref().unwrap().write_syscall(fd, buf, count) }
}

#[no_mangle]
pub extern "C" fn quick_read(fd: i32, buf: *mut u8, size: usize, cageid: u64) -> i32 {
    unsafe { CAGE_TABLE[cageid as usize].as_ref().unwrap().read_syscall(fd, buf, size) }
}

#[no_mangle]
pub extern "C" fn dispatcher(cageid: u64, callnum: i32, arg1: Arg, arg2: Arg, arg3: Arg, arg4: Arg, arg5: Arg, arg6: Arg) -> i32 {

    // need to match based on if cage exists
    let cage = interface::cagetable_getref(cageid);

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
        FCHDIR_SYSCALL => {
            check_and_dispatch!(cage.fchdir_syscall, interface::get_int(arg1))
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
            check_and_dispatch!(cage.ioctl_syscall, interface::get_int(arg1), interface::get_uint(arg2), interface::get_ioctlptrunion(arg3))
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
                let mut newsockaddr = interface::GenSockaddr::V4(interface::SockaddrV4::default()); //dummy value, rust would complain if we used an uninitialized value here
                let rv = check_and_dispatch!(cage.recvfrom_syscall, interface::get_int(arg1), interface::get_mutcbuf(arg2), interface::get_usize(arg3), interface::get_int(arg4), Ok::<&mut Option<&mut interface::GenSockaddr>, i32>(&mut Some(&mut newsockaddr)));

                if rv >= 0 {
                    interface::copy_out_sockaddr(arg5, arg6, newsockaddr);
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
        GETIFADDRS_SYSCALL => {
            check_and_dispatch!(cage.getifaddrs_syscall, interface::get_mutcbuf(arg1), interface::get_usize(arg2))
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
        CHMOD_SYSCALL => { 
            check_and_dispatch!(cage.chmod_syscall, interface::get_cstr(arg1), interface::get_uint(arg2))
        }
        FCHMOD_SYSCALL => { 
            check_and_dispatch!(cage.fchmod_syscall, interface::get_int(arg1), interface::get_uint(arg2))
        }
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
        PIPE2_SYSCALL => {
            check_and_dispatch!(cage.pipe2_syscall, interface::get_pipearray(arg1), interface::get_int(arg2))
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
        SHMGET_SYSCALL => {
            check_and_dispatch!(cage.shmget_syscall, interface::get_int(arg1), interface::get_usize(arg2), interface::get_int(arg3))
        }
        SHMAT_SYSCALL => {
            check_and_dispatch!(cage.shmat_syscall, interface::get_int(arg1), interface::get_mutcbuf(arg2), interface::get_int(arg3))
        }
        SHMDT_SYSCALL => {
            check_and_dispatch!(cage.shmdt_syscall, interface::get_mutcbuf(arg1))
        }
        SHMCTL_SYSCALL => {
            let cmd = get_onearg!(interface::get_int(arg2));
            let buf = if cmd == IPC_STAT {Some(get_onearg!(interface::get_shmidstruct(arg3)))} else {None};
            check_and_dispatch!(cage.shmctl_syscall, interface::get_int(arg1), Ok::<i32, i32>(cmd), Ok::<Option<&mut interface::ShmidsStruct>, i32>(buf))
        }

        MUTEX_CREATE_SYSCALL => {
            check_and_dispatch!(cage.mutex_create_syscall, )
        }
        MUTEX_DESTROY_SYSCALL => {
            check_and_dispatch!(cage.mutex_destroy_syscall, interface::get_int(arg1))
        }
        MUTEX_LOCK_SYSCALL => {
            check_and_dispatch!(cage.mutex_lock_syscall, interface::get_int(arg1))
        }
        MUTEX_TRYLOCK_SYSCALL => {
            check_and_dispatch!(cage.mutex_trylock_syscall, interface::get_int(arg1))
        }
        MUTEX_UNLOCK_SYSCALL => {
            check_and_dispatch!(cage.mutex_unlock_syscall, interface::get_int(arg1))
        }
        COND_CREATE_SYSCALL => {
            check_and_dispatch!(cage.cond_create_syscall, )
        }
        COND_DESTROY_SYSCALL => {
            check_and_dispatch!(cage.cond_destroy_syscall, interface::get_int(arg1))
        }
        COND_WAIT_SYSCALL => {
            check_and_dispatch!(cage.cond_wait_syscall, interface::get_int(arg1), interface::get_int(arg2))
        }
        COND_BROADCAST_SYSCALL => {
            check_and_dispatch!(cage.cond_broadcast_syscall, interface::get_int(arg1))
        }
        COND_SIGNAL_SYSCALL => {
            check_and_dispatch!(cage.cond_signal_syscall, interface::get_int(arg1))
        }
        COND_TIMEDWAIT_SYSCALL => {
            check_and_dispatch!(cage.cond_timedwait_syscall, interface::get_int(arg1), interface::get_int(arg2), interface::duration_fromtimespec(arg3))
        }
        TRUNCATE_SYSCALL => {
            check_and_dispatch!(cage.truncate_syscall, interface::get_cstr(arg1), interface::get_isize(arg2))
        }
        FTRUNCATE_SYSCALL => {
            check_and_dispatch!(cage.ftruncate_syscall, interface::get_int(arg1), interface::get_isize(arg2))
        }
        SEM_INIT_SYSCALL => {
            check_and_dispatch!(cage.sem_init_syscall, interface::get_uint(arg1), interface::get_int(arg2), interface::get_uint(arg3))
        }
        SEM_WAIT_SYSCALL => {
            check_and_dispatch!(cage.sem_wait_syscall, interface::get_uint(arg1))
        }
        SEM_POST_SYSCALL => {
            check_and_dispatch!(cage.sem_post_syscall, interface::get_uint(arg1))
        }
        SEM_DESTROY_SYSCALL => {
            check_and_dispatch!(cage.sem_destroy_syscall, interface::get_uint(arg1))
        }
        SEM_GETVALUE_SYSCALL => {
            check_and_dispatch!(cage.sem_getvalue_syscall, interface::get_uint(arg1))
        }
        SEM_TRYWAIT_SYSCALL => {
            check_and_dispatch!(cage.sem_trywait_syscall, interface::get_uint(arg1))
        }
        SEM_TIMEDWAIT_SYSCALL => {
            check_and_dispatch!(cage.sem_timedwait_syscall, interface::get_uint(arg1), interface::duration_fromtimespec(arg2))
        }

        _ => {//unknown syscall
            -1
        }
    }
}



#[no_mangle]
pub extern "C" fn lindcancelinit(cageid: u64) {
    let cage = interface::cagetable_getref(cageid);
    cage.cancelstatus.store(true, interface::RustAtomicOrdering::Relaxed);
    cage.signalcvs();
}

#[no_mangle]
pub extern "C" fn lindsetthreadkill(cageid: u64, pthreadid: u64, kill: bool) {
    let cage = interface::cagetable_getref(cageid);
    cage.thread_table.insert(pthreadid, kill);
}

#[no_mangle]
pub extern "C" fn lindcheckthread(cageid: u64, pthreadid: u64) -> bool {
    interface::check_thread(cageid, pthreadid)
}

#[no_mangle]
pub extern "C" fn lindthreadremove(cageid: u64, pthreadid: u64) {
    let cage = interface::cagetable_getref(cageid);
    cage.thread_table.remove(&pthreadid);
}

#[no_mangle]
pub extern "C" fn lindrustinit(verbosity: isize) {

    let _ = interface::VERBOSE.set(verbosity); //assigned to suppress unused result warning
    interface::cagetable_init();
    load_fs();
    incref_root();
    incref_root();
    
    let utilcage = Cage{
        cageid: 0, 
        cwd: interface::RustLock::new(interface::RustRfc::new(interface::RustPathBuf::from("/"))),
        parent: 0, 
        filedescriptortable: init_fdtable(),
        cancelstatus: interface::RustAtomicBool::new(false),
        getgid: interface::RustAtomicI32::new(-1), 
        getuid: interface::RustAtomicI32::new(-1), 
        getegid: interface::RustAtomicI32::new(-1), 
        geteuid: interface::RustAtomicI32::new(-1),
        rev_shm: interface::Mutex::new(vec!()),
        mutex_table: interface::RustLock::new(vec!()),
        cv_table: interface::RustLock::new(vec!()),
        thread_table: interface::RustHashMap::new(),
        sem_table: interface::RustHashMap::new(),
    };
    interface::cagetable_insert(0, utilcage);

    //init cage is its own parent
    let initcage = Cage{
        cageid: 1, 
        cwd: interface::RustLock::new(interface::RustRfc::new(interface::RustPathBuf::from("/"))),
        parent: 1, 
        filedescriptortable: init_fdtable(),
        cancelstatus: interface::RustAtomicBool::new(false),
        getgid: interface::RustAtomicI32::new(-1), 
        getuid: interface::RustAtomicI32::new(-1), 
        getegid: interface::RustAtomicI32::new(-1), 
        geteuid: interface::RustAtomicI32::new(-1),
        rev_shm: interface::Mutex::new(vec!()),
        mutex_table: interface::RustLock::new(vec!()),
        cv_table: interface::RustLock::new(vec!()),
        thread_table: interface::RustHashMap::new(),
        sem_table: interface::RustHashMap::new(),
    };
    interface::cagetable_insert(1, initcage);
}

#[no_mangle]
pub extern "C" fn lindrustfinalize() {

    interface::cagetable_clear();

    // remove any open domain socket inodes
    for truepath in NET_METADATA.get_domainsock_paths() {
        remove_domain_sock(truepath);
    }

    // if we get here, persist and delete log
    persist_metadata(&FS_METADATA);
    if interface::pathexists(LOGFILENAME.to_string()) {
        // remove file if it exists, assigning it to nothing to avoid the compiler yelling about unused result
        let mut logobj = LOGMAP.write();
        let log = logobj.take().unwrap();
        let _close = log.close().unwrap();
        let _logremove = interface::removefile(LOGFILENAME.to_string());
    }
}
