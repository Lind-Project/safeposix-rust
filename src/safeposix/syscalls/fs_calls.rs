// File system related system calls

use crate::interface;

use super::fs_constants::*;
use crate::safeposix::cage::{CAGE_TABLE, Cage, FileDescriptor::*, FileDesc};
use crate::safeposix::filesystem::*;

impl Cage {
    pub fn open_syscall(&self, path: std::ffi::CString, flags: i32, mode: u32) -> i32 {
        if path == std::ffi::CString::default() {return -1;}//ENOENT later
        //here we assume correct cstring, probably erroneous
        let truepath = normpath(convpath(path.into_string().unwrap()), self);

        //currently TOCTTOU vulnerable, it will be fixed later
        match metawalk(truepath.clone()) {
            None => {
                if 0 != (flags & O_CREAT) {
                    return -1; //ENOENT later
                }
                let pardirinode = match metawalk(match truepath.clone().parent(){Some(p) => {p.to_path_buf()}, None => {return -1;}}) {
                    Some(inodeno) => inodeno,
                    None => {return -1;} //ENOTDIR later
                };
                let filename = truepath.file_name(); //for now we assume this is sane, but we should fix later
                if 0 != (S_IFCHR & flags) {
                    return -1;
                    //you shouldn't be able to create a character file except by mknod
                } 
                let effective_mode = S_IFREG as u32 | mode;
                //assert sane mode bits?
                let newinode = Inode::File(GenericInode {
                    size: 0, uid: DEFAULT_UID, gid: DEFAULT_GID,
                    mode: effective_mode, linkcount: 1, refcount: 0,
                    atime: DEFAULTTIME, ctime: DEFAULTTIME, mtime: DEFAULTTIME,
                });
                let mut fmd = FS_METADATA.write().unwrap();
                let newinode = fmd.nextinode;
                fmd.nextinode += 1;
                if let Inode::Dir(ind) = fmd.inodetable.get_mut(&pardirinode).unwrap() {
                    ind.filename_to_inode_dict.insert(filename.unwrap().to_owned(), newinode);
                }
                //persist metadata?
            },
            Some(inodeno) => {
                if (O_CREAT | O_EXCL) == (flags & (O_CREAT | O_EXCL)) {
                    return -1; //EEXIST later
                }
                if 0 != (flags & O_TRUNC) {
                    let mut fmd = FS_METADATA.write().unwrap();
                    if fmd.fileobjecttable.contains_key(&inodeno) {
                        fmd.fileobjecttable.get(&inodeno).unwrap().close().unwrap();
                    }
                    match fmd.inodetable.get_mut(&inodeno).unwrap() {
                        Inode::File(g) => {g.size = 0;}
                        _ => {return -1;}
                    }
                    let sysfilename = format!("{}{}", FILEDATAPREFIX, inodeno);
                    interface::removefile(sysfilename.clone()).unwrap();
                    fmd.fileobjecttable.insert(inodeno, interface::emulated_open(sysfilename, true).unwrap());
                }
            },
        }
        if let Some(inodeno) = metawalk(truepath) {
            let mut mdobj = FS_METADATA.write().unwrap();
            let mut inodeobj = mdobj.inodetable.get_mut(&inodeno).unwrap();
            let mode;
            let size;
            match inodeobj {
                Inode::File(f) => {size = f.size; mode = f.mode; f.refcount += 1},
                Inode::CharDev(f) => {size = f.size; mode = f.mode; f.refcount += 1},
                Inode::Dir(f) => {size = f.size; mode = f.mode; f.refcount += 1},
                Inode::Stream(f) => {size = f.size; mode = f.mode; f.refcount += 1},
                Inode::Pipe(f) => {panic!("How did you even manage to open a pipe like that?");},
                Inode::Socket(f) => {size = f.size; mode = f.mode; f.refcount += 1},
            }
            let mut fdt = self.filedescriptortable.write().unwrap();
            let thisfd = match self.get_next_fd(None) {
                Some(j) => j,
                None => {return -1;} //some error later
            };
            if is_reg(mode) {
                if mdobj.fileobjecttable.contains_key(&inodeno) {
                    let sysfilename = format!("{}{}", FILEDATAPREFIX, inodeno);
                    mdobj.fileobjecttable.insert(inodeno, interface::emulated_open(sysfilename, false).unwrap());
                }
            }
            let position = if 0 != flags & O_APPEND {size} else {0};
            fdt.insert(thisfd, interface::RustRfc::new(interface::RustLock::new(interface::RustRfc::new(File(FileDesc {position: position, inode: inodeno, flags: flags & O_RDWRFLAGS})))));
        } else {panic!("Inode not created for some reason");}
        0
    }

