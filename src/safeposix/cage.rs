#![allow(dead_code)]
use crate::interface;
//going to get the datatypes and errnos from the cage file from now on
pub use crate::interface::errnos::{Errno, syscall_error};
pub use crate::interface::types::{Arg, EpollEvent, FSData, Rlimit, StatData, PipeArray, PollStruct, IoctlPtrUnion, ShmidsStruct};

pub use super::syscalls::fs_constants::*;
pub use super::syscalls::sys_constants::*;
pub use super::syscalls::net_constants::*;
use super::filesystem::normpath;

pub use crate::interface::{CAGE_TABLE};

#[derive(Debug, Clone)]
pub enum FileDescriptor {
    File(FileDesc),
    Stream(StreamDesc),
    Socket(SocketDesc),
    Pipe(PipeDesc),
    Epoll(EpollDesc)
}

#[derive(Debug, Clone)]
pub struct FileDesc {
    pub position: usize,
    pub inode: usize,
    pub flags: i32,
    pub advlock: interface::RustRfc<interface::AdvisoryLock>
}

#[derive(Debug, Clone)]
pub struct StreamDesc {
    pub position: usize,
    pub stream: i32, //0 for stdin, 1 for stdout, 2 for stderr
    pub flags: i32,
    pub advlock: interface::RustRfc<interface::AdvisoryLock>
}

#[derive(Debug, Clone)]
pub struct SocketDesc {
    pub mode: i32,
    pub domain: i32,
    pub realdomain: i32,
    pub reallocalpath: Option<interface::RustPathBuf>,
    pub optinode: Option<usize>,
    pub socktype: i32,
    pub protocol: i32,
    pub options: i32,
    pub sndbuf: i32,
    pub rcvbuf: i32,
    pub flags: i32,
    pub errno: i32,
    pub localaddr: Option<interface::GenSockaddr>,
    pub remoteaddr: Option<interface::GenSockaddr>,
    pub last_peek: interface::RustDeque<u8>,
    pub socketobjectid: Option<i32>,
    pub advlock: interface::RustRfc<interface::AdvisoryLock>
}

#[derive(Debug, Clone)]
pub struct PipeDesc {
    pub pipe: interface::RustRfc<interface::EmulatedPipe>,
    pub flags: i32,
    pub advlock: interface::RustRfc<interface::AdvisoryLock>
}

#[derive(Debug, Clone)]
pub struct EpollDesc {
    pub mode: i32,
    pub registered_fds: interface::RustHashMap<i32, EpollEvent>,
    pub advlock: interface::RustRfc<interface::AdvisoryLock>,
    pub errno: i32,
    pub flags: i32
}

pub type FdTable = Vec<interface::RustRfc<interface::RustLock<Option<FileDescriptor>>>>;

#[derive(Debug)]
pub struct Cage {
    pub cageid: u64,
    pub cwd: interface::RustLock<interface::RustRfc<interface::RustPathBuf>>,
    pub parent: u64,
    pub filedescriptortable: FdTable,
    pub getgid: interface::RustAtomicI32,
    pub getuid: interface::RustAtomicI32,
    pub getegid: interface::RustAtomicI32,
    pub geteuid: interface::RustAtomicI32,
    pub rev_shm: interface::Mutex<Vec<(u32, i32)>>, //maps addr within cage to shmid
    pub mutex_table: interface::RustLock<Vec<Option<interface::RustRfc<interface::RawMutex>>>>,
    pub cv_table: interface::RustLock<Vec<Option<interface::RustRfc<interface::RawCondvar>>>>
}

impl Cage {

    pub fn get_next_fd(&self, startfd: Option<i32>) -> (i32, Option<interface::RustLockGuard<Option<FileDescriptor>>>) {

        let start = match startfd {
            Some(startfd) => startfd,
            None => STARTINGFD,
        };

        // let's get the next available fd number. The standard says we need to return the lowest open fd number.
        for fd in start..MAXFD{
            let fdguard = self.filedescriptortable[fd as usize].try_write();
            if let Some(ref fdopt) = fdguard {
                // we grab the lock here and if there is no occupied cage, we return the fdno and guard while keeping the fd slot locked
                if fdopt.is_none() { return (fd, fdguard); }
            }
        };
        return (syscall_error(Errno::ENFILE, "get_next_fd", "no available file descriptor number could be found"), None);
    }

    pub fn changedir(&self, newdir: interface::RustPathBuf) {
        let newwd = interface::RustRfc::new(normpath(newdir, self));
        let mut cwdbox = self.cwd.write();
        *cwdbox = newwd;
    }

}

pub fn init_fdtable() -> FdTable {
    let mut fdtable = Vec::new();
    // load lower handle stubs
    let stdin = interface::RustRfc::new(interface::RustLock::new(Some(FileDescriptor::Stream(StreamDesc {position: 0, stream: 0, flags: O_RDONLY, advlock: interface::RustRfc::new(interface::AdvisoryLock::new())}))));
    let stdout = interface::RustRfc::new(interface::RustLock::new(Some(FileDescriptor::Stream(StreamDesc {position: 0, stream: 1, flags: O_WRONLY, advlock: interface::RustRfc::new(interface::AdvisoryLock::new())}))));
    let stderr = interface::RustRfc::new(interface::RustLock::new(Some(FileDescriptor::Stream(StreamDesc {position: 0, stream: 2, flags: O_WRONLY, advlock: interface::RustRfc::new(interface::AdvisoryLock::new())}))));
    fdtable.push(stdin);
    fdtable.push(stdout);
    fdtable.push(stderr);

    for _fd in 3..MAXFD as usize {
        fdtable.push(interface::RustRfc::new(interface::RustLock::new(None)));
    }
    fdtable
}
