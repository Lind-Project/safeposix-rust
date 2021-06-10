#![allow(dead_code)]
use crate::interface;

use crate::safeposix::syscalls::fs_constants::*;

pub static CAGE_TABLE: interface::RustLazyGlobal<interface::RustLock<interface::RustRfc<interface::RustHashMap<u64, Cage>>>> = interface::RustLazyGlobal::new(|| interface::RustLock::new(interface::RustRfc::new(interface::new_hashmap())));


enum FileDescriptor {
    File(FileDesc),
    Stream(StreamDesc),
    Socket(SocketDesc),
    Pipe(PipeDesc)
}

struct FileDesc {
    position: usize,
    inode: usize,
    flags: usize
}

struct StreamDesc {
    position: usize,
    inode: usize,
    flags: usize
}

struct SocketDesc {
    mode: usize,
    domain: usize,
    socktype: usize,
    protocol: usize,
    options: usize,
    sndbuf: usize,
    rcvbuf: usize,
    state: usize,
    flags: usize,
    errno: usize
}

struct PipeDesc {
    pipe: usize,
    flags: usize
}

pub struct Cage {
    cageid: usize,
    cwd: String,
    parent: usize,
    filedescriptortable: interface::RustHashMap<usize, interface::RustLock<interface::RustRfc<FileDescriptor>>>
}

impl Cage {

    //Creates new cage - parent and old fdtable supplied on fork()
    fn new(cageid: usize, workingdir: String, parent: usize, fdtable: Option<interface::RustHashMap<usize, interface::RustLock<interface::RustRfc<FileDescriptor>>>>) -> Cage {

        let fdt2put = match fdtable {
            Some(fdtable) => fdtable,
            None => interface::new_hashmap()
        };
        
        Cage {cageid: cageid, cwd: workingdir, parent: parent, filedescriptortable: fdt2put}

    }

    fn get_next_fd(&self, startfd: Option<usize>) -> Option<usize> {

        let start = match startfd {
            Some(startfd) => startfd,
            None => STARTINGFD,
        };

        // let's get the next available fd number. The standard says we need to return the lowest open fd number.
        for fd in start..MAXFD{
            if !self.filedescriptortable.contains_key(&fd) {
                return Some(fd);
            }
        };
        None
    }

    fn add_to_fd_table(&mut self, fd: usize, descriptor: FileDescriptor) {
        self.filedescriptortable.insert(fd, interface::RustLock::new(interface::RustRfc::new(descriptor)));
    }

    fn rm_from_fd_table(&mut self, fd: &usize) {
        self.filedescriptortable.remove(fd);
    }

    fn changedir(&mut self, newdir: String) {
        self.cwd = newdir;
    }

}