    // ADD FSTAT AS WELL

    //------------------STAT SYSCALL------------------

    pub fn stat_syscall(&self, path: std::ffi::CString) -> &mut statdata {        
        //need to get datalock somehow

        let truepath = normpath(convpath(path.into_string().unwrap()), self);

        if let Some(inodeno) = metawalk(truepath) {
            let mut mdobj = FS_METADATA.write().unwrap();
            let mut inodeobj = mdobj.inodetable.get_mut(&inodeno).unwrap();
            let mode;
            let size;
            match inodeobj {
                Inode::File(f) => {size = f.size; mode = f.mode; f.refcount += 1},
                Inode::CharDev(f) => {size = f.size; mode = f.mode; f.refcount += 1},
                Inode::Dir(f) => {size = f.size; mode = f.mode; f.refcount += 1},
                Inode::Stream(f) => {size = f.size; mode = f.mode; f.refcount += 1},
                Inode::Pipe(f) => {panic!("How did you even manage to open a pipe like that?");},
                Inode::Socket(f) => {size = f.size; mode = f.mode; f.refcount += 1},
                _ => {return -1;},
            }

            let ret : &mut statdata;

            if (is_chr(mode)) {
                return _istat_helper_chr_file(inodeobj, ret);
            }

            return _istat_helper(inodeobj, ret);
        }

    }

    //not sure how you want "FS_CALL_DICTIONARY["stat_syscall"] = stat_syscall" implemented
    //"FS_CALL_DICTIONARY["fstat_syscall"] = fstat_syscall," too

    pub fn _istat_helper(inodeobj: GenericInode, ret: &mut statdata) { 
        // ret.dev_id = inodeobj.dev_id;
        // ret.inode = inodeobj;
        ret.mode = inodeobj.mode;
        ret.linkcount = inodeobj.linkcount;
        ret.refcount = inodeobj.refcount;
        ret.uid = inodeobj.uid;
        ret.gid = inodeobj.gid;
        // ret.dev = 0;
        ret.size = inodeobj.size;
        // ret.blksize = 0;
        // ret.blocks = 0;
        ret.atime = inodeobj.atime;
        // ret.atimens = 0;
        ret.mtime = inodeobj.mtime;
        // ret.mtimens = 0;
        ret.ctime = inodeobj.ctime;
        // ret.ctimens = 0;
    }

    pub fn _istat_helper_chr_file(inodeobj: GenericInode, ret: &mut statdata) {   //please check this and the other file's Inode type implementations
        // ret.dev_id = 5;     //it's always 5
        // ret.inode = inodeobj;
        ret.mode = inodeobj.mode;
        ret.linkcount = inodeobj.linkcount;
        ret.refcount = inodeobj.refcount;
        ret.uid = inodeobj.uid;
        ret.gid = inodeobj.gid;
        // ret.dev = inodeobj.dev;
        ret.size = inodeobj.size;
        // ret.blksize = 0;
        // ret.blocks = 0;
        ret.atime = inodeobj.atime;
        // ret.atimens = 0;
        ret.mtime = inodeobj.mtime;
        // ret.mtimens = 0;
        ret.ctime = inodeobj.ctime;
        // ret.ctimens = 0;
    }

    //------------------ACCESS SYSCALL------------------

    fn access_syscall(&self, path: std::ffi::CString, amode: u32) -> i32 {
        //somehow get data lock
        let truepath = normpath(convpath(path.into_string().unwrap()), self);
        if let Some(inodeno) = metawalk(truepath) {
            let mut mdobj = FS_METADATA.write().unwrap();
            let mut inodeobj = mdobj.inodetable.get_mut(&inodeno).unwrap();
            let mode;

            let newmode: u32 = 0;
            if amode & X_OK {newmode |= S_IXUSR; }
            if amode & W_OK {newmode |= S_IWUSR; }
            if amode & R_OK {newmode |= S_IRUSR; }

            if inodeobj.mode & newmode == newmode {return 0;}

            return -1; //returns -1 if requested access is denied
        }
    }
}
