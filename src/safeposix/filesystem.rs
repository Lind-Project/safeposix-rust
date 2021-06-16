// Filesystem metadata struct
#![allow(dead_code)]

use crate::interface;
use super::syscalls::fs_constants::*;
use super::cage::Cage;


enum Inode {
    File(GenericInode),
    CharDev(DeviceInode),
    Dir(DirectoryInode),
    Stream(GenericInode),
    Pipe(GenericInode),
    Socket(GenericInode)
}

pub struct GenericInode {
    pub size: usize,
    pub uid: usize,
    pub gid: usize,
    pub mode: usize,
    pub linkcount: usize,
    pub refcount: usize,
    pub atime: u64,
    pub ctime: u64,
    pub mtime: u64
}
pub struct DeviceInode {
    pub size: usize,
    pub uid: usize,
    pub gid: usize,
    pub mode: usize,
    pub linkcount: usize,
    pub refcount: usize,
    pub atime: u64,
    pub ctime: u64,
    pub mtime: u64,
    pub dev: DevNo,
}

pub struct DirectoryInode {
    pub size: usize,
    pub uid: usize,
    pub gid: usize,
    pub mode: usize,
    pub linkcount: usize,
    pub refcount: usize,
    pub atime: u64,
    pub ctime: u64,
    pub mtime: u64,
    pub filename_to_inode_dict: interface::RustHashMap<String, usize>
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

pub fn normpath(origp: interface::PathBuf, cage: Cage) -> interface::PathBuf {
    let mut newp = interface::PathBuf::new();
    if origp.is_relative() {
        newp.push(cage.cwd);
    }

    for comp in origp.components() {
        match comp {
            interface::Component::RootDir => {newp.push(comp);},
            interface::Component::Normal(_) => {newp.push(comp);},
            interface::Component::ParentDir => {if newp.parent().is_some() {newp.pop();};}
            _ => {},
        };
    }
    newp
}
