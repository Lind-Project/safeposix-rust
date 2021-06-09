// Filesystem metadata struct

use crate::interface;


enum Inode {
    File(GenericInode),
    Dir(DirectoryInode),
    Stream(GenericInode),
    Pipe(GenericInode),
    Socket(GenericInode)
}

struct GenericInode {
    size: usize,
    uid: usize,
    gid: usize,
    mode: usize,
    linkcount: usize,
    atime: u64,
    ctime: u64,
    mtime: u64,
    refcount: usize
}

struct DirInode {
    size: usize,
    uid: usize,
    gid: usize,
    mode: usize,
    linkcount: usize,
    atime: u64,
    ctime: u64,
    mtime: u64
    filename_to_inode_dict: RustHashMap<String, usize>,
    refcount: usize
}


pub struct FilesystemMetadata {
    nextinode: usize,
    dev_id: usize,
    inodetable: RustHashMap<usize, Inode>
    fileobjecttable: RustHashMap<usize, EmulatedFile>
} 

impl FilesystemMetadata {

}
