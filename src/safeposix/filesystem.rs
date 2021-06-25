// Filesystem metadata struct
#![allow(dead_code)]

use crate::interface;
use super::syscalls::fs_constants::*;
use super::cage::Cage;

const METADATAFILENAME: &str = "lind.metadata";

pub static FS_METADATA: interface::RustLazyGlobal<interface::RustRfc<interface::RustLock<FilesystemMetadata>>> = 
    interface::RustLazyGlobal::new(||
        interface::RustRfc::new(interface::RustLock::new(FilesystemMetadata::blank_fs_init()))
    ); //we want to check if fs exists before doing a blank init, but not for now

#[derive(interface::RustSerialize, interface::RustDeserialize, Debug)]
pub enum Inode {
    File(GenericInode),
    CharDev(DeviceInode),
    Dir(DirectoryInode),
    //Stream(GenericInode), streams don't have a real inode
    Pipe(GenericInode),
    Socket(GenericInode)
}

#[derive(interface::RustSerialize, interface::RustDeserialize, Debug)]
pub struct GenericInode {
    pub size: usize,
    pub uid: u32,
    pub gid: u32,
    pub mode: u32,
    pub linkcount: u32,
    pub refcount: u32,
    pub atime: u64,
    pub ctime: u64,
    pub mtime: u64
}
#[derive(interface::RustSerialize, interface::RustDeserialize, Debug)]
pub struct DeviceInode {
    pub size: usize,
    pub uid: u32,
    pub gid: u32,
    pub mode: u32,
    pub linkcount: u32,
    pub refcount: u32,
    pub atime: u64,
    pub ctime: u64,
    pub mtime: u64,
    pub dev: DevNo,
}

#[derive(interface::RustSerialize, interface::RustDeserialize, Debug)]
pub struct DirectoryInode {
    pub size: usize,
    pub uid: u32,
    pub gid: u32,
    pub mode: u32,
    pub linkcount: u32,
    pub refcount: u32,
    pub atime: u64,
    pub ctime: u64,
    pub mtime: u64,
    pub filename_to_inode_dict: interface::RustHashMap<std::ffi::OsString, usize>
}

#[derive(interface::RustSerialize, interface::RustDeserialize, Debug)]
pub struct FilesystemMetadata {
    pub nextinode: usize,
    pub dev_id: u64,
    pub inodetable: interface::RustHashMap<usize, Inode>,
    pub fileobjecttable: interface::RustHashMap<usize, interface::EmulatedFile>
} 

impl FilesystemMetadata {
    pub fn blank_fs_init() -> FilesystemMetadata {
        //remove open files?
        let mut retval = FilesystemMetadata {nextinode: STREAMINODE + 1, dev_id: 20, inodetable: interface::RustHashMap::new(), fileobjecttable: interface::RustHashMap::new()};
        let time = interface::timestamp(); //We do a real timestamp now
        let mut dirin = DirectoryInode {size: 0, uid: DEFAULT_UID, gid: DEFAULT_GID, 
            mode: S_IFDIR as u32 | S_IRWXA, atime: time, ctime: time, mtime: time,
            linkcount: 3, refcount: 0, filename_to_inode_dict: interface::RustHashMap::new()};//this is where cwd starts
        dirin.filename_to_inode_dict.insert(std::ffi::OsString::from("."), ROOTDIRECTORYINODE);
        dirin.filename_to_inode_dict.insert(std::ffi::OsString::from(".."), ROOTDIRECTORYINODE);
        retval.inodetable.insert(ROOTDIRECTORYINODE, Inode::Dir(dirin));
        retval
    }
}

// Serialize Metadata Struct to JSON, write to file
pub fn persist_metadata() {

    // Serialize metadata to string
    let metadata = FS_METADATA.read().unwrap();
    let metadatastring = interface::rust_serialize_to_string(&metadata).unwrap();
    
    // remove file if it exists
    interface::removefile(METADATAFILENAME.to_string()).unwrap(); 

    // write to file
    let metadatafo = interface::openfile(METADATAFILENAME.to_string(), true).unwrap();
    metadatafo.write_from_string(metadatastring, 0);
    metadatafo.close();
}

// Read file, and deserialize json to FS METADATA
pub fn restore_metadata() {

    // Read JSON from file
    let metadatafo = interface::openfile(METADATAFILENAME.to_string(), true).unwrap();
    let metadatastring = metadatafo.read_to_new_string(0).unwrap();
    metadatafo.close();

    // Restore metadata
    let metadata = FS_METADATA.read().unwrap();
    *metadata =  interface::rust_deserialize_from_string(&metadatastring).unwrap();
}

pub fn convpath(cpath: &str) -> interface::RustPathBuf {
    interface::RustPathBuf::from(cpath)
}

//returns tuple consisting of inode number of file (if it exists), and inode number of parent (if it exists)
pub fn metawalkandparent(path: &interface::RustPath, guard: Option<&FilesystemMetadata>) -> (Option<usize>, Option<usize>) {
    let ourreader;
    //Acquire a readlock if we were not passed in a reference
    let md = if let Some(rl) = guard {rl} else {
        ourreader = FS_METADATA.read().unwrap(); 
        &ourreader
    };

    let mut curnode = Some(md.inodetable.get(&ROOTDIRECTORYINODE).unwrap());
    let mut inodeno = Some(ROOTDIRECTORYINODE);
    let mut previnodeno = None;

    //Iterate over the components of the pathbuf in order to walk the file tree
    for comp in path.components() {
        match comp {
            //We've already done what initialization needs to be done
            interface::RustPathComponent::RootDir => {},

            interface::RustPathComponent::Normal(f) => {
                //If we're trying to get the child of a nonexistent directory, exit out
                if inodeno.is_none() {return (None, None);}
                match curnode {
                    Some(Inode::Dir(d)) => {
                        previnodeno = inodeno;

                        //populate child inode number from parent directory's inode dict
                        inodeno = match d.filename_to_inode_dict.get(f) {
                            Some(num) => {
                                curnode = md.inodetable.get(&num);
                                Some(*num)
                            }

                            //if no such child exists, update curnode, inodeno accordingly so that
                            //we can check against none as we do at the beginning of the Normal match arm
                            None => {
                                curnode = None;
                                None
                            }
                        }
                    }
                    //if we're trying to get a child of a non-directory inode, exit out
                    _ => {return (None, None);}
                }
            },

            //If it's a component of the pathbuf that we don't expect given a normed path, exit out
            _ => {return (None, None);}
        }
    }
    //return inode number and it's parent's number
    (inodeno, previnodeno)
}
pub fn metawalk(path: &interface::RustPath, guard: Option<&FilesystemMetadata>) -> Option<usize> {
    metawalkandparent(path, guard).0
}
pub fn normpath(origp: interface::RustPathBuf, cage: &Cage) -> interface::RustPathBuf {
    //If path is relative, prefix it with the current working directory, otherwise populate it with rootdir
    let mut newp = if origp.is_relative() {(**cage.cwd.read().unwrap()).clone()} else {interface::RustPathBuf::from("/")};

    for comp in origp.components() {
        match comp {
            //if we have a normal path component, push it on to our normed path
            interface::RustPathComponent::Normal(_) => {newp.push(comp);},

            //if we have a .. path component, pop the last component off our normed path
            interface::RustPathComponent::ParentDir => {newp.pop();},

            //if we have a . path component (Or a root dir or a prefix(?)) do nothing
            _ => {},
        };
    }
    newp
}
