// Define all syscall numbers
#![allow(dead_code)]
#![allow(unused_variables)]

use crate::interface;

// Define constants using static or const

pub const MAX_SYSCALL_NUMBER: usize = 256;

pub const ACCESS_SYSCALL: i32 = 2;
pub const UNLINK_SYSCALL: i32 = 4;
pub const LINK_SYSCALL: i32 = 5;
pub const RENAME_SYSCALL: i32 = 6;
 
pub const XSTAT_SYSCALL: i32 = 9;
pub const OPEN_SYSCALL: i32 = 10;
pub const CLOSE_SYSCALL: i32 = 11;
pub const READ_SYSCALL: i32 = 12;
pub const WRITE_SYSCALL: i32 = 13;
pub const LSEEK_SYSCALL: i32 = 14;
pub const IOCTL_SYSCALL: i32 = 15;
pub const FXSTAT_SYSCALL: i32 = 17;
pub const FSTATFS_SYSCALL: i32 = 19;
pub const MMAP_SYSCALL: i32 = 21;
pub const MUNMAP_SYSCALL: i32 = 22;
pub const GETDENTS_SYSCALL: i32 = 23;
pub const DUP_SYSCALL: i32 = 24;
pub const DUP2_SYSCALL: i32 = 25;
pub const STATFS_SYSCALL: i32 = 26;
pub const FCNTL_SYSCALL: i32 = 28;
 
pub const GETPPID_SYSCALL: i32 = 29;
pub const EXIT_SYSCALL: i32 = 30;
pub const GETPID_SYSCALL: i32 = 31;
 
pub const BIND_SYSCALL: i32 = 33;
pub const SEND_SYSCALL: i32 = 34;
pub const SENDTO_SYSCALL: i32 = 35;
pub const RECV_SYSCALL: i32 = 36;
pub const RECVFROM_SYSCALL: i32 = 37;
pub const CONNECT_SYSCALL: i32 = 38;
pub const LISTEN_SYSCALL: i32 = 39;
pub const ACCEPT_SYSCALL: i32 = 40;
 
pub const GETSOCKOPT_SYSCALL: i32 = 43;
pub const SETSOCKOPT_SYSCALL: i32 = 44;
pub const SHUTDOWN_SYSCALL: i32 = 45;
pub const SELECT_SYSCALL: i32 = 46;
pub const GETCWD_SYSCALL: i32 = 47;
pub const POLL_SYSCALL: i32 = 48;
pub const SOCKETPAIR_SYSCALL: i32 = 49;
pub const GETUID_SYSCALL: i32 = 50;
pub const GETEUID_SYSCALL: i32 = 51;
pub const GETGID_SYSCALL: i32 = 52;
pub const GETEGID_SYSCALL: i32 = 53;
pub const FLOCK_SYSCALL: i32 = 54;
pub const EPOLL_CREATE_SYSCALL: i32 = 56;
pub const EPOLL_CTL_SYSCALL: i32 = 57;
pub const EPOLL_WAIT_SYSCALL: i32 = 58;
 
pub const SHMGET_SYSCALL: i32 = 62;
pub const SHMAT_SYSCALL: i32 = 63;
pub const SHMDT_SYSCALL: i32 = 64;
pub const SHMCTL_SYSCALL: i32 = 65;
 
pub const PIPE_SYSCALL: i32 = 66;
pub const PIPE2_SYSCALL: i32 = 67;
pub const FORK_SYSCALL: i32 = 68;
pub const EXEC_SYSCALL: i32 = 69;
 
pub const GETHOSTNAME_SYSCALL: i32 = 125;
pub const PREAD_SYSCALL: i32 = 126;
pub const PWRITE_SYSCALL: i32 = 127;
pub const CHDIR_SYSCALL: i32 = 130;
pub const MKDIR_SYSCALL: i32 = 131;
pub const RMDIR_SYSCALL: i32 = 132;
pub const CHMOD_SYSCALL: i32 = 133;
 
pub const SOCKET_SYSCALL: i32 = 136;
 
pub const GETSOCKNAME_SYSCALL: i32 = 144;
pub const GETPEERNAME_SYSCALL: i32 = 145;
pub const GETIFADDRS_SYSCALL: i32 = 146;
