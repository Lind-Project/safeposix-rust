// File system related system calls

use crate::interface;

use super::fs_constants::*;
use crate::safeposix::cage::{CAGE_TABLE, Cage, FileDescriptor::*, FileDesc};
use crate::safeposix::filesystem::*;

impl Cage {
    pub fn open_syscall(&self, path: &str, flags: i32, mode: u32) -> i32 {
        //Check that path is not empty
        if path.len() != 0 {return -1;}//ENOENT later

        let truepath = normpath(convpath(path), self);

        //file descriptor table write lock held for the whole function to prevent TOCTTOU
        let mut fdt = self.filedescriptortable.write().unwrap();
        //file system metadata table write lock held for the whole function to prevent TOCTTOU
        let mut fmd = FS_METADATA.write().unwrap();

        match metawalkandparent(truepath.as_path(), Some(&fmd)) {
            //If neither the file nor parent exists
            (None, None) => {
                if 0 != (flags & O_CREAT) {
                    return -1; //ENOENT later
                }
                return -1; //ENOTDIR later
            }

            //If the file doesn't exist but the parent does
            (None, Some(pardirinode)) => {
                if 0 != (flags & O_CREAT) {
                    return -1; //ENOENT later
                }

                let filename = truepath.file_name(); //for now we assume this is sane, but maybe this should be checked later

                if 0 != (S_IFCHR & flags) {
                    return -1; //you shouldn't be able to create a character file except by mknod
                } 

                let effective_mode = S_IFREG as u32 | mode;

                assert_eq!(mode & (S_IRWXA | S_FILETYPEFLAGS as u32), mode); //assert sane mode bits

                let time = interface::timestamp(); //We do a real timestamp now
                let newinode = Inode::File(GenericInode {
                    size: 0, uid: DEFAULT_UID, gid: DEFAULT_GID,
                    mode: effective_mode, linkcount: 1, refcount: 0,
                    atime: time, ctime: time, mtime: time,
                });

                let newinode = fmd.nextinode;
                fmd.nextinode += 1;
                if let Inode::Dir(ind) = fmd.inodetable.get_mut(&pardirinode).unwrap() {
                    ind.filename_to_inode_dict.insert(filename.unwrap().to_owned(), newinode);
                } //insert a reference to the file in the parent directory
                //persist metadata?
            }

            //If the file exists (we don't need to look at parent here)
            (Some(inodeno), ..) => {
                if (O_CREAT | O_EXCL) == (flags & (O_CREAT | O_EXCL)) {
                    return -1; //EEXIST later
                }

                if 0 != (flags & O_TRUNC) {
                    //close the file object if another cage has it open
                    if fmd.fileobjecttable.contains_key(&inodeno) {
                        fmd.fileobjecttable.get(&inodeno).unwrap().close().unwrap();
                    }

                    //set size of file to 0
                    match fmd.inodetable.get_mut(&inodeno).unwrap() {
                        Inode::File(g) => {g.size = 0;}
                        _ => {return -1;}
                    }

                    //remove the previous file and add a new one of 0 length
                    let sysfilename = format!("{}{}", FILEDATAPREFIX, inodeno);
                    interface::removefile(sysfilename.clone()).unwrap();
                    fmd.fileobjecttable.insert(inodeno, interface::emulated_open(sysfilename, true).unwrap());
                }
            }
        }

        //We redo our metawalk in case of O_CREAT, but this is somewhat inefficient
        if let Some(inodeno) = metawalk(truepath.as_path(), Some(&fmd)) {
            let mut inodeobj = fmd.inodetable.get_mut(&inodeno).unwrap();
            let mode;
            let size;

            //increment number of open handles to the file, retrieve other data from inode
            match inodeobj {
                Inode::File(f) => {size = f.size; mode = f.mode; f.refcount += 1},
                Inode::Dir(f) => {size = f.size; mode = f.mode; f.refcount += 1},
                _ => {panic!("How did you even manage to open another kind of file like that?");},
            }

            let thisfd = match self.get_next_fd(None) {
                Some(j) => j,
                None => {return -1;} //some error later
            };

            //If the file is a regular file, open the file object
            if is_reg(mode) {
                if fmd.fileobjecttable.contains_key(&inodeno) {
                    let sysfilename = format!("{}{}", FILEDATAPREFIX, inodeno);
                    fmd.fileobjecttable.insert(inodeno, interface::emulated_open(sysfilename, false).unwrap());
                }
            }

            //insert file descriptor into fdtable of the cage
            let position = if 0 != flags & O_APPEND {size} else {0};
            let newfd = File(FileDesc {position: position, inode: inodeno, flags: flags & O_RDWRFLAGS});
            let wrappedfd = interface::RustRfc::new(interface::RustLock::new(interface::RustRfc::new(newfd)));
            fdt.insert(thisfd, wrappedfd);
        } else {panic!("Inode not created for some reason");}
        0
    }

