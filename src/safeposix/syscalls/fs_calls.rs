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

                let newinodeno = fmd.nextinode;
                fmd.nextinode += 1;
                if let Inode::Dir(ind) = fmd.inodetable.get_mut(&pardirinode).unwrap() {
                    ind.filename_to_inode_dict.insert(filename.unwrap().to_owned(), newinodeno);
                } //insert a reference to the file in the parent directory
                fmd.inodetable.insert(newinodeno, newinode).unwrap();
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
            let inodeobj = fmd.inodetable.get_mut(&inodeno).unwrap();
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

    pub fn stat_syscall(&self, path: &str, ret : &mut StatData) -> i32 {
        let truepath = normpath(convpath(path), self);
        let mdobj = FS_METADATA.read().unwrap();

        if let Some(inodeno) = metawalk(truepath.as_path(), Some(&mdobj)) {
            let inodeobj = mdobj.inodetable.get(&inodeno).unwrap();
            
            match inodeobj {
                Inode::File(f) => {
                    Self::_istat_helper(f, ret);
                },
                Inode::CharDev(f) => {
                    Self::_istat_helper_chr_file(f, ret);
                },
                Inode::Dir(f) => {
                    Self::_istat_helper_dir(f, ret);
                },
                Inode::Pipe(_) => {
                    panic!("How did you even manage to refer to a pipe like that?");
                },
                Inode::Socket(_) => {
                    panic!("How did you even manage to refer to a socket like that?");
                },
            }
            ret.st_dev = mdobj.dev_id;
            ret.st_ino = inodeno;
            0
        } else {
            -1
        }

    }

    pub fn _istat_helper(inodeobj: &GenericInode, ret: &mut StatData) {
        ret.st_mode = inodeobj.mode;
        ret.st_nlink = inodeobj.linkcount;
        ret.st_uid = inodeobj.uid;
        ret.st_gid = inodeobj.gid;
        ret.st_rdev = 0;
        ret.st_size = inodeobj.size;
        ret.st_blksize = 0;
        ret.st_blocks = 0;
    }

    pub fn _istat_helper_dir(inodeobj: &DirectoryInode, ret: &mut StatData) {
        ret.st_mode = inodeobj.mode;
        ret.st_nlink = inodeobj.linkcount;
        ret.st_uid = inodeobj.uid;
        ret.st_gid = inodeobj.gid;
        ret.st_rdev = 0;
        ret.st_size = inodeobj.size;
        ret.st_blksize = 0;
        ret.st_blocks = 0;
    }

    pub fn _istat_helper_chr_file(inodeobj: &DeviceInode, ret: &mut StatData) {
        //compose inode object like in glibc makedev
        ret.st_dev = 5;
        ret.st_mode = inodeobj.mode;
        ret.st_nlink = inodeobj.linkcount;
        ret.st_uid = inodeobj.uid;
        ret.st_gid = inodeobj.gid;
        ret.st_rdev = ((inodeobj.dev.major as u64 & 0x00000fff) <<  8) | 
                     ((inodeobj.dev.major as u64 & 0xfffff000) << 32) |
                     ((inodeobj.dev.minor as u64 & 0x000000ff) <<  0) |
                     ((inodeobj.dev.minor as u64 & 0xffffff00) << 12);
        ret.st_size = inodeobj.size;
    }

    //------------------ACCESS SYSCALL------------------

    fn access_syscall(&self, path: &str, amode: u32) -> i32 {
        let truepath = normpath(convpath(path), self);
        let mdobj = FS_METADATA.read().unwrap();

        if let Some(inodeno) = metawalk(truepath.as_path(), Some(&mdobj)) {
            let inodeobj = mdobj.inodetable.get(&inodeno).unwrap();

            let mode = match inodeobj {
                Inode::File(f) => {f.mode},
                Inode::CharDev(f) => {f.mode},
                Inode::Dir(f) => {f.mode},
                Inode::Pipe(_) => {
                    panic!("How did you even manage to refer to a pipe like that?");
                },
                Inode::Socket(_) => {
                    panic!("How did you even manage to refer to a socket like that?");
                },
            };

            let mut newmode: u32 = 0;
            if amode & X_OK == X_OK {newmode |= S_IXUSR;}
            if amode & W_OK == W_OK {newmode |= S_IWUSR;}
            if amode & R_OK == R_OK {newmode |= S_IRUSR;}

            if mode & newmode == newmode {0} else {-1}
        } else {
          -1
        }
    }

    fn chdir_syscall(&mut self, path: &str) -> i32 {
        let truepath = normpath(convpath(path), self);
        let mdobj = FS_METADATA.read().unwrap();
        if let Some(inodeno) = metawalk(&truepath, Some(&mdobj)) {
            if let Inode::Dir(_dir) = mdobj.inodetable.get(&inodeno).unwrap() {
                self.cwd = truepath;
                0
            } else {
                -1
            }
        } else {
            -1
        }
    }
}
