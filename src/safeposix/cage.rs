
use crate::interface;

use syscalls::fs_constants::*;


pub static cage_table: RustLazyGlobal<RustLock<RustRfc<RustHashMap<usize, Cage>>>> = RustLazyGlobal::new(|| rust_rfc::new(rust_lock::new(new_hashmap())));


enum FileDescriptor {
    File(FileDesc),
    Stream(StreamDesc),
    Socket(SocketDesc),
    Pipe(PipeDesc)
}

struct FileDesc {
    position: usize,
    inode: usize,
    flags, usize
}

struct StreamDesc {
    position: usize,
    inode: usize,
    flags, usize
}

struct SocketDesc {
    mode: usize,
    domain: usize,
    type: usize,
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
    filedescriptortable: RustHashMap<usize, RustLock<RustRfc<FileDescriptor>>>
}

impl Cage {

    //Creates new cage - parent and old fdtable supplied on fork()
    fn new(cageid: usize, workingdir: String, parent: Option<usize>, fdtable: Option<RustHashMap<usize, RustLock<RustRfc<FileDescriptor>>>>) -> Cage {
        match parent {
            Some(parent) => parent,
            None => 0
        }

        match fdtable {
            Some(fdtable) => fdtable,
            None => new_hashmap()<usize, RustLock<RustRfc<FileDescriptor>>>
        }
        
        Cage {cageid: cageid, cwd: workingdir, parent: parent, filedescriptortable: fdtable})

    }

    fn get_next_fd(&self, startfd: Option<usize>) -> usize {

        match startfd {
            Some(startfd) => startfd,
            None => STARTINGFD
        }

        // let's get the next available fd number. The standard says we need to return the lowest open fd number.
        for fd in startfd..MAX_FD){
            if !self.filedescriptortable.contains_key(fd) {
                return fd;
            }
        }

    }

    fn add_to_fd_table(&self, fd: usize, descriptor: FileDescriptor) {
        self.filedescriptortable.insert(fd, descriptor);
    }

    fn rm_from_fd_table(&self, fd: usize) {
        self.filedescriptortable.remove(fd);        
    }


    fn changedir(&self, newdir: String) {
        self.cwd = newdir;
    }


}
 