    // ADD FSTAT AS WELL

    //------------------STAT SYSCALL------------------

    pub fn stat_syscall(&self, path: std::ffi::CString, ret : &mut StatData) -> &mut StatData {        
        //need to get datalock somehow
        let truepath = normpath(convpath(path.into_string().unwrap()), self);

        if let Some(inodeno) = metawalk(truepath) {
            let mut mdobj = FS_METADATA.write().unwrap();
            let mut inodeobj = mdobj.inodetable.get_mut(&inodeno).unwrap();
            let mode;
            // let linkcount;
            // let refcount;
            let size;
            // let uid;
            // let gid;
            // let size;
            // let atime;
            // let mtime;
            // let ctime;
            
            match inodeobj {
                Inode::File(f) => {size = f.size; mode = f.mode; f.refcount += 1},
                Inode::CharDev(f) => {size = f.size; mode = f.mode; f.refcount += 1},
                Inode::Dir(f) => {size = f.size; mode = f.mode; f.refcount += 1},
                Inode::Stream(f) => {size = f.size; mode = f.mode; f.refcount += 1},
                Inode::Pipe(f) => {panic!("How did you even manage to open a pipe like that?");},
                Inode::Socket(f) => {size = f.size; mode = f.mode; f.refcount += 1},
            }

            if is_chr(mode) {
                Self::_istat_helper_chr_file(inodeobj, ret, inodeno);
            }

            Self::_istat_helper(inodeobj, ret, inodeno);
        }
        return ret;

    }

    pub fn _istat_helper(inodeobj: GenericInode, ret: &mut StatData, inodeno: usize) { 
        // ret.dev_id = inodeobj.dev_id;
        // ret.inode = inodeno;
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

    pub fn _istat_helper_chr_file(inodeobj: GenericInode, ret: &mut StatData, inodeno: usize) {   //please check this and the other file's Inode type implementations
        // ret.dev_id = 5;     //it's always 5
        // ret.inode = inodeno;
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

            match inodeobj {
                Inode::File(f) => {mode = f.mode; f.refcount += 1},
                Inode::CharDev(f) => {mode = f.mode; f.refcount += 1},
                Inode::Dir(f) => {mode = f.mode; f.refcount += 1},
                Inode::Stream(f) => {mode = f.mode; f.refcount += 1},
                Inode::Pipe(f) => {panic!("How did you even manage to open a pipe like that?");},
                Inode::Socket(f) => {mode = f.mode; f.refcount += 1},
            }

            let newmode: u32 = 0;
            if amode & X_OK == X_OK {newmode |= S_IXUSR; }
            if amode & W_OK == W_OK {newmode |= S_IWUSR; }
            if amode & R_OK == R_OK {newmode |= S_IRUSR; }

            if mode & newmode == newmode {return 0;}

        }
        return -1; //returns -1 if requested access is denied
    }
}
