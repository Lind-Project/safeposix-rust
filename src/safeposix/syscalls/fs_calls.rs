// File system related system calls

use crate::interface;

use super::fs_constants::*;
use crate::safeposix::cage::{CAGE_TABLE, Cage, FileDescriptor::*, FileDesc};
use crate::safeposix::filesystem::*;

impl Cage {

    //------------------OPEN SYSCALL------------------

    pub fn open_syscall(&self, path: &str, flags: i32, mode: u32) -> i32 {
        //Check that path is not empty
        if path.len() != 0 {return -1;}//ENOENT later

        let truepath = normpath(convpath(path), self);

        //file descriptor table write lock held for the whole function to prevent TOCTTOU
        let mut fdtable = self.filedescriptortable.write().unwrap();
        //file system metadata table write lock held for the whole function to prevent TOCTTOU
        let mut mutmetadata = FS_METADATA.write().unwrap();

        let thisfd = if let Some(fd) = self.get_next_fd(None) {fd} else {return -1/*ENFILE*/};


        match metawalkandparent(truepath.as_path(), Some(&mutmetadata)) {
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

                if mode & (S_IRWXA | S_FILETYPEFLAGS as u32) != mode {
                    return -1; //serious error, but not sure what to put here... ENOPERM?
                } //assert sane mode bits

                let time = interface::timestamp(); //We do a real timestamp now
                let newinode = Inode::File(GenericInode {
                    size: 0, uid: DEFAULT_UID, gid: DEFAULT_GID,
                    mode: effective_mode, linkcount: 1, refcount: 0,
                    atime: time, ctime: time, mtime: time,
                });

                let newinodenum = mutmetadata.nextinode;
                mutmetadata.nextinode += 1;
                if let Inode::Dir(ind) = mutmetadata.inodetable.get_mut(&pardirinode).unwrap() {
                    ind.filename_to_inode_dict.insert(filename.unwrap().to_owned(), newinodenum);
                } //insert a reference to the file in the parent directory
                mutmetadata.inodetable.insert(newinodenum, newinode).unwrap();
                //persist metadata?
            }

            //If the file exists (we don't need to look at parent here)
            (Some(inodenum), ..) => {
                if (O_CREAT | O_EXCL) == (flags & (O_CREAT | O_EXCL)) {
                    return -1; //EEXIST later
                }

                if 0 != (flags & O_TRUNC) {
                    //close the file object if another cage has it open
                    if mutmetadata.fileobjecttable.contains_key(&inodenum) {
                        mutmetadata.fileobjecttable.get(&inodenum).unwrap().close().unwrap();
                    }

                    //set size of file to 0
                    match mutmetadata.inodetable.get_mut(&inodenum).unwrap() {
                        Inode::File(g) => {g.size = 0;}
                        Inode::Dir(_) => {return -1;/*EISDIR*/}
                        _ => {return -1;/*EINVAL*/}
                    }

                    //remove the previous file and add a new one of 0 length
                    let sysfilename = format!("{}{}", FILEDATAPREFIX, inodenum);
                    interface::removefile(sysfilename.clone()).unwrap();
                    mutmetadata.fileobjecttable.insert(inodenum, interface::openfile(sysfilename, true).unwrap());
                }
            }
        }

