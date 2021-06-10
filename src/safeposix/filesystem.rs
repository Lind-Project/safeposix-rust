// Filesystem metadata struct
#![allow(dead_code)]

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
    refcount: usize,
    atime: u64,
    ctime: u64,
    mtime: u64
}

struct DirectoryInode {
    size: usize,
    uid: usize,
    gid: usize,
    mode: usize,
    linkcount: usize,
    refcount: usize,
    atime: u64,
    ctime: u64,
    mtime: u64,
    filename_to_inode_dict: interface::RustHashMap<String, usize>
}


pub struct FilesystemMetadata {
    nextinode: usize,
    dev_id: usize,
    inodetable: interface::RustHashMap<usize, Inode>,
    fileobjecttable: interface::RustHashMap<usize, interface::EmulatedFile>
} 

impl FilesystemMetadata {

}

pub fn persist_metadata() {
}

pub fn restore_metadata() {
    
}
