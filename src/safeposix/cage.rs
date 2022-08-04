#![allow(dead_code)]
use crate::interface;
//going to get the datatypes and errnos from the cage file from now on
pub use crate::interface::errnos::{Errno, syscall_error};
pub use crate::interface::types::{Arg, EpollEvent, FSData, Rlimit, StatData, PipeArray, PollStruct, IoctlPtrUnion, ShmidsStruct};

pub use super::syscalls::fs_constants::*;
pub use super::syscalls::sys_constants::*;
pub use super::syscalls::net_constants::*;
use super::filesystem::normpath;

pub static CAGE_TABLE: interface::RustLazyGlobal<interface::RustHashMap<u64, interface::RustRfc<Cage>>> = interface::RustLazyGlobal::new(|| interface::new_hashmap());

pub static PIPE_TABLE: interface::RustLazyGlobal<interface::RustHashMap<i32, interface::RustRfc<interface::EmulatedPipe>>> = 
    interface::RustLazyGlobal::new(|| 
        interface::new_hashmap()
);

#[derive(Debug, Clone)]
pub enum FileDescriptor {
    File(FileDesc),
    Stream(StreamDesc),
    Socket(SocketDesc),
    DomainSocket(DomainSocketDesc),
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
pub struct DomainSocketDesc {
    pub mode: i32,
    pub domain: i32,
    pub localpath: Option<interface::RustPathBuf>,
    pub inode: Option<usize>,
    pub socktype: i32,
    pub protocol: i32,
    pub options: i32,
    pub sndbuf: i32,
    pub rcvbuf: i32,
    pub pipe: i32,
    pub remotepipe: i32,
    pub flags: i32,
    pub errno: i32,
    pub localaddr: Option<interface::GenSockaddr>,
    pub remoteaddr: Option<interface::GenSockaddr>,
    pub state: ConnState,
    pub last_peek: interface::RustDeque<u8>,
    pub advlock: interface::RustRfc<interface::AdvisoryLock>
}

#[derive(Debug, Clone)]
pub struct PipeDesc {
    pub pipe: i32,
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

pub type FdTable = interface::RustHashMap<i32, interface::RustRfc<interface::RustLock<FileDescriptor>>>;

#[derive(Debug)]
pub struct Cage {
    pub cageid: u64,
    pub cwd: interface::RustLock<interface::RustRfc<interface::RustPathBuf>>,
    pub parent: u64,
    pub filedescriptortable: FdTable,
    pub getgid: interface::RustAtomicI32,
    pub getuid: interface::RustAtomicI32,
    pub getegid: interface::RustAtomicI32,
    pub geteuid: interface::RustAtomicI32
}

impl Cage {

    pub fn get_next_fd(&self, startfd: Option<i32>, fdobj: FileDescriptor) -> i32 {

        let start = match startfd {
            Some(startfd) => startfd,
            None => STARTINGFD,
        };

        // let's get the next available fd number. The standard says we need to return the lowest open fd number.
        for fd in start..MAXFD{
            match self.filedescriptortable.entry(fd) {
                interface::RustHashEntry::Occupied(_) => {}
                interface::RustHashEntry::Vacant(vacant) => {
                    vacant.insert(interface::RustRfc::new(interface::RustLock::new(fdobj)));
                    return fd;
                }
            };
        };
        return syscall_error(Errno::ENFILE, "get_next_fd", "no available file descriptor number could be found");
    }

    pub fn add_to_fd_table(&self, fd: i32, descriptor: FileDescriptor) {
        self.filedescriptortable.insert(fd, interface::RustRfc::new(interface::RustLock::new(descriptor)));
    }

    pub fn rm_from_fd_table(&self, fd: &i32) {
        self.filedescriptortable.remove(fd);
    }

    pub fn changedir(&self, newdir: interface::RustPathBuf) {
        let newwd = interface::RustRfc::new(normpath(newdir, self));
        let mut cwdbox = self.cwd.write();
        *cwdbox = newwd;
    }

    pub fn load_lower_handle_stubs(&mut self) {
        let stdin = interface::RustRfc::new(interface::RustLock::new(FileDescriptor::Stream(StreamDesc {position: 0, stream: 0, flags: O_RDONLY, advlock: interface::RustRfc::new(interface::AdvisoryLock::new())})));
        let stdout = interface::RustRfc::new(interface::RustLock::new(FileDescriptor::Stream(StreamDesc {position: 0, stream: 1, flags: O_WRONLY, advlock: interface::RustRfc::new(interface::AdvisoryLock::new())})));
        let stderr = interface::RustRfc::new(interface::RustLock::new(FileDescriptor::Stream(StreamDesc {position: 0, stream: 2, flags: O_WRONLY, advlock: interface::RustRfc::new(interface::AdvisoryLock::new())})));
        let fdtable = &self.filedescriptortable;
        fdtable.insert(0, stdin);
        fdtable.insert(1, stdout);
        fdtable.insert(2, stderr);
    }

}

pub fn insert_next_pipe(pipe: interface::EmulatedPipe) -> Option<i32> {
    for fd in STARTINGPIPE..MAXPIPE {
        if let interface::RustHashEntry::Vacant(v) = PIPE_TABLE.entry(fd) {
            v.insert(interface::RustRfc::new(pipe));
            return Some(fd);
        }
    }

    return None;
}

pub fn create_unix_sockpipes() -> (i32, i32) {

    // get next available pipe number, and set up pipe for remote
    let pipenumber1 = if let Some(pipeno) = insert_next_pipe(interface::new_pipe(UDSOCK_CAPACITY, true)) {
        pipeno
    } else { return (-1, -1); }; // return on error

    // get next available pipe number, and set up pipe for remote
    let pipenumber2 = if let Some(pipeno) = insert_next_pipe(interface::new_pipe(UDSOCK_CAPACITY, true)) {
        pipeno
    } else {
        PIPE_TABLE.remove(&pipenumber1);
        return (-1, -1);
    };

    (pipenumber1, pipenumber2)
}