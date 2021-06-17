#![allow(dead_code)]
use crate::interface;

use super::syscalls::fs_constants::*;
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
    pub flags: i32
}

#[derive(Debug)]
pub struct StreamDesc {
    pub position: usize,
    pub inode: usize,
    pub flags: i32
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
    pub errno: usize
}

#[derive(Debug)]
pub struct PipeDesc {
    pub pipe: usize,
    pub flags: i32
}

#[derive(Debug)]
pub struct Cage {
    pub cageid: u64,
    pub cwd: interface::PathBuf,
    pub parent: u64,
    pub filedescriptortable: interface::RustLock<interface::RustHashMap<i32, interface::RustRfc<interface::RustLock<interface::RustRfc<FileDescriptor>>>>>
}

impl Cage {

    pub fn get_next_fd(&self, startfd: Option<i32>) -> Option<i32> {

        let start = match startfd {
            Some(startfd) => startfd,
            None => STARTINGFD,
        };

        // let's get the next available fd number. The standard says we need to return the lowest open fd number.
        let rdguard = self.filedescriptortable.read().unwrap();
        for fd in start..MAXFD{
            if !rdguard.contains_key(&fd) {
                return Some(fd);
            }
        };
        None
    }

    pub fn add_to_fd_table(&mut self, fd: i32, descriptor: FileDescriptor) {
        self.filedescriptortable.write().unwrap().insert(fd, interface::RustRfc::new(interface::RustLock::new(interface::RustRfc::new(descriptor))));
    }

    pub fn rm_from_fd_table(&mut self, fd: &i32) {
        self.filedescriptortable.write().unwrap().remove(fd);
    }

    pub fn changedir(&mut self, newdir: interface::PathBuf) {
        self.cwd = normpath(self.cwd.join(newdir), self);
    }

    pub fn load_lower_handle_stubs(&mut self) {
        let stdin = interface::RustRfc::new(interface::RustLock::new(interface::RustRfc::new(FileDescriptor::Stream(StreamDesc {position: 0, inode: STREAMINODE, flags: O_RDONLY}))));
        let stdout = interface::RustRfc::new(interface::RustLock::new(interface::RustRfc::new(FileDescriptor::Stream(StreamDesc {position: 0, inode: STREAMINODE, flags: O_WRONLY}))));
        let stderr = interface::RustRfc::new(interface::RustLock::new(interface::RustRfc::new(FileDescriptor::Stream(StreamDesc {position: 0, inode: STREAMINODE, flags: O_WRONLY}))));
        let mut fdtable = self.filedescriptortable.write().unwrap();
        fdtable.insert(0, stdin);
        fdtable.insert(1, stdout);
        fdtable.insert(2, stderr);
    }
}
