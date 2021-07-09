#![allow(dead_code)]
use crate::interface;

pub use super::syscalls::fs_constants::*;
use super::filesystem::normpath;

pub static CAGE_TABLE: interface::RustLazyGlobal<interface::RustLock<interface::RustHashMap<u64, interface::RustRfc<Cage>>>> = interface::RustLazyGlobal::new(|| interface::RustLock::new(interface::new_hashmap()));


#[derive(Debug)]
pub enum FileDescriptor {
    File(FileDesc),
    Stream(StreamDesc),
    Socket(SocketDesc),
    Pipe(PipeDesc)
}

#[derive(Debug)]
pub struct FileDesc {
    pub position: usize,
    pub inode: usize,
    pub flags: i32,
    // pub access_lock: interface::RustLock<()>,
    // pub readLock: Vec<interface::RustLockReadGuard<()>>,
    // pub writeLock: Option<interface::RustLockWriteGuard<()>>
}

#[derive(Debug)]
pub struct StreamDesc {
    pub position: usize,
    pub stream: i32, //0 for stdin, 1 for stdout, 2 for stderr
    pub flags: i32,
    // pub access_lock: interface::RustLock<()>,
    // pub readLock: Vec<interface::RustLockReadGuard<'static, ()>>,
    // pub writeLock: Option<interface::RustLockWriteGuard<'static, ()>>
}

#[derive(Debug)]
pub struct SocketDesc {
    pub mode: u32,
    pub domain: usize,
    pub socktype: usize,
    pub protocol: usize,
    pub options: usize,
    pub sndbuf: usize,
    pub rcvbuf: usize,
    pub state: usize,
    pub flags: i32,
    pub errno: usize,
    // pub access_lock: interface::RustLock<()>,
    // pub readLock: Vec<interface::RustLockReadGuard<'static, ()>>,
    // pub writeLock: Option<interface::RustLockWriteGuard<'static, ()>>
}

#[derive(Debug)]
pub struct PipeDesc {
    pub pipe: usize,
    pub flags: i32,
    // pub access_lock: interface::RustLock<()>,
    // pub readLock: Vec<interface::RustLockReadGuard<'static, ()>>,
    // pub writeLock: Option<interface::RustLockWriteGuard<'static, ()>>
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
        let stdin = interface::RustRfc::new(interface::RustLock::new(FileDescriptor::Stream(StreamDesc {position: 0, stream: 0, flags: O_RDONLY})));
        let stdout = interface::RustRfc::new(interface::RustLock::new(FileDescriptor::Stream(StreamDesc {position: 0, stream: 1, flags: O_WRONLY})));
        let stderr = interface::RustRfc::new(interface::RustLock::new(FileDescriptor::Stream(StreamDesc {position: 0, stream: 2, flags: O_WRONLY})));
        let mut fdtable = self.filedescriptortable.write().unwrap();
        fdtable.insert(0, stdin);
        fdtable.insert(1, stdout);
        fdtable.insert(2, stderr);
    }

}
