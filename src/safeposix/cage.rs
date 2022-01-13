#![allow(dead_code)]
use crate::interface;
//going to get the datatypes and errnos from the cage file from now on
pub use crate::interface::errnos::{Errno, syscall_error};
pub use crate::interface::types::{Arg, EpollEvent, FSData, Rlimit, StatData, PipeArray, PollStruct};

pub use super::syscalls::fs_constants::*;
pub use super::syscalls::sys_constants::*;
pub use super::syscalls::net_constants::*;
use super::filesystem::normpath;

pub static CAGE_TABLE: interface::RustLazyGlobal<interface::RustLock<interface::RustHashMap<u64, interface::RustRfc<Cage>>>> = interface::RustLazyGlobal::new(|| interface::RustLock::new(interface::new_hashmap()));

pub static PIPE_TABLE: interface::RustLazyGlobal<interface::RustLock<interface::RustHashMap<i32, interface::RustRfc<interface::EmulatedPipe>>>> = 
    interface::RustLazyGlobal::new(|| 
        interface::RustLock::new(interface::new_hashmap())
);

pub static LOCK_TABLE: interface::RustLazyGlobal<interface::RustLock<interface::RustHashMap<u64, interface::RustRfc<interface::AdvisoryLock>>>> = interface::RustLazyGlobal::new(|| interface::RustLock::new(interface::new_hashmap()));


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
    pub advlock: u64
}

#[derive(Debug, Clone)]
pub struct StreamDesc {
    pub position: usize,
    pub stream: i32, //0 for stdin, 1 for stdout, 2 for stderr
    pub flags: i32,
    pub advlock: u64
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
    //pub state: ConnState,
    pub flags: i32,
    pub errno: i32,
    //pub pendingconnections: Vec<(Result<interface::Socket, i32>, interface::GenSockaddr)>,
    //pub localaddr: Option<interface::GenSockaddr>,
    //pub remoteaddr: Option<interface::GenSockaddr>,
    //pub last_peek: interface::RustDeque<u8>,
    pub socketobjectid: Option<i32>,
    pub advlock: u64
}

#[derive(Debug, Clone)]
pub struct PipeDesc {
    pub pipe: i32,
    pub flags: i32,
    pub advlock: u64
}

#[derive(Debug, Clone)]
pub struct EpollDesc {
    pub mode: i32,
    pub registered_fds: interface::RustHashMap<i32, EpollEvent>,
    pub advlock: u64,
    pub errno: i32,
    pub flags: i32
}

pub type FdTable = interface::RustHashMap<i32, interface::RustRfc<interface::RustLock<FileDescriptor>>>;

#[derive(Debug)]
pub struct Cage {
    pub cageid: u64,
    pub cwd: interface::RustLock<interface::RustRfc<interface::RustPathBuf>>,
    pub parent: u64,
    pub filedescriptortable: interface::RustLock<FdTable>
}

impl Cage {

    pub fn get_next_fd(&self, startfd: Option<i32>, fdtable_option: Option<&FdTable>) -> Option<i32> {

        let start = match startfd {
            Some(startfd) => startfd,
            None => STARTINGFD,
        };

        // let's get the next available fd number. The standard says we need to return the lowest open fd number.
        let ourreader;
        let rdguard = if let Some(fdtable) = fdtable_option {fdtable} else {
            ourreader = self.filedescriptortable.read().unwrap(); &ourreader
        };
        for fd in start..MAXFD{
            if !rdguard.contains_key(&fd) {
                return Some(fd);
            }
        };
        None
    }

    pub fn add_to_fd_table(&self, fd: i32, descriptor: FileDescriptor, fdtable_option: Option<&mut FdTable>) {
        let mut ourwriter;
        let writeguard = if let Some(fdtable) = fdtable_option {fdtable} else {
            ourwriter = self.filedescriptortable.write().unwrap();
            &mut ourwriter
        };
        writeguard.insert(fd, interface::RustRfc::new(interface::RustLock::new(descriptor)));
    }

    pub fn rm_from_fd_table(&self, fd: &i32) {
        self.filedescriptortable.write().unwrap().remove(fd);
    }

    pub fn changedir(&self, newdir: interface::RustPathBuf) {
        let newwd = interface::RustRfc::new(normpath(newdir, self));
        let mut cwdbox = self.cwd.write().unwrap();
        *cwdbox = newwd;
    }

    pub fn load_lower_handle_stubs(&mut self) {
        let stdin = interface::RustRfc::new(interface::RustLock::new(FileDescriptor::Stream(StreamDesc {position: 0, stream: 0, flags: O_RDONLY, advlock:  add_advlock().unwrap()})));
        let stdout = interface::RustRfc::new(interface::RustLock::new(FileDescriptor::Stream(StreamDesc {position: 0, stream: 1, flags: O_WRONLY, advlock: add_advlock().unwrap()})));
        let stderr = interface::RustRfc::new(interface::RustLock::new(FileDescriptor::Stream(StreamDesc {position: 0, stream: 2, flags: O_WRONLY, advlock: add_advlock().unwrap()})));
        let mut fdtable = self.filedescriptortable.write().unwrap();
        fdtable.insert(0, stdin);
        fdtable.insert(1, stdout);
        fdtable.insert(2, stderr);
    }

}

pub fn get_next_pipe() -> Option<i32> {
    let table = PIPE_TABLE.read().unwrap();
    for fd in STARTINGPIPE..MAXPIPE {
        if !table.contains_key(&fd) {
            return Some(fd);
        }
    }

    return None;
}

pub fn add_advlock() -> Option<u64> {
    let mut table = LOCK_TABLE.write().unwrap();
    for fd in 0..10000 {
        if !table.contains_key(&fd) {
            table.insert(fd, interface::RustRfc::new(interface::AdvisoryLock::new()));
            return Some(fd);
        }
    }

    return None;
}

