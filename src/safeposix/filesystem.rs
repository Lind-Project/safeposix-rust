// Filesystem metadata struct
#![allow(dead_code)]

use crate::interface;
use super::syscalls::fs_constants::*;


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
    pub fn blank_fs_init() -> FilesystemMetadata {
        //remove open files?
        let mut retval = FilesystemMetadata {nextinode: STREAMINODE + 1, dev_id: 20, inodetable: interface::RustHashMap::new(), fileobjecttable: interface::RustHashMap::new()};
        let mut dirin = DirectoryInode {size: 0, uid: DEFAULT_UID, gid: DEFAULT_GID, 
            mode: S_IFDIR | S_IRWXA, atime: DEFAULTTIME, ctime: DEFAULTTIME, mtime: DEFAULTTIME,
            linkcount: 2, refcount: 0, filename_to_inode_dict: interface::RustHashMap::new()};
        dirin.filename_to_inode_dict.insert(".".to_string(), ROOTDIRECTORYINODE);
        dirin.filename_to_inode_dict.insert("..".to_string(), ROOTDIRECTORYINODE);
        retval.inodetable.insert(ROOTDIRECTORYINODE, Inode::Dir(dirin));
        retval
    }
}

pub fn persist_metadata() {
}

pub fn restore_metadata() {
    
}