        //We redo our metawalk in case of O_CREAT, but this is somewhat inefficient
        if let Some(inodenum) = metawalk(truepath.as_path(), Some(&mutmetadata)) {
            let inodeobj = mutmetadata.inodetable.get_mut(&inodenum).unwrap();
            let mode;
            let size;

            //increment number of open handles to the file, retrieve other data from inode
            match inodeobj {
                Inode::File(f) => {size = f.size; mode = f.mode; f.refcount += 1},
                Inode::Dir(f) => {size = f.size; mode = f.mode; f.refcount += 1},
                Inode::CharDev(f) => {size = f.size; mode = f.mode; f.refcount += 1},
                _ => {panic!("How did you even manage to open another kind of file like that?");},
            }

            //If the file is a regular file, open the file object
            if is_reg(mode) {
                if mutmetadata.fileobjecttable.contains_key(&inodenum) {
                    let sysfilename = format!("{}{}", FILEDATAPREFIX, inodenum);
                    mutmetadata.fileobjecttable.insert(inodenum, interface::openfile(sysfilename, false).unwrap());
                }
            }

            //insert file descriptor into fdtableable of the cage
            let position = if 0 != flags & O_APPEND {size} else {0};
            let newfd = File(FileDesc {position: position, inode: inodenum, flags: flags & O_RDWRFLAGS});
            let wrappedfd = interface::RustRfc::new(interface::RustLock::new(interface::RustRfc::new(newfd)));
            fdtable.insert(thisfd, wrappedfd);
        } else {panic!("Inode not created for some reason");}
        0
    }

    //------------------CREAT SYSCALL------------------
    
    pub fn creat_syscall(&self, path: &str, mode: u32) -> i32 {
        self.open_syscall(path, O_CREAT | O_TRUNC | O_WRONLY, mode)
    }

    //------------------STAT SYSCALL------------------

    pub fn stat_syscall(&self, path: &str, statbuf: &mut StatData) -> i32 {
        let truepath = normpath(convpath(path), self);
        let metadata = FS_METADATA.read().unwrap();

        //Walk the file tree to get inode from path
        if let Some(inodenum) = metawalk(truepath.as_path(), Some(&metadata)) {
            let inodeobj = metadata.inodetable.get(&inodenum).unwrap();
            
            //populate those fields in statbuf which depend on things other than the inode object
            statbuf.st_dev = metadata.dev_id;
            statbuf.st_ino = inodenum;

            //delegate the rest of populating statbuf to the relevant helper
            match inodeobj {
                Inode::File(f) => {
                    Self::_istat_helper(f, statbuf);
                },
                Inode::CharDev(f) => {
                    Self::_istat_helper_chr_file(f, statbuf);
                },
                Inode::Dir(f) => {
                    Self::_istat_helper_dir(f, statbuf);
                },
                Inode::Pipe(_) => {
                    panic!("How did you even manage to refer to a pipe using a path?");
                },
                Inode::Socket(_) => {
                    panic!("How did you even manage to refer to a socket using a path?");
                },
            }
            0
        } else {
            -1 //
        }

    }

    fn _istat_helper(inodeobj: &GenericInode, statbuf: &mut StatData) {
        statbuf.st_mode = inodeobj.mode;
        statbuf.st_nlink = inodeobj.linkcount;
        statbuf.st_uid = inodeobj.uid;
        statbuf.st_gid = inodeobj.gid;
        statbuf.st_rdev = 0;
        statbuf.st_size = inodeobj.size;
        statbuf.st_blksize = 0;
        statbuf.st_blocks = 0;
    }

    fn _istat_helper_dir(inodeobj: &DirectoryInode, statbuf: &mut StatData) {
        statbuf.st_mode = inodeobj.mode;
        statbuf.st_nlink = inodeobj.linkcount;
        statbuf.st_uid = inodeobj.uid;
        statbuf.st_gid = inodeobj.gid;
        statbuf.st_rdev = 0;
        statbuf.st_size = inodeobj.size;
        statbuf.st_blksize = 0;
        statbuf.st_blocks = 0;
    }

    fn _istat_helper_chr_file(inodeobj: &DeviceInode, statbuf: &mut StatData) {
        statbuf.st_dev = 5;
        statbuf.st_mode = inodeobj.mode;
        statbuf.st_nlink = inodeobj.linkcount;
        statbuf.st_uid = inodeobj.uid;
        statbuf.st_gid = inodeobj.gid;
        //compose device number into u64
        statbuf.st_rdev = makedev(&inodeobj.dev);
        statbuf.st_size = inodeobj.size;
    }

    //Streams and pipes don't have associated inodes so we populate them from mostly dummy information
    fn _stat_alt_helper(&self, statbuf: &mut StatData, inodenum: usize, metadata: &FilesystemMetadata) {
        statbuf.st_dev = metadata.dev_id;
        statbuf.st_ino = inodenum;
        statbuf.st_mode = 49590; //r and w priveliged 
        statbuf.st_nlink = 1;
        statbuf.st_uid = DEFAULT_UID;
        statbuf.st_gid = DEFAULT_GID;
        statbuf.st_rdev = 0;
        statbuf.st_size = 0;
        statbuf.st_blksize = 0;
        statbuf.st_blocks = 0;
    }


    //------------------FSTAT SYSCALL------------------

    pub fn fstat_syscall(&self, fd: i32, statbuf: &mut StatData) -> i32 {
        let fdtable = self.filedescriptortable.read().unwrap();
 
        if let Some(wrappedfd) = fdtable.get(&fd) {
            let filedesc_enum = wrappedfd.read().unwrap();
            let metadata = FS_METADATA.read().unwrap();

            //Delegate populating statbuf to the relevant helper depending on the file type.
            //First we check in the file descriptor to handle sockets, streams, and pipes,
            //and if it is a normal file descriptor we handle regular files, dirs, and char 
            //files based on the information in the inode.
            match &**filedesc_enum {
                File(normalfile_filedesc_obj) => {
                    let inode = metadata.inodetable.get(&normalfile_filedesc_obj.inode).unwrap();

                    //populate those fields in statbuf which depend on things other than the inode object
                    statbuf.st_ino = normalfile_filedesc_obj.inode;
                    statbuf.st_dev = metadata.dev_id;

                    match inode {
                        Inode::File(f) => {
                            Self::_istat_helper(&f, statbuf);
                        }
                        Inode::CharDev(f) => {
                            Self::_istat_helper_chr_file(&f, statbuf);
                        }
                        Inode::Dir(f) => {
                            Self::_istat_helper_dir(&f, statbuf);
                        }
                        _ => {panic!("A file fd points to a socket or pipe");}
                    }
                }
                Socket(_) => {return -1;/*Socket fstat not implemented (yet), ENOTSUPP*/}
                Stream(_) => {self._stat_alt_helper(statbuf, STREAMINODE, &metadata);}
                Pipe(_) => {self._stat_alt_helper(statbuf, 0xfeef0000, &metadata);}
            }
            0
        } else {
            -1 //EBADF
        }
    }

    //------------------ACCESS SYSCALL------------------

    pub fn access_syscall(&self, path: &str, amode: u32) -> i32 {
        let truepath = normpath(convpath(path), self);
        let metadata = FS_METADATA.read().unwrap();


        //Walk the file tree to get inode from path
        if let Some(inodenum) = metawalk(truepath.as_path(), Some(&metadata)) {
            let inodeobj = metadata.inodetable.get(&inodenum).unwrap();

            //Get the mode bits if the type of the inode is sane
            let mode = match inodeobj {
                Inode::File(f) => {f.mode},
                Inode::CharDev(f) => {f.mode},
                Inode::Dir(f) => {f.mode},
                Inode::Pipe(_) => {
                    panic!("How did you even manage to refer to a pipe by a path?");
                },
                Inode::Socket(_) => {
                    panic!("How did you even manage to refer to a socket by a path?");
                },
            };

            //We assume that the current user owns the file

            //Construct desired access bits (i.e. 0777) based on the amode parameter
            let mut newmode: u32 = 0;
            if amode & X_OK == X_OK {newmode |= S_IXUSR;}
            if amode & W_OK == W_OK {newmode |= S_IWUSR;}
            if amode & R_OK == R_OK {newmode |= S_IRUSR;}

            //if the desired access bits are compatible with the actual access bits 
            //of the file, return a success result, else return a failure result
            if mode & newmode == newmode {0} else {-1/**/}
        } else {
          -1 //ENOENT
        }
    }

    //------------------CHDIR SYSCALL------------------
    
    pub fn chdir_syscall(&/*mut*/ self, path: &str) -> i32 {
        let truepath = normpath(convpath(path), self);
        let metadata = FS_METADATA.read().unwrap();

        //Walk the file tree to get inode from path
        if let Some(inodenum) = metawalk(&truepath, Some(&metadata)) {
            if let Inode::Dir(_dir) = metadata.inodetable.get(&inodenum).unwrap() {
                //self.cwd = truepath; as getting self as mut currently is fraught this may not be so easy
                //increment refcount to prevent current directory from being removed while you're in it
                0
            } else {
                -1 //ENOTDIR
            }
        } else {
            -1 //ENOENT
        }
    }
}
