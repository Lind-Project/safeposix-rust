// Filesystem metadata struct
#![allow(dead_code)]

use crate::interface;
use super::syscalls::fs_constants::*;
use super::cage::Cage;

pub static FS_METADATA: interface::RustLazyGlobal<interface::RustRfc<interface::RustLock<FilesystemMetadata>>> = interface::RustLazyGlobal::new(|| interface::RustRfc::new(interface::RustLock::new(FilesystemMetadata::blank_fs_init()))); //we want to check if fs exists before doing a blank init, but not for now


pub enum Inode {
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
    pub mode: u32,
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
    pub mode: u32,
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
    pub mode: u32,
    pub linkcount: usize,
    pub refcount: usize,
    pub atime: u64,
    pub ctime: u64,
    pub mtime: u64,
    pub filename_to_inode_dict: interface::RustHashMap<std::ffi::OsString, usize>
}


pub struct FilesystemMetadata {
    pub nextinode: usize,
    pub dev_id: usize,
    pub inodetable: interface::RustHashMap<usize, Inode>,
    pub fileobjecttable: interface::RustHashMap<usize, interface::EmulatedFile>
} 

impl FilesystemMetadata {
    pub fn blank_fs_init() -> FilesystemMetadata {
        //remove open files?
        let mut retval = FilesystemMetadata {nextinode: STREAMINODE + 1, dev_id: 20, inodetable: interface::RustHashMap::new(), fileobjecttable: interface::RustHashMap::new()};
        let mut dirin = DirectoryInode {size: 0, uid: DEFAULT_UID, gid: DEFAULT_GID, 
            mode: S_IFDIR as u32 | S_IRWXA, atime: DEFAULTTIME, ctime: DEFAULTTIME, mtime: DEFAULTTIME,
            linkcount: 2, refcount: 0, filename_to_inode_dict: interface::RustHashMap::new()};
        dirin.filename_to_inode_dict.insert(std::ffi::OsString::from("."), ROOTDIRECTORYINODE);
        dirin.filename_to_inode_dict.insert(std::ffi::OsString::from(".."), ROOTDIRECTORYINODE);
        retval.inodetable.insert(ROOTDIRECTORYINODE, Inode::Dir(dirin));
        retval
    }
}

pub fn persist_metadata() {
}

pub fn restore_metadata() {
    
}

pub fn convpath(cpath: String) -> interface::PathBuf {
   interface::PathBuf::from(cpath)
}
pub fn metawalk(path: interface::PathBuf) -> Option<usize> {
   let md = FS_METADATA.read().unwrap();
   let mut curnode = md.inodetable.get(&ROOTDIRECTORYINODE).unwrap();
   let mut inodeno = None;
   for comp in path.components() {
      match comp {
          interface::Component::RootDir => {},
          interface::Component::Normal(f) => {
              match curnode {
                  Inode::Dir(d) => {
                      inodeno = Some(*d.filename_to_inode_dict.get(f).unwrap());
                      curnode = md.inodetable.get(&inodeno.unwrap()).unwrap();
                  },
                  _ => {return None;}
              }
          },
          _ => {return None;}
      }
   }
   inodeno
}
pub fn normpath(origp: interface::PathBuf, cage: &Cage) -> interface::PathBuf {
    let mut newp = interface::PathBuf::new();
    if origp.is_relative() {
        newp = newp.join(cage.cwd.clone());
    }

    for comp in origp.components() {
        match comp {
            interface::Component::RootDir => {newp.push(comp);},
            interface::Component::Normal(_) => {newp.push(comp);},
            interface::Component::ParentDir => {newp.pop();},
            _ => {},
        };
    }
    newp
}
