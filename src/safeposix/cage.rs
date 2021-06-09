
use crate::interface;

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

}
