#![allow(dead_code)]

// File system related system calls
use crate::interface;
use crate::safeposix::cage::{*, FileDescriptor::*};
use crate::safeposix::filesystem::*;
use super::fs_constants::*;

impl Cage {

    //------------------------------------OPEN SYSCALL------------------------------------

    pub fn open_syscall(&self, path: &str, flags: i32, mode: u32) -> i32 {
        //Check that path is not empty
        if path.len() == 0 {return syscall_error(Errno::ENOENT, "open", "given path was null");}

        let truepath = normpath(convpath(path), self);

        //file descriptor table write lock held for the whole function to prevent TOCTTOU
        let mut fdtable = self.filedescriptortable.write().unwrap();
        //file system metadata table write lock held for the whole function to prevent TOCTTOU
        let mut mutmetadata = FS_METADATA.write().unwrap();

        let thisfd = if let Some(fd) = self.get_next_fd(None, Some(&fdtable)) {
            fd
        } else {
            return syscall_error(Errno::ENFILE, "open", "no available file descriptor number could be found");
        };


        match metawalkandparent(truepath.as_path(), Some(&mutmetadata)) {
            //If neither the file nor parent exists
            (None, None) => {
                if 0 == (flags & O_CREAT) {
                    return syscall_error(Errno::ENOENT, "open", "tried to open a file that did not exist, and O_CREAT was not specified");
                }
                return syscall_error(Errno::ENOENT, "open", "a directory component in pathname does not exist or is a dangling symbolic link");
            }

            //If the file doesn't exist but the parent does
            (None, Some(pardirinode)) => {
                if 0 == (flags & O_CREAT) {
                    return syscall_error(Errno::ENOENT, "open", "tried to open a file that did not exist, and O_CREAT was not specified");
                }

                let filename = truepath.file_name().unwrap().to_str().unwrap().to_string(); //for now we assume this is sane, but maybe this should be checked later

                if S_IFCHR == (S_IFCHR & flags) {
                    return syscall_error(Errno::EINVAL, "open", "Invalid value in flags");
                } 

                let effective_mode = S_IFREG as u32 | mode;

                if mode & (S_IRWXA | S_FILETYPEFLAGS as u32) != mode {
                    return syscall_error(Errno::EPERM, "open", "Mode bits were not sane");
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
                    ind.filename_to_inode_dict.insert(filename, newinodenum);
                    ind.linkcount += 1;
                } //insert a reference to the file in the parent directory
                mutmetadata.inodetable.insert(newinodenum, newinode);
                persist_metadata(&mutmetadata);
            }

            //If the file exists (we don't need to look at parent here)
            (Some(inodenum), ..) => {
                if (O_CREAT | O_EXCL) == (flags & (O_CREAT | O_EXCL)) {
                    return syscall_error(Errno::EEXIST, "open", "file already exists and O_CREAT and O_EXCL were used");
                }

                if O_TRUNC == (flags & O_TRUNC) {
                    //close the file object if another cage has it open
                    let mut fobjtable = FILEOBJECTTABLE.write().unwrap();
                    if fobjtable.contains_key(&inodenum) {
                        fobjtable.get(&inodenum).unwrap().close().unwrap();
                    }

                    //set size of file to 0
                    match mutmetadata.inodetable.get_mut(&inodenum).unwrap() {
                        Inode::File(g) => {g.size = 0;}
                        _ => {
                            return syscall_error(Errno::EINVAL, "open", "file is not a normal file and thus cannot be truncated");
                        }
                    }

                    //remove the previous file and add a new one of 0 length
                    fobjtable.remove(&inodenum); //remove bookkeeping so it'll get re-created if it already is opened
                    let sysfilename = format!("{}{}", FILEDATAPREFIX, inodenum);
                    interface::removefile(sysfilename.clone()).unwrap();
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
                Inode::File(f) => {size = f.size; mode = f.mode; f.refcount += 1;}
                Inode::Dir(f) => {size = f.size; mode = f.mode; f.refcount += 1;}
                Inode::CharDev(f) => {size = f.size; mode = f.mode; f.refcount += 1;}
            }

            //If the file is a regular file, open the file object
            if is_reg(mode) {
                let mut fobjtable = FILEOBJECTTABLE.write().unwrap();
                if !fobjtable.contains_key(&inodenum) {
                    let sysfilename = format!("{}{}", FILEDATAPREFIX, inodenum);
                    fobjtable.insert(inodenum, interface::openfile(sysfilename, true).unwrap());
                }
            }

            //insert file descriptor into fdtableable of the cage
            let position = if 0 != flags & O_APPEND {size} else {0};
            let newfd = File(FileDesc {position: position, inode: inodenum, flags: flags & O_RDWRFLAGS, advlock: interface::RustRfc::new(interface::AdvisoryLock::new())});
            let wrappedfd = interface::RustRfc::new(interface::RustLock::new(newfd));
            fdtable.insert(thisfd, wrappedfd);
        } else {panic!("Inode not created for some reason");}
        thisfd //open returns the opened file descriptr
    }

    //------------------MKDIR SYSCALL------------------

    pub fn mkdir_syscall(&self, path: &str, mode: u32) -> i32 {
        //Check that path is not empty
        if path.len() == 0 {return syscall_error(Errno::ENOENT, "mkdir", "given path was null");}

        let truepath = normpath(convpath(path), self);

        let mut mutmetadata = FS_METADATA.write().unwrap();

        match metawalkandparent(truepath.as_path(), Some(&mutmetadata)) {
            //If neither the file nor parent exists
            (None, None) => {
                syscall_error(Errno::ENOENT, "mkdir", "a directory component in pathname does not exist or is a dangling symbolic link")
            }

            //If the file doesn't exist but the parent does
            (None, Some(pardirinode)) => {
                let filename = truepath.file_name().unwrap().to_str().unwrap().to_string(); //for now we assume this is sane, but maybe this should be checked later

                let effective_mode = S_IFDIR as u32 | mode;

                //assert sane mode bits
                if mode & (S_IRWXA | S_FILETYPEFLAGS as u32) != mode {
                    return syscall_error(Errno::EPERM, "mkdir", "Mode bits were not sane");
                }

                let newinodenum = mutmetadata.nextinode;
                mutmetadata.nextinode += 1;
                let time = interface::timestamp(); //We do a real timestamp now

                let newinode = Inode::Dir(DirectoryInode {
                    size: 0, uid: DEFAULT_UID, gid: DEFAULT_GID,
                    mode: effective_mode, linkcount: 3, refcount: 0, //2 because ., and .., as well as reference in parent directory
                    atime: time, ctime: time, mtime: time, 
                    filename_to_inode_dict: init_filename_to_inode_dict(newinodenum, pardirinode)
                });

                if let Inode::Dir(parentdir) = mutmetadata.inodetable.get_mut(&pardirinode).unwrap() {
                    parentdir.filename_to_inode_dict.insert(filename, newinodenum);
                    parentdir.linkcount += 1;
                } //insert a reference to the file in the parent directory
                else {unreachable!();}
                mutmetadata.inodetable.insert(newinodenum, newinode);

                persist_metadata(&mutmetadata);
                0 //mkdir has succeeded
            }

            (Some(_), ..) => {
                syscall_error(Errno::EEXIST, "mkdir", "pathname already exists, cannot create directory")
            }
        }
    }

    //------------------MKNOD SYSCALL------------------

    pub fn mknod_syscall(&self, path: &str, mode: u32, dev: u64) -> i32 {
        //Check that path is not empty
        if path.len() == 0 {return syscall_error(Errno::ENOENT, "mknod", "given path was null");}

        let truepath = normpath(convpath(path), self);

        let mut mutmetadata = FS_METADATA.write().unwrap();

        match metawalkandparent(truepath.as_path(), Some(&mutmetadata)) {
            //If neither the file nor parent exists
            (None, None) => {
                syscall_error(Errno::ENOENT, "mknod", "a directory component in pathname does not exist or is a dangling symbolic link")
            }

            //If the file doesn't exist but the parent does
            (None, Some(pardirinode)) => {
                let filename = truepath.file_name().unwrap().to_str().unwrap().to_string(); //for now we assume this is sane, but maybe this should be checked later

                //assert sane mode bits (asserting that the mode bits make sense)
                if mode & (S_IRWXA | S_FILETYPEFLAGS as u32) != mode {
                    return syscall_error(Errno::EPERM, "mknod", "Mode bits were not sane");
                }
                if mode as i32 & S_IFCHR == 0 {
                    return syscall_error(Errno::EINVAL, "mknod", "only character files are supported");
                }
                let time = interface::timestamp(); //We do a real timestamp now
                let newinode = Inode::CharDev(DeviceInode {
                    size: 0, uid: DEFAULT_UID, gid: DEFAULT_GID,
                    mode: mode, linkcount: 1, refcount: 0,
                    atime: time, ctime: time, mtime: time, dev: devtuple(dev)
                });

                let newinodenum = mutmetadata.nextinode;
                mutmetadata.nextinode += 1;
                if let Inode::Dir(parentdir) = mutmetadata.inodetable.get_mut(&pardirinode).unwrap() {
                    parentdir.filename_to_inode_dict.insert(filename, newinodenum);
                    parentdir.linkcount += 1;
                } //insert a reference to the file in the parent directory
                mutmetadata.inodetable.insert(newinodenum, newinode);

                persist_metadata(&mutmetadata);
                0 //mknod has succeeded
            }

            (Some(_), ..) => {
                syscall_error(Errno::EEXIST, "mknod", "pathname already exists, cannot create device file")
            }
        }
    }

    //------------------------------------LINK SYSCALL------------------------------------

    pub fn link_syscall(&self, oldpath: &str, newpath: &str) -> i32 {
        if oldpath.len() == 0 {return syscall_error(Errno::ENOENT, "link", "given oldpath was null");}
        if newpath.len() == 0 {return syscall_error(Errno::ENOENT, "link", "given newpath was null");}
        let trueoldpath = normpath(convpath(oldpath), self);
        let truenewpath = normpath(convpath(newpath), self);
        let filename = truenewpath.file_name().unwrap().to_str().unwrap().to_string(); //for now we assume this is sane, but maybe this should be checked later

        let mut mutmetadata = FS_METADATA.write().unwrap();

        match metawalk(trueoldpath.as_path(), Some(&mutmetadata)) {
            //If neither the file nor parent exists
            None => {
                syscall_error(Errno::ENOENT, "link", "a directory component in pathname does not exist or is a dangling symbolic link")
            }
            Some(inodenum) => {
                let inodeobj = mutmetadata.inodetable.get_mut(&inodenum).unwrap();

                match inodeobj {
                    Inode::File(ref mut normalfile_inode_obj) => {
                        normalfile_inode_obj.linkcount += 1; //add link to inode
                        match metawalkandparent(truenewpath.as_path(), Some(&mutmetadata)) {
                            (None, None) => {syscall_error(Errno::ENOENT, "link", "newpath cannot be created")}

                            (None, Some(pardirinode)) => {
                                if let Inode::Dir(ind) = mutmetadata.inodetable.get_mut(&pardirinode).unwrap() {
                                    ind.filename_to_inode_dict.insert(filename, inodenum);
                                    ind.linkcount += 1;
                                } //insert a reference to the inode in the parent directory
                                persist_metadata(&mutmetadata);
                                0 //link has succeeded
                            }

                            (Some(_), ..) => {syscall_error(Errno::EEXIST, "link", "newpath already exists")}
                        }
                    }

                    Inode::CharDev(ref mut chardev_inode_obj) => {
                        chardev_inode_obj.linkcount += 1; //add link to inode
                        match metawalkandparent(truenewpath.as_path(), Some(&mutmetadata)) {
                            (None, None) => {syscall_error(Errno::ENOENT, "link", "newpath cannot be created")}

                            (None, Some(pardirinode)) => {
                                if let Inode::Dir(ind) = mutmetadata.inodetable.get_mut(&pardirinode).unwrap() {
                                    ind.filename_to_inode_dict.insert(filename, inodenum);
                                    ind.linkcount += 1;
                                } //insert a reference to the inode in the parent directory
                                persist_metadata(&mutmetadata);
                                0 //link has succeeded
                            }

                            (Some(_), ..) => {syscall_error(Errno::EEXIST, "link", "newpath already exists")}
                        }
                    }

                    Inode::Dir(_) => {syscall_error(Errno::EPERM, "link", "oldpath is a directory")}
                }
            }
        }
    }

    //------------------------------------UNLINK SYSCALL------------------------------------

    pub fn unlink_syscall(&self, path: &str) -> i32 {
        if path.len() == 0 {return syscall_error(Errno::ENOENT, "unmknod", "given oldpath was null");}
        let truepath = normpath(convpath(path), self);

        let mut mutmetadata = FS_METADATA.write().unwrap();

        match metawalkandparent(truepath.as_path(), Some(&mutmetadata)) {
            //If the file does not exist
            (None, ..) => {
                syscall_error(Errno::ENOENT, "unlink", "path does not exist")
            }

            //If the file exists but has no parent, it's the root directory
            (Some(_), None) => {
                syscall_error(Errno::EISDIR, "unlink", "cannot unlink root directory")
            }

            //If both the file and the parent directory exists
            (Some(inodenum), Some(parentinodenum)) => {
                let inodeobj = mutmetadata.inodetable.get_mut(&inodenum).unwrap();

                let (currefcount, curlinkcount, has_fobj) = match inodeobj {
                    Inode::File(f) => {f.linkcount -= 1; (f.refcount, f.linkcount, true)},
                    Inode::CharDev(f) => {f.linkcount -= 1; (f.refcount, f.linkcount, false)},
                    Inode::Dir(_) => {return syscall_error(Errno::EISDIR, "unlink", "cannot unlink directory");},
                }; //count current number of links and references

                let parentinodeobj = mutmetadata.inodetable.get_mut(&parentinodenum).unwrap();
                let directory_parent_inode_obj = if let Inode::Dir(x) = parentinodeobj {x} else {
                    panic!("File was a child of something other than a directory????");
                };
                directory_parent_inode_obj.filename_to_inode_dict.remove(&truepath.file_name().unwrap().to_str().unwrap().to_string()); //for now we assume this is sane, but maybe this should be checked later
                directory_parent_inode_obj.linkcount -= 1;
                //remove reference to file in parent directory

                if curlinkcount == 0 {
                    if currefcount == 0  {

                        //actually remove file and the handle to it
                        mutmetadata.inodetable.remove(&inodenum);
                        if has_fobj {
                            let sysfilename = format!("{}{}", FILEDATAPREFIX, inodenum);
                            interface::removefile(sysfilename).unwrap();
                        }

                    } //we don't need a separate unlinked flag, we can just check that refcount is 0
                }
                persist_metadata(&mutmetadata);

                0 //unlink has succeeded
            }

        }
    }

    //------------------------------------CREAT SYSCALL------------------------------------
    
    pub fn creat_syscall(&self, path: &str, mode: u32) -> i32 {
        self.open_syscall(path, O_CREAT | O_TRUNC | O_WRONLY, mode)
    }

    //------------------------------------STAT SYSCALL------------------------------------

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
            }
            0 //stat has succeeded!
        } else {
            syscall_error(Errno::ENOENT, "stat", "path refers to an invalid file")
        }
    }

    fn _istat_helper(inodeobj: &GenericInode, statbuf: &mut StatData) {
        statbuf.st_nlink = inodeobj.linkcount as u64;
        statbuf.st_mode = inodeobj.mode;
        statbuf.st_uid = inodeobj.uid;
        statbuf.st_gid = inodeobj.gid;
        statbuf.__pad0 = 0;
        statbuf.st_rdev = 0;
        statbuf.st_size = inodeobj.size;
        statbuf.st_blksize = 0;
        statbuf.st_blocks = 0;
    }

    fn _istat_helper_dir(inodeobj: &DirectoryInode, statbuf: &mut StatData) {
        statbuf.st_nlink = inodeobj.linkcount as u64;
        statbuf.st_mode = inodeobj.mode;
        statbuf.st_uid = inodeobj.uid;
        statbuf.st_gid = inodeobj.gid;
        statbuf.__pad0 = 0;
        statbuf.st_rdev = 0;
        statbuf.st_size = inodeobj.size;
        statbuf.st_blksize = 0;
        statbuf.st_blocks = 0;
    }

    fn _istat_helper_chr_file(inodeobj: &DeviceInode, statbuf: &mut StatData) {
        statbuf.st_dev = 5;
        statbuf.st_nlink = inodeobj.linkcount as u64;
        statbuf.st_mode = inodeobj.mode;
        statbuf.st_uid = inodeobj.uid;
        statbuf.st_gid = inodeobj.gid;
        //compose device number into u64
        statbuf.__pad0 = 0;
        statbuf.st_rdev = makedev(&inodeobj.dev);
        statbuf.st_size = inodeobj.size;
    }

    //Streams and pipes don't have associated inodes so we populate them from mostly dummy information
    fn _stat_alt_helper(&self, statbuf: &mut StatData, inodenum: usize, metadata: &FilesystemMetadata) {
        statbuf.st_dev = metadata.dev_id;
        statbuf.st_ino = inodenum;
        statbuf.st_nlink = 1;
        statbuf.st_mode = 49590; //r and w priveliged 
        statbuf.st_uid = DEFAULT_UID;
        statbuf.st_gid = DEFAULT_GID;
        statbuf.__pad0 = 0;
        statbuf.st_rdev = 0;
        statbuf.st_size = 0;
        statbuf.st_blksize = 0;
        statbuf.st_blocks = 0;
    }


    //------------------------------------FSTAT SYSCALL------------------------------------

    pub fn fstat_syscall(&self, fd: i32, statbuf: &mut StatData) -> i32 {
        let fdtable = self.filedescriptortable.read().unwrap();
 
        if let Some(wrappedfd) = fdtable.get(&fd) {
            let filedesc_enum = wrappedfd.read().unwrap();
            let metadata = FS_METADATA.read().unwrap();

            //Delegate populating statbuf to the relevant helper depending on the file type.
            //First we check in the file descriptor to handle sockets, streams, and pipes,
            //and if it is a normal file descriptor we handle regular files, dirs, and char 
            //files based on the information in the inode.
            match &*filedesc_enum {
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
                    }
                }
                Socket(_) => {
                    return syscall_error(Errno::EOPNOTSUPP, "fstat", "we don't support fstat on sockets yet");
                }
                Stream(_) => {self._stat_alt_helper(statbuf, STREAMINODE, &metadata);}
                Pipe(_) => {self._stat_alt_helper(statbuf, 0xfeef0000, &metadata);}
                Epoll(_) => {self._stat_alt_helper(statbuf, 0xfeef0000, &metadata);}
            }
            0 //fstat has succeeded!
        } else {
            syscall_error(Errno::ENOENT, "fstat", "invalid file descriptor")
        }
    }

    //------------------------------------STATFS SYSCALL------------------------------------

    pub fn statfs_syscall(&self, path: &str, databuf: &mut FSData) -> i32 {
        let truepath = normpath(convpath(path), self);
        let metadata = FS_METADATA.read().unwrap();

        //Walk the file tree to get inode from path
        if let Some(inodenum) = metawalk(truepath.as_path(), Some(&metadata)) {
            let _inodeobj = metadata.inodetable.get(&inodenum).unwrap();
            
            //populate the dev id field -- can be done outside of the helper
            databuf.f_fsid = metadata.dev_id;

            //delegate the rest of populating statbuf to the relevant helper
            return Self::_istatfs_helper(self, databuf);
        } else {
            syscall_error(Errno::ENOENT, "stat", "path refers to an invalid file")
        }
    }

    //------------------------------------FSTATFS SYSCALL------------------------------------

    pub fn fstatfs_syscall(&self, fd: i32, databuf: &mut FSData) -> i32 {
        let fdtable = self.filedescriptortable.read().unwrap();

        if let Some(wrappedfd) = fdtable.get(&fd) {
            let filedesc_enum = wrappedfd.read().unwrap();
            let metadata = FS_METADATA.read().unwrap();
            
            //populate the dev id field -- can be done outside of the helper
            databuf.f_fsid = metadata.dev_id;

            match &*filedesc_enum {
                File(normalfile_filedesc_obj) => {
                    let _inodeobj = metadata.inodetable.get(&normalfile_filedesc_obj.inode).unwrap();

                    return Self::_istatfs_helper(self, databuf);
                },
                Socket(_) | Pipe(_) | Stream(_) | Epoll(_)=> {return syscall_error(Errno::EBADF, "fstatfs", "can't fstatfs on socket, stream, pipe, or epollfd");}
            }
        }
        return syscall_error(Errno::EBADF, "statfs", "invalid file descriptor");
    }
    
    pub fn _istatfs_helper(&self, databuf: &mut FSData) -> i32 {
        
        databuf.f_type = 0xBEEFC0DE; //unassigned 
        databuf.f_bsize = 4096;
        databuf.f_blocks = 0; //int(limits['diskused']) / 4096
        databuf.f_bfree = 1024 * 1024 * 1024; //(int(limits['diskused']-usage['diskused'])) / 4096
        databuf.f_bavail = 1024 * 1024 * 1024; //(int(limits['diskused']-usage['diskused'])) / 4096
        databuf.f_files = 1024*1024*1024;
        databuf.f_ffiles = 1024*1024*515;
        databuf.f_namelen = 254;
        databuf.f_frsize = 4096;
        databuf.f_spare = [0; 32];

        0 //success!
    }

    //------------------------------------READ SYSCALL------------------------------------

    pub fn read_syscall(&self, fd: i32, buf: *mut u8, count: usize) -> i32 {
        let fdtable = self.filedescriptortable.read().unwrap();
        
        if let Some(wrappedfd) = fdtable.get(&fd) {
            let mut filedesc_enum = wrappedfd.write().unwrap();

            //delegate to pipe, stream, or socket helper if specified by file descriptor enum type (none of them are implemented yet)
            match &mut *filedesc_enum {
                //we must borrow the filedesc object as a mutable reference to update the position
                File(ref mut normalfile_filedesc_obj) => {
                    if is_wronly(normalfile_filedesc_obj.flags) {
                        return syscall_error(Errno::EBADF, "read", "specified file not open for reading");
                    }

                    let metadata = FS_METADATA.read().unwrap();
                    let inodeobj = metadata.inodetable.get(&normalfile_filedesc_obj.inode).unwrap();

                    //delegate to character if it's a character file, checking based on the type of the inode object
                    match inodeobj {
                        Inode::File(_) => {
                            let position = normalfile_filedesc_obj.position;
                            let fobjtable = FILEOBJECTTABLE.read().unwrap();
                            let fileobject = fobjtable.get(&normalfile_filedesc_obj.inode).unwrap();

                            if let Ok(bytesread) = fileobject.readat(buf, count, position) {
                                //move position forward by the number of bytes we've read

                                normalfile_filedesc_obj.position += bytesread;
                                bytesread as i32
                            } else {
                               0 //0 bytes read, but not an error value that can/should be passed to the user
                            }
                        }

                        Inode::CharDev(char_inode_obj) => {
                            self._read_chr_file(char_inode_obj, buf, count)
                        }

                        Inode::Dir(_) => {
                            syscall_error(Errno::EISDIR, "read", "attempted to read from a directory")
                        }
                    }
                }
                Socket(_) => {syscall_error(Errno::EOPNOTSUPP, "read", "recv not implemented yet")}
                Stream(_) => {syscall_error(Errno::EOPNOTSUPP, "read", "reading from stdin not implemented yet")}
                Pipe(pipe_filedesc_obj) => {
                    if is_wronly(pipe_filedesc_obj.flags) {
                        return syscall_error(Errno::EBADF, "read", "specified file not open for reading");
                    }

                    // get the pipe, read from it, and return bytes read
                    let pipe = PIPE_TABLE.read().unwrap().get(&pipe_filedesc_obj.pipe).unwrap().clone();
                    pipe.read_from_pipe(buf, count) as i32
                }
                Epoll(_) => {syscall_error(Errno::EINVAL, "read", "fd is attached to an object which is unsuitable for reading")}
            }
        } else {
            syscall_error(Errno::EBADF, "read", "invalid file descriptor")
        }
    }

    //------------------------------------PREAD SYSCALL------------------------------------
    pub fn pread_syscall(&self, fd: i32, buf: *mut u8, count: usize, offset: isize) -> i32 {
        let fdtable = self.filedescriptortable.read().unwrap();
 
        if let Some(wrappedfd) = fdtable.get(&fd) {
            let mut filedesc_enum = wrappedfd.write().unwrap();

            match &mut *filedesc_enum {
                //we must borrow the filedesc object as a mutable reference to update the position
                File(ref mut normalfile_filedesc_obj) => {
                    if is_wronly(normalfile_filedesc_obj.flags) {
                        return syscall_error(Errno::EBADF, "pread", "specified file not open for reading");
                    }

                    let metadata = FS_METADATA.read().unwrap();
                    let inodeobj = metadata.inodetable.get(&normalfile_filedesc_obj.inode).unwrap();

                    //delegate to character if it's a character file, checking based on the type of the inode object
                    match inodeobj {
                        Inode::File(_) => {
                            let fobjtable = FILEOBJECTTABLE.read().unwrap();
                            let fileobject = fobjtable.get(&normalfile_filedesc_obj.inode).unwrap();

                            if let Ok(bytesread) = fileobject.readat(buf, count, offset as usize) {
                                bytesread as i32
                            } else {
                               0 //0 bytes read, but not an error value that can/should be passed to the user
                            }
                        }

                        Inode::CharDev(char_inode_obj) => {
                            self._read_chr_file(char_inode_obj, buf, count)
                        }

                        Inode::Dir(_) => {
                            syscall_error(Errno::EISDIR, "pread", "attempted to read from a directory")
                        }
                    }
                }
                Socket(_) => {
                    syscall_error(Errno::ESPIPE, "pread", "file descriptor is associated with a socket, cannot seek")
                }
                Stream(_) => {
                    syscall_error(Errno::ESPIPE, "pread", "file descriptor is associated with a stream, cannot seek")
                }
                Pipe(_) => {
                    syscall_error(Errno::ESPIPE, "pread", "file descriptor is associated with a pipe, cannot seek")
                }
                Epoll(_) => {
                    syscall_error(Errno::ESPIPE, "pread", "file descriptor is associated with an epollfd, cannot seek")
                }
            }
        } else {
            syscall_error(Errno::EBADF, "pread", "invalid file descriptor")
        }
    }

    fn _read_chr_file(&self, inodeobj: &DeviceInode, buf: *mut u8, count: usize) -> i32 {
        match inodeobj.dev {
            NULLDEVNO => {0} //reading from /dev/null always reads 0 bytes
            ZERODEVNO => {interface::fillzero(buf, count)}
            RANDOMDEVNO => {interface::fillrandom(buf, count)}
            URANDOMDEVNO => {interface::fillrandom(buf, count)}
            _ => {syscall_error(Errno::EOPNOTSUPP, "read or pread", "read from specified device not implemented")}
        }
    }

    //------------------------------------WRITE SYSCALL------------------------------------

    pub fn write_syscall(&self, fd: i32, buf: *const u8, count: usize) -> i32 {
        let fdtable = self.filedescriptortable.read().unwrap();
 
        if let Some(wrappedfd) = fdtable.get(&fd) {
            let mut filedesc_enum = wrappedfd.write().unwrap();

            //delegate to pipe, stream, or socket helper if specified by file descriptor enum type
            match &mut *filedesc_enum {
                //we must borrow the filedesc object as a mutable reference to update the position
                File(ref mut normalfile_filedesc_obj) => {
                    if is_rdonly(normalfile_filedesc_obj.flags) {
                        return syscall_error(Errno::EBADF, "write", "specified file not open for writing");
                    }

                    let mut metadata = FS_METADATA.write().unwrap();

                    let inodeobj = metadata.inodetable.get_mut(&normalfile_filedesc_obj.inode).unwrap();

                    //delegate to character helper or print out if it's a character file or stream,
                    //checking based on the type of the inode object
                    match inodeobj {
                        Inode::File(ref mut normalfile_inode_obj) => {
                            let position = normalfile_filedesc_obj.position;

                            let filesize = normalfile_inode_obj.size;
                            let blankbytecount = position as isize - filesize as isize;

                            let mut fobjtable = FILEOBJECTTABLE.write().unwrap();
                            let fileobject = fobjtable.get_mut(&normalfile_filedesc_obj.inode).unwrap();

                            //we need to pad the file with blank bytes if we are at a position past the end of the file!
                            if blankbytecount > 0 {
                                if let Ok(byteswritten) = fileobject.zerofill_at(filesize, blankbytecount as usize) {
                                    if byteswritten != blankbytecount as usize {
                                        panic!("Write of blank bytes for write failed!");
                                    }
                                } else {
                                    panic!("Write of blank bytes for write failed!");
                                }
                            }

                            let newposition;
                            if let Ok(byteswritten) = fileobject.writeat(buf, count, position) {
                                //move position forward by the number of bytes we've written
                                normalfile_filedesc_obj.position = position + byteswritten;
                                newposition = normalfile_filedesc_obj.position;
                                if newposition > normalfile_inode_obj.size {
                                    normalfile_inode_obj.size = newposition;
                                    persist_metadata(&metadata);
                                } //update file size if necessary
                                
                                byteswritten as i32
                            } else {
                                0 //0 bytes written, but not an error value that can/should be passed to the user
                            }
                        }

                        Inode::CharDev(char_inode_obj) => {
                            self._write_chr_file(char_inode_obj, buf, count)
                        }

                        Inode::Dir(_) => {
                            syscall_error(Errno::EISDIR, "write", "attempted to write to a directory")
                        }
                    }
                }
                Socket(_) => {syscall_error(Errno::EOPNOTSUPP, "write", "send not implemented yet")}
                Stream(stream_filedesc_obj) => {
                    //if it's stdout or stderr, print out and we're done
                    if stream_filedesc_obj.stream == 1 || stream_filedesc_obj.stream == 2 {
                        interface::log_from_ptr(buf, count);
                        count as i32
                    } else {
                        return syscall_error(Errno::EBADF, "write", "specified stream not open for writing");
                    }
                }
                Pipe(pipe_filedesc_obj) => {
                    if is_rdonly(pipe_filedesc_obj.flags) {
                        return syscall_error(Errno::EBADF, "write", "specified pipe not open for writing");
                    }
                    // get the pipe, write to it, and return bytes written
                    let pipe = PIPE_TABLE.read().unwrap().get(&pipe_filedesc_obj.pipe).unwrap().clone();
                    pipe.write_to_pipe(buf, count) as i32
  
                }
                Epoll(_) => {syscall_error(Errno::EINVAL, "write", "fd is attached to an object which is unsuitable for writing")}
            }
        } else {
            syscall_error(Errno::EBADF, "write", "invalid file descriptor")
        }
    }

    //------------------------------------PWRITE SYSCALL------------------------------------

    pub fn pwrite_syscall(&self, fd: i32, buf: *const u8, count: usize, offset: isize) -> i32 {
        let fdtable = self.filedescriptortable.read().unwrap();
 
        if let Some(wrappedfd) = fdtable.get(&fd) {
            let mut filedesc_enum = wrappedfd.write().unwrap();

            match &mut *filedesc_enum {
                //we must borrow the filedesc object as a mutable reference to update the position
                File(ref mut normalfile_filedesc_obj) => {
                    if is_rdonly(normalfile_filedesc_obj.flags) {
                        return syscall_error(Errno::EBADF, "pwrite", "specified file not open for writing");
                    }

                    let mut metadata = FS_METADATA.write().unwrap();

                    let inodeobj = metadata.inodetable.get_mut(&normalfile_filedesc_obj.inode).unwrap();

                    //delegate to character helper or print out if it's a character file or stream,
                    //checking based on the type of the inode object
                    match inodeobj {
                        Inode::File(ref mut normalfile_inode_obj) => {
                            let position = offset as usize;
                            let filesize = normalfile_inode_obj.size;
                            let blankbytecount = offset - filesize as isize;

                            let mut fobjtable = FILEOBJECTTABLE.write().unwrap();
                            let fileobject = fobjtable.get_mut(&normalfile_filedesc_obj.inode).unwrap();

                            //we need to pad the file with blank bytes if we are seeking past the end of the file!
                            if blankbytecount > 0 {
                                if let Ok(byteswritten) = fileobject.zerofill_at(filesize, blankbytecount as usize) {
                                    if byteswritten != blankbytecount as usize {
                                        panic!("Write of blank bytes for pwrite failed!");
                                    }
                                } else {
                                    panic!("Write of blank bytes for pwrite failed!");
                                }
                            }

                            let newposition;
                            let retval = if let Ok(byteswritten) = fileobject.writeat(buf, count, position) {
                                //move position forward by the number of bytes we've written
                                newposition = position + byteswritten;

                                byteswritten as i32
                            } else {
                                newposition = position;
                                0 //0 bytes written, but not an error value that can/should be passed to the user
                                  //we still may need to update file size from blank bytes write, so we don't bail out
                            };

                            if newposition > filesize {
                               normalfile_inode_obj.size = newposition;
                               persist_metadata(&metadata);
                            } //update file size if necessary

                            retval
                        }

                        Inode::CharDev(char_inode_obj) => {
                            self._write_chr_file(char_inode_obj, buf, count)
                        }

                        Inode::Dir(_) => {
                            syscall_error(Errno::EISDIR, "pwrite", "attempted to write to a directory")
                        }
                    }
                }
                Socket(_) => {
                    syscall_error(Errno::ESPIPE, "pwrite", "file descriptor is associated with a socket, cannot seek")
                }
                Stream(_) => {
                    syscall_error(Errno::ESPIPE, "pwrite", "file descriptor is associated with a stream, cannot seek")
                }
                Pipe(_) => {
                    syscall_error(Errno::ESPIPE, "pwrite", "file descriptor is associated with a pipe, cannot seek")
                }
                Epoll(_) => {
                    syscall_error(Errno::ESPIPE, "pwrite", "file descriptor is associated with an epollfd, cannot seek")
                }
            }
        } else {
            syscall_error(Errno::EBADF, "pwrite", "invalid file descriptor")
        }
    }

    fn _write_chr_file(&self, inodeobj: &DeviceInode, _buf: *const u8, count: usize) -> i32 {
        //writes to any of these device files transparently succeed while doing nothing
        match inodeobj.dev {
            NULLDEVNO => {count as i32}
            ZERODEVNO => {count as i32}
            RANDOMDEVNO => {count as i32}
            URANDOMDEVNO => {count as i32}
            _ => {syscall_error(Errno::EOPNOTSUPP, "write or pwrite", "write to specified device not implemented")}
        }
    }

    //------------------------------------LSEEK SYSCALL------------------------------------
    pub fn lseek_syscall(&self, fd: i32, offset: isize, whence: i32) -> i32 {
        let fdtable = self.filedescriptortable.read().unwrap();
 
        if let Some(wrappedfd) = fdtable.get(&fd) {
            let mut filedesc_enum = wrappedfd.write().unwrap();

            //confirm fd type is seekable
            match &mut *filedesc_enum {
                File(ref mut normalfile_filedesc_obj) => {
                    let metadata = FS_METADATA.read().unwrap();
                    let inodeobj = metadata.inodetable.get(&normalfile_filedesc_obj.inode).unwrap();

                    //handle files/directories differently
                    match inodeobj {
                        Inode::File(normalfile_inode_obj) => {
                            let eventualpos = match whence {
                                SEEK_SET => {offset}
                                SEEK_CUR => {normalfile_filedesc_obj.position as isize + offset}
                                SEEK_END => {normalfile_inode_obj.size as isize + offset}
                                _ => {return syscall_error(Errno::EINVAL, "lseek", "unknown whence");}
                            };

                            if eventualpos < 0 {
                                return syscall_error(Errno::EINVAL, "lseek", "seek to before position 0 in file");
                            }
                            //subsequent writes to the end of the file must zero pad up until this point if we
                            //overran the end of our file when seeking

                            normalfile_filedesc_obj.position = eventualpos as usize;
                            //return the location that we sought to
                            eventualpos as i32
                        }

                        Inode::CharDev(_) => {
                            0 //for character files, rather than seeking, we transparently do nothing
                        }

                        Inode::Dir(dir_inode_obj) => {
                            //for directories we seek between entries, and thus our end position is the total number of entries
                            let eventualpos = match whence {
                                SEEK_SET => {offset}
                                SEEK_CUR => {normalfile_filedesc_obj.position as isize + offset}
                                SEEK_END => {dir_inode_obj.filename_to_inode_dict.len() as isize + offset}
                                _ => {return syscall_error(Errno::EINVAL, "lseek", "unknown whence");}
                            };

                            //confirm that the location we want to seek to is valid
                            if eventualpos < 0 {
                                return syscall_error(Errno::EINVAL, "lseek", "seek to before position 0 in directory");
                            }
                            if eventualpos > dir_inode_obj.filename_to_inode_dict.len() as isize {
                                return syscall_error(Errno::EINVAL, "lseek", "seek to after last position in directory");
                            }

                            normalfile_filedesc_obj.position = eventualpos as usize;
                            //return the location that we sought to
                            eventualpos as i32
                        }
                    }
                }
                Socket(_) => {
                    syscall_error(Errno::ESPIPE, "lseek", "file descriptor is associated with a socket, cannot seek")
                }
                Stream(_) => {
                    syscall_error(Errno::ESPIPE, "lseek", "file descriptor is associated with a stream, cannot seek")
                }
                Pipe(_) => {
                    syscall_error(Errno::ESPIPE, "lseek", "file descriptor is associated with a pipe, cannot seek")
                }
                Epoll(_) => {
                    syscall_error(Errno::ESPIPE, "lseek", "file descriptor is associated with an epollfd, cannot seek")
                }
            }
        } else {
            syscall_error(Errno::EBADF, "lseek", "invalid file descriptor")
        }

    }


    //------------------------------------ACCESS SYSCALL------------------------------------

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
            };

            //We assume that the current user owns the file

            //Construct desired access bits (i.e. 0777) based on the amode parameter
            let mut newmode: u32 = 0;
            if amode & X_OK == X_OK {newmode |= S_IXUSR;}
            if amode & W_OK == W_OK {newmode |= S_IWUSR;}
            if amode & R_OK == R_OK {newmode |= S_IRUSR;}

            //if the desired access bits are compatible with the actual access bits 
            //of the file, return a success result, else return a failure result
            if mode & newmode == newmode {
                0
            } else {
                syscall_error(Errno::EACCES, "access", "the requested access would be denied to the file")
            }
        } else {
            syscall_error(Errno::ENOENT, "access", "path does not refer to an existing file")
        }
    }

    //------------------------------------CHDIR SYSCALL------------------------------------
    
    pub fn chdir_syscall(&self, path: &str) -> i32 {
        let truepath = normpath(convpath(path), self);
        let mut mutmetadata = FS_METADATA.write().unwrap();

        //Walk the file tree to get inode from path
        if let Some(inodenum) = metawalk(&truepath, Some(&mutmetadata)) {
            if let Inode::Dir(ref mut dir) = mutmetadata.inodetable.get_mut(&inodenum).unwrap() {

                //increment refcount of new cwd inode to ensure that you can't remove a directory while it is the cwd of a cage
                dir.refcount += 1;

            } else {
                return syscall_error(Errno::ENOTDIR, "chdir", "the last component in path is not a directory");
            }
        } else {
            return syscall_error(Errno::ENOENT, "chdir", "the directory referred to in path does not exist");
        }
        //at this point, syscall isn't an error
        let mut cwd_container = self.cwd.write().unwrap();

        //decrement refcount of previous cwd's inode, to allow it to be removed if no cage has it as cwd
        decref_dir(&mut mutmetadata, &*cwd_container);

        *cwd_container = interface::RustRfc::new(truepath);
        0 //chdir has succeeded!;
    }

    //------------------------------------DUP & DUP2 SYSCALLS------------------------------------

    pub fn dup_syscall(&self, fd: i32, start_desc: Option<i32>) -> i32 {
        let mut fdtable = self.filedescriptortable.write().unwrap();

        //if a starting fd was passed, then use that as the starting point, but otherwise, use the designated minimum of STARTINGFD
        let start_fd = match start_desc {
            Some(start_desc) => start_desc,
            None => STARTINGFD,
        };

        //checking whether the fd exists in the file table
        if let Some(_) = fdtable.get(&fd) {
            let nextfd = if let Some(fd) = self.get_next_fd(Some(start_fd), Some(&fdtable)) {fd} 
            else {return syscall_error(Errno::ENFILE, "dup", "no available file descriptor number could be found");};
            return Self::_dup2_helper(&self, fd, nextfd, Some(&mut fdtable))
        } else {
            return syscall_error(Errno::EBADF, "dup", "file descriptor not found")
        }
    }

    pub fn dup2_syscall(&self, oldfd: i32, newfd: i32) -> i32{
        let mut fdtable = self.filedescriptortable.write().unwrap();

        //if the old fd exists, execute the helper, else return error
        if let Some(_) = fdtable.get(&oldfd) {
            return Self::_dup2_helper(&self, oldfd, newfd, Some(&mut fdtable));
        } else {
            return syscall_error(Errno::EBADF, "dup2","Invalid old file descriptor.");
        }
    }

    pub fn _dup2_helper(&self, oldfd: i32, newfd: i32, fdtable_lock: Option<&mut FdTable>) -> i32 {
        
        //pass the lock of the FdTable to this helper. If passed table is none, then create new lock instance
        let mut writer;
        let fdtable = if let Some(fdtb) = fdtable_lock {fdtb} else {
            writer = self.filedescriptortable.write().unwrap(); 
            &mut writer
        };
        
        //checking if the new fd is out of range
        if newfd >= MAXFD || newfd < 0 {
            return syscall_error(Errno::EBADF, "dup or dup2", "provided file descriptor is out of range");
        }

        {
            let locked_filedesc = fdtable.get(&oldfd).unwrap();
            let filedesc_enum = locked_filedesc.read().unwrap();
            let mut mutmetadata = FS_METADATA.write().unwrap();

            match &*filedesc_enum {
                File(normalfile_filedesc_obj) => {
                    let inodenum = normalfile_filedesc_obj.inode;
                    let inodeobj = mutmetadata.inodetable.get_mut(&inodenum).unwrap();
                    //incrementing the ref count so that when close is executed on the dup'd file
                    //the original file does not get a negative ref count
                    match inodeobj {
                        Inode::File(normalfile_inode_obj) => {
                            normalfile_inode_obj.refcount += 1;
                        },
                        Inode::Dir(dir_inode_obj) => {
                            dir_inode_obj.refcount += 1;
                        },
                        Inode::CharDev(chardev_inode_obj) => {
                            chardev_inode_obj.refcount += 1;
                        },
                    }
                },
                Pipe(normalfile_filedesc_obj) => {
                    let pipe = PIPE_TABLE.write().unwrap().get(&normalfile_filedesc_obj.pipe).unwrap().clone();
                    pipe.incr_ref(normalfile_filedesc_obj.flags);
                },
                Stream(normalfile_filedesc_obj) => {
                    // no stream refs
                }
                _ => {return syscall_error(Errno::EACCES, "dup or dup2", "can't dup the provided file");},
            }
        }
        
        //if the file descriptors are equal, return the new one
        if newfd == oldfd {
            return newfd;
        }

        //close the fd in the way of the new fd. If an error is returned from the helper, return the error, else continue to end
        if fdtable.contains_key(&newfd) {
            let close_result = Self::_close_helper(&self, newfd, Some(fdtable));
            if close_result != 0 {
                return close_result;
            }
        }    

        // get and clone fd, wrap and insert into table.
        let filedesc_clone;
        {
            let locked_oldfiledesc = fdtable.get(&oldfd).unwrap();
            let oldfiledesc_enum = locked_oldfiledesc.read().unwrap();
            filedesc_clone = (*&oldfiledesc_enum).clone();
        }

        let wrappedfd = interface::RustRfc::new(interface::RustLock::new(filedesc_clone));
        fdtable.insert(newfd, wrappedfd);
        return newfd;
    }

    //------------------------------------CLOSE SYSCALL------------------------------------

    pub fn close_syscall(&self, fd: i32) -> i32 {
        let mut fdtable = self.filedescriptortable.write().unwrap();
 
        //check that the fd is valid
        match fdtable.get(&fd) {
            Some(_) => {return Self::_close_helper(self, fd, Some(&mut fdtable));},
            None => {return syscall_error(Errno::EBADF, "close", "invalid file descriptor");},
        }
    }

    pub fn _close_helper(&self, fd: i32, fdtable_lock: Option<&mut FdTable>) -> i32 {
        //pass the lock of the FdTable to this helper. If passed table is none, then create new lock instance
        let mut writer;
        let fdtable = if let Some(rl) = fdtable_lock {rl} else {
            writer = self.filedescriptortable.write().unwrap(); 
            &mut writer
        };

        //unpacking and getting the type to match for
        {
            let locked_filedesc = fdtable.get(&fd).unwrap();
            let filedesc_enum = locked_filedesc.read().unwrap();
            let mut mutmetadata = FS_METADATA.write().unwrap();

            //Decide how to proceed depending on the fd type.
            //First we check in the file descriptor to handle sockets (no-op), sockets (clean the socket), and pipes (clean the pipe),
            //and if it is a normal file descriptor we decrement the refcount to reflect
            //one less reference to the file.
            match &*filedesc_enum {
                //if we are a socket, we dont change disk metadata
                Stream(_) => {},
                Epoll(_) => {}, //Epoll closing not implemented yet
                Socket(_) => {
                    //drop(filedesc_enum);    //to appease Rust ownership, we drop the fdtable borrow before calling cleanup_socket
                    //drop(locked_filedesc);  
                    //let retval = Self::_cleanup_socket(self, fd, false, fdtable);
                    //if retval != 0 {return retval;}
                },
                Pipe(pipe_filedesc_obj) => {
                    let pipe = PIPE_TABLE.write().unwrap().get(&pipe_filedesc_obj.pipe).unwrap().clone();
               
                    pipe.decr_ref(pipe_filedesc_obj.flags);

                    //Code below needs to reflect addition of pipes
                    if pipe.get_write_ref() == 0 && pipe_filedesc_obj.flags == O_WRONLY {
                        // we're closing the last write end, lets set eof
                        pipe.set_eof();
                    }

                    if pipe.get_write_ref() + pipe.get_read_ref() == 0 {
                        // last reference, lets remove it
                        PIPE_TABLE.write().unwrap().remove(&pipe_filedesc_obj.pipe).unwrap();
                    }

                },
                File(normalfile_filedesc_obj) => {
                    let inodenum = normalfile_filedesc_obj.inode;
                    let inodeobj = mutmetadata.inodetable.get_mut(&inodenum).unwrap();
                    let mut fobjtable = FILEOBJECTTABLE.write().unwrap();

                    match inodeobj {
                        Inode::File(ref mut normalfile_inode_obj) => {
                            normalfile_inode_obj.refcount -= 1;

                            //if it's not a reg file, then we have nothing to close
                            //Inode::File is a regular file by default
                            if normalfile_inode_obj.refcount == 0 {
                                fobjtable.remove(&inodenum).unwrap().close().unwrap();
                                if normalfile_inode_obj.linkcount == 0 {
                                    //removing the file from the entire filesystem (interface, metadata, and object table)
                                    mutmetadata.inodetable.remove(&inodenum);
                                    let sysfilename = format!("{}{}", FILEDATAPREFIX, inodenum);
                                    interface::removefile(sysfilename).unwrap();
                                } 
                                persist_metadata(&mutmetadata);
                            }
                        },
                        Inode::Dir(ref mut dir_inode_obj) => {
                            dir_inode_obj.refcount -= 1;

                            //if it's not a reg file, then we have nothing to close
                            match fobjtable.get(&inodenum) {
                                Some(_) => {return syscall_error(Errno::ENOEXEC, "close or dup", "Non-regular file in file object table");},
                                None => {}
                            }
                            if dir_inode_obj.linkcount == 2 && dir_inode_obj.refcount == 0 {
                                //removing the file from the metadata 
                                mutmetadata.inodetable.remove(&inodenum);
                                persist_metadata(&mutmetadata);
                            } 
                        }
                        Inode::CharDev(ref mut char_inode_obj) => {
                            char_inode_obj.refcount -= 1;

                            //if it's not a reg file, then we have nothing to close
                            match fobjtable.get(&inodenum) {
                                Some(_) => {return syscall_error(Errno::ENOEXEC, "close or dup", "Non-regular file in file object table");},
                                None => {}
                            }
                            if char_inode_obj.linkcount == 0 && char_inode_obj.refcount == 0 {
                                //removing the file from the metadata 
                                mutmetadata.inodetable.remove(&inodenum);
                                persist_metadata(&mutmetadata);
                            } 
                        }
                    }
                },
            }
        }

        //removing inode from fd table
        fdtable.remove(&fd);
        0 //_close_helper has succeeded!
    }
    
    //------------------------------------FCNTL SYSCALL------------------------------------
    
    pub fn fcntl_syscall(&self, fd: i32, cmd: i32, arg: i32) -> i32 {
        let fdtable = self.filedescriptortable.write().unwrap();

        if let Some(wrappedfd) = fdtable.get(&fd) {
            let mut filedesc_enum = wrappedfd.write().unwrap();

            let flags = match &mut *filedesc_enum {
                Epoll(obj) => {&mut obj.flags},
                Pipe(obj) => {&mut obj.flags},
                Stream(obj) => {&mut obj.flags},
                Socket(obj) => {&mut obj.flags},
                File(obj) => {&mut obj.flags},
            };
            
            //matching the tuple
            match (cmd, arg) {
                //because the arg parameter is not used in certain commands, it can be anything (..)
                (F_GETFD, ..) => {
                    ((*flags & O_CLOEXEC) != 0) as i32
                }
                // set the flags but make sure that the flags are valid
                (F_SETFD, arg) if arg >= 0 => {
                    *flags |= O_CLOEXEC;
                    0
                }
                (F_GETFL, ..) => {
                    //for get, we just need to return the flags
                    *flags
                }
                (F_SETFL, arg) if arg >= 0 => {
                    *flags = arg;
                    0
                }
                (F_DUPFD, arg) if arg >= 0 => {
                    Self::dup_syscall(self, fd, Some(arg))
                }
                //TO DO: implement. this one is saying get the signals
                (F_GETOWN, ..) => {
                    0 //TO DO: traditional SIGIO behavior
                }
                (F_SETOWN, arg) if arg >= 0 => {
                    0 //this would return the PID if positive and the process group if negative,
                    //either way do nothing and return success
                }
                _ => {syscall_error(Errno::EINVAL, "fcntl", "Arguments provided do not match implemented parameters")}
            }
        } else {
            syscall_error(Errno::EBADF, "fcntl", "Invalid file descriptor")
        }
    }

    //------------------------------------CHMOD SYSCALL------------------------------------

    pub fn chmod_syscall(&self, path: &str, mode: u32) -> i32 {

        let mut metadata = FS_METADATA.write().unwrap();
        let truepath = normpath(convpath(path), self);

        //check if there is a valid path or not there to an inode
        if let Some(inodenum) = metawalk(truepath.as_path(), Some(&metadata)) {
            let thisinode = metadata.inodetable.get_mut(&inodenum).unwrap();
            if mode & (S_IRWXA|(S_FILETYPEFLAGS as u32)) == mode {
                match thisinode {
                    Inode::File(ref mut general_inode) => {
                        general_inode.mode = (general_inode.mode &!S_IRWXA) | mode
                    }
                    Inode::CharDev(ref mut dev_inode) => {
                        dev_inode.mode = (dev_inode.mode &!S_IRWXA) | mode;
                    }
                    Inode::Dir(ref mut dir_inode) => {
                        dir_inode.mode = (dir_inode.mode &!S_IRWXA) | mode;
                    }
                }
            }
            else {
                //there doesn't seem to be a good syscall error errno for this
                return syscall_error(Errno::EACCES, "chmod", "provided file mode is not valid");
            }
        } else {
            return syscall_error(Errno::ENOENT, "chmod", "the provided path does not exist");
        }
        persist_metadata(&metadata);
        0 //success!
    }

    //------------------------------------MMAP SYSCALL------------------------------------
    
    pub fn mmap_syscall(&self, addr: *mut u8, len: usize, prot: i32, flags: i32, fildes: i32, off: i64) -> i32 {
        if len == 0 {syscall_error(Errno::EINVAL, "mmap", "the value of len is 0");}

        if 0 == flags & (MAP_PRIVATE | MAP_SHARED) {
            syscall_error(Errno::EINVAL, "mmap", "The value of flags is invalid (neither MAP_PRIVATE nor MAP_SHARED is set)");
        }

        if 0 != flags & MAP_ANONYMOUS {
            return interface::libc_mmap(addr, len, prot, flags, -1, 0);
        }

        let fdtable = self.filedescriptortable.read().unwrap();
        if let Some(wrappedfd) = fdtable.get(&fildes) {
            let mut filedesc_enum = wrappedfd.write().unwrap();

            //confirm fd type is mappable
            match &mut *filedesc_enum {
                File(ref mut normalfile_filedesc_obj) => {
                    let metadata = FS_METADATA.read().unwrap();
                    let inodeobj = metadata.inodetable.get(&normalfile_filedesc_obj.inode).unwrap();

                    //confirm inode type is mappable
                    match inodeobj {
                        Inode::File(normalfile_inode_obj) => {
                            //if we want to write our changes back to the file the file needs to be open for reading and writing
                            if (flags & MAP_SHARED != 0) && (flags & PROT_WRITE != 0) && (normalfile_filedesc_obj.flags & O_RDWR != 0) {
                                return syscall_error(Errno::EACCES, "mmap", "file descriptor is not open RDWR, but MAP_SHARED and PROT_WRITE are set");
                            }
                            let filesize = normalfile_inode_obj.size;
                            if off < 0 || off > filesize as i64 {
                                return syscall_error(Errno::ENXIO, "mmap", "Addresses in the range [off,off+len) are invalid for the object specified by fildes.");
                            }
                            //because of NaCl's internal workings we must allow mappings to extend past the end of a file
                            let fobjtable = FILEOBJECTTABLE.read().unwrap();
                            let fobj = fobjtable.get(&normalfile_filedesc_obj.inode).unwrap();
                            //we cannot mmap a rust file in quite the right way so we retrieve the fd number from it
                            //this is the system fd number--the number of the lind.<inodenum> file in our host system
                            let fobjfdno = fobj.as_fd_handle_raw_int();


                            interface::libc_mmap(addr, len, prot, flags, fobjfdno, off)
                        }

                        Inode::CharDev(_chardev_inode_obj) => {
                            syscall_error(Errno::EOPNOTSUPP, "mmap", "lind currently does not support mapping character files")
                        }

                        Inode::Dir(_) => {syscall_error(Errno::EACCES, "mmap", "the fildes argument refers to a file whose type is not supported by mmap")}
                    }
                }
                _ => {syscall_error(Errno::EACCES, "mmap", "the fildes argument refers to a file whose type is not supported by mmap")}
            }
        } else {
            syscall_error(Errno::EBADF, "mmap", "invalid file descriptor")
        }
    }

    //------------------------------------MUNMAP SYSCALL------------------------------------
    
    pub fn munmap_syscall(&self, addr: *mut u8, len: usize) -> i32 {
        if len == 0 {syscall_error(Errno::EINVAL, "mmap", "the value of len is 0");}
        //NaCl's munmap implementation actually just writes over the previously mapped data with PROT_NONE
        //This frees all of the resources except page table space, and is put inside safeposix for consistency
        interface::libc_mmap(addr, len, PROT_NONE, MAP_PRIVATE | MAP_ANONYMOUS | MAP_FIXED, -1, 0)
    }

    //------------------------------------FLOCK SYSCALL------------------------------------

    pub fn flock_syscall(&self, fd: i32, operation: i32) -> i32 {
        let fdtable = self.filedescriptortable.read().unwrap();
 
        if let Some(wrappedfd) = fdtable.get(&fd) {
            let filedesc_enum = wrappedfd.read().unwrap();

            let lock = match &*filedesc_enum {
                File(normalfile_filedesc_obj) => {&normalfile_filedesc_obj.advlock}
                Socket(socket_filedesc_obj) => {&socket_filedesc_obj.advlock}
                Stream(stream_filedesc_obj) => {&stream_filedesc_obj.advlock}
                Pipe(pipe_filedesc_obj) => {&pipe_filedesc_obj.advlock}
                Epoll(epoll_filedesc_obj) => {&epoll_filedesc_obj.advlock}
            };
            match operation & (LOCK_SH | LOCK_EX | LOCK_UN) {
                LOCK_SH => {
                    if operation & LOCK_NB == LOCK_NB {
                        //EAGAIN and EWOULDBLOCK are the same
                        if !lock.try_lock_sh() {return syscall_error(Errno::EAGAIN, "flock", "shared lock would block")};
                    } else {
                        lock.lock_sh();
                    }
                }
                LOCK_EX => {
                    if operation & LOCK_NB == LOCK_NB {
                        if !lock.try_lock_ex() {return syscall_error(Errno::EAGAIN, "flock", "exclusive lock would block")};
                    } else {
                        lock.lock_ex();
                    }
                }
                LOCK_UN => {
                    if operation & LOCK_NB == LOCK_NB {
                        lock.unlock();
                    } else {
                        lock.unlock();
                    }
                }
                _ => {return syscall_error(Errno::EINVAL, "flock", "unknown operation");}
            }
            0 //flock has  succeeded!
        } else {
            syscall_error(Errno::ENOENT, "flock", "invalid file descriptor")
        }
    }

    //------------------RMDIR SYSCALL------------------

    pub fn rmdir_syscall(&self, path: &str) -> i32 {
        if path.len() == 0 {return syscall_error(Errno::ENOENT, "rmdir", "Given path is null");}

        let truepath = normpath(convpath(path), self);
        let mut metadata = FS_METADATA.write().unwrap();

        // try to get inodenum of input path and its parent
        match metawalkandparent(truepath.as_path(), Some(&metadata)) {
            (None, ..) => {
                syscall_error(Errno::EEXIST, "rmdir", "Path does not exist")
            }
            (Some(_), None) => { // path exists but parent does not => path is root dir
                syscall_error(Errno::EBUSY, "rmdir", "Cannot remove root directory")
            }
            (Some(inodenum), Some(parent_inodenum)) => {
                let inodeobj = metadata.inodetable.get_mut(&inodenum).unwrap();

                match inodeobj {
                    // make sure inode matches a directory
                    Inode::Dir(dir_obj) => {
                        if dir_obj.linkcount > 3 {return syscall_error(Errno::ENOTEMPTY, "rmdir", "Directory is not empty");}
                        if !is_dir(dir_obj.mode) {panic!("This directory does not have its mode set to S_IFDIR");}

                        // check if dir has write permission
                        if dir_obj.mode as u32 & (S_IWOTH | S_IWGRP | S_IWUSR) == 0 {return syscall_error(Errno::EPERM, "rmdir", "Directory does not have write permission")}
                        
                        // remove entry of corresponding inodenum from inodetable
                        metadata.inodetable.remove(&inodenum).unwrap();
                        
                        if let Inode::Dir(parent_dir) = metadata.inodetable.get_mut(&parent_inodenum).unwrap() {
                            // check if parent dir has write permission
                            if parent_dir.mode as u32 & (S_IWOTH | S_IWGRP | S_IWUSR) == 0 {return syscall_error(Errno::EPERM, "rmdir", "Parent directory does not have write permission")}
                            
                            // remove entry of corresponding filename from filename-inode dict
                            parent_dir.filename_to_inode_dict.remove(&truepath.file_name().unwrap().to_str().unwrap().to_string()).unwrap();
                            parent_dir.linkcount -= 1; // decrement linkcount of parent dir
                        }
                        persist_metadata(&metadata);
                        0 // success
                    }
                    _ => { syscall_error(Errno::ENOTDIR, "rmdir", "Path is not a directory") }
                }
            }
        }
    }

    //------------------RENAME SYSCALL------------------

    pub fn rename_syscall(&self, oldpath: &str, newpath: &str) -> i32 {
        if oldpath.len() == 0 {return syscall_error(Errno::ENOENT, "rename", "Old path is null");}
        if newpath.len() == 0 {return syscall_error(Errno::ENOENT, "rename", "New path is null");}

        let true_oldpath = normpath(convpath(oldpath), self);
        let true_newpath = normpath(convpath(newpath), self);
        let mut metadata = FS_METADATA.write().unwrap();

        // try to get inodenum of old path and its parent
        match metawalkandparent(true_oldpath.as_path(), Some(&metadata)) {
            (None, ..) => {
                syscall_error(Errno::EEXIST, "rename", "Old path does not exist")
            }
            (Some(_), None) => {
                syscall_error(Errno::EBUSY, "rename", "Cannot rename root directory")
            }
            (Some(inodenum), Some(parent_inodenum)) => {
                // make sure file is not moved to another dir 
                // get inodenum for parent of new path
                let (_, new_par_inodenum) = metawalkandparent(true_newpath.as_path(), Some(&metadata));
                // check if old and new paths share parent
                if new_par_inodenum != Some(parent_inodenum) {
                    return syscall_error(Errno::EOPNOTSUPP, "rename", "Cannot move file to another directory");
                }
                
                if let Inode::Dir(parent_dir) = metadata.inodetable.get_mut(&parent_inodenum).unwrap() {
                    // add pair of new path and its inodenum to filename-inode dict
                    parent_dir.filename_to_inode_dict.insert(true_newpath.file_name().unwrap().to_str().unwrap().to_string(), inodenum);

                    // remove entry of old path from filename-inode dict
                    parent_dir.filename_to_inode_dict.remove(&true_oldpath.file_name().unwrap().to_str().unwrap().to_string());
                }
                persist_metadata(&metadata);
                0 // success
            }
        }
    }

    //------------------FTRUNCATE SYSCALL------------------
    
    pub fn ftruncate_syscall(&self, fd: i32, length: isize) -> i32 {
        let fdtable = self.filedescriptortable.read().unwrap();
 
        if let Some(wrappedfd) = fdtable.get(&fd) {

            let filedesc_enum = wrappedfd.read().unwrap();
            let mut mutmetadata = FS_METADATA.write().unwrap();

            match &*filedesc_enum {
                // only proceed when fd references a regular file
                File(normalfile_filedesc_obj) => {
                    let inodenum = normalfile_filedesc_obj.inode;
                    let inodeobj = mutmetadata.inodetable.get_mut(&inodenum).unwrap();

                    match inodeobj {
                        // only proceed when inode matches with a file
                        Inode::File(ref mut normalfile_inode_obj) => {
                            // get file object table with write lock
                            let mut fobjtable = FILEOBJECTTABLE.write().unwrap();
                            
                            let mut fileobject = fobjtable.get_mut(&inodenum).unwrap();
                            let filesize = normalfile_inode_obj.size as isize;
                            
                            // if length is greater than original filesize,
                            // file is extented with null bytes
                            if filesize < length {
                                let blankbytecount = length - filesize;
                                if let Ok(byteswritten) = fileobject.zerofill_at(filesize as usize, blankbytecount as usize) {
                                    if byteswritten != blankbytecount as usize {
                                        panic!("zerofill_at() has failed");
                                    }
                                } else {
                                    panic!("zerofill_at() has failed");
                                }
                            } else { // if length is smaller than original filesize,
                                     // extra data are cut off
                                fileobject.shrink(length as usize);
                            } 
                            persist_metadata(&mutmetadata);
                        }
                        Inode::CharDev(_) => {
                            return syscall_error(Errno::EISDIR, "ftruncate", "The named file is a character driver");
                        }
                        Inode::Dir(_) => {
                            return syscall_error(Errno::EISDIR, "ftruncate", "The named file is a directory");
                        }
                    };
                }
                _ => {
                    return syscall_error(Errno::EINVAL, "ftruncate", "fd does not reference a regular file");
                }
            };
            0 // ftruncate() has succeeded!
        } else { 
            syscall_error(Errno::EBADF, "ftruncate", "fd is not a valid file descriptor")
        }
    }

    //------------------TRUNCATE SYSCALL------------------
    pub fn truncate_syscall(&self, path: &str, length: isize) -> i32 {
        self.ftruncate_syscall(self.open_syscall(path, O_RDWR, S_IRWXA), length)
    }

    //------------------PIPE SYSCALL------------------

    pub fn pipe_syscall(&self, pipefd: &mut PipeArray) -> i32 {

        let mut fdtable = self.filedescriptortable.write().unwrap();

        // get next available pipe number, and set up pipe
        let pipenumber = if let Some(pipeno) = get_next_pipe() {
            pipeno
        } else {
            return syscall_error(Errno::ENFILE, "pipe", "no available pipe number could be found");
        };


        let mut pipetable = PIPE_TABLE.write().unwrap();

        pipetable.insert(pipenumber, interface::RustRfc::new(interface::new_pipe(PIPE_CAPACITY)));
        
        // get an fd for each end of the pipe and set flags to RD_ONLY and WR_ONLY
        // append each to pipefds list

        let flags = [O_RDONLY, O_WRONLY];
        for flag in flags {

            let thisfd = if let Some(fd) = self.get_next_fd(None, Some(&fdtable)) {
                fd
            } else {
                pipetable.remove(&pipenumber).unwrap();
                return syscall_error(Errno::ENFILE, "pipe", "no available file descriptor number could be found");
            };

            let newfd = Pipe(PipeDesc {pipe: pipenumber, flags: flag, advlock: interface::RustRfc::new(interface::AdvisoryLock::new())});
            let wrappedfd = interface::RustRfc::new(interface::RustLock::new(newfd));
            fdtable.insert(thisfd, wrappedfd);

            match flag {
                O_RDONLY => {pipefd.readfd = thisfd;},
                O_WRONLY => {pipefd.writefd = thisfd;},
                _ => panic!("How did you get here."),
            }
        }

      0 // success
    }  
        
    //------------------GETDENTS SYSCALL------------------

    pub fn getdents_syscall(&self, fd: i32, dirp: *mut u8, bufsize: u32)-> i32 {
        
        let mut vec: Vec<(interface::ClippedDirent, Vec<u8>)> = Vec::new();

        // make sure bufsize is at least greater than size of a ClippedDirent struct
        if bufsize <= interface::CLIPPED_DIRENT_SIZE {
            return syscall_error(Errno::EINVAL, "getdents", "Result buffer is too small.");
        }
        
        let fdtable = self.filedescriptortable.read().unwrap();

        if let Some(wrappedfd) = fdtable.get(&fd) { // check if fd is valid
            let mut filedesc_enum = wrappedfd.write().unwrap();
            
            match &mut *filedesc_enum {
                // only proceed when fd represents a file
                File(ref mut normalfile_filedesc_obj) => {
                    let metadata = FS_METADATA.read().unwrap();
                    let inodeobj = metadata.inodetable.get(&normalfile_filedesc_obj.inode).unwrap();

                    match inodeobj {
                        // only proceed when inode is a dir
                        Inode::Dir(dir_inode_obj) => {
                            let position = normalfile_filedesc_obj.position;
                            let mut bufcount = 0;
                            let mut curr_size;
                            let mut count = 0;
                            let mut temp_len = 0;

                            // iterate over filename-inode pairs in dict
                            for (filename, inode) in dir_inode_obj.filename_to_inode_dict.clone().into_iter().skip(position) {
                                // convert filename to a filename vector of u8
                                let mut vec_filename: Vec<u8> = filename.as_bytes().to_vec();
                                vec_filename.push(b'\0'); // make filename null-terminated
                                
                                vec_filename.push(DT_UNKNOWN); // push DT_UNKNOWN as d_type (for now)
                                temp_len = interface::CLIPPED_DIRENT_SIZE + vec_filename.len() as u32; // get length of current filename vector for padding calculation
                                
                                // pad filename vector to the next highest 8 byte boundary
                                for _ in 0..(temp_len + 7) / 8 * 8 - temp_len {
                                    vec_filename.push(00);
                                }
                                
                                // the fixed dirent size and length of filename vector add up to total size
                                curr_size = interface::CLIPPED_DIRENT_SIZE + vec_filename.len() as u32;
                                
                                bufcount += curr_size; // increment bufcount
                                
                                // stop iteration if current bufcount exceeds argument bufsize
                                if bufcount > bufsize {
                                    bufcount = bufcount - curr_size; // decrement bufcount since current element is not actually written
                                    break;
                                }
                                
                                // push properly constructed tuple to vector storing result
                                vec.push((interface::ClippedDirent{d_ino: inode as u64, d_off: bufcount as u64, d_reclen: curr_size as u16}, vec_filename));
                                count += 1;
                            }
                            // update file position
                            normalfile_filedesc_obj.position = interface::rust_min(position + count, dir_inode_obj.filename_to_inode_dict.len());

                            interface::pack_dirents(vec, dirp);
                            bufcount as i32 // return the number of bytes written
                        }
                        _ => {
                            syscall_error(Errno::ENOTDIR, "getdents", "File descriptor does not refer to a directory.")
                        }
                    }
                }
                // raise error when fd represents a socket, pipe, or stream
                _ => {
                    syscall_error(Errno::ESPIPE, "getdents", "Cannot getdents since fd does not refer to a file.")
                }
            }
        } else {
            syscall_error(Errno::EBADF, "getdents", "Invalid file descriptor")
        }
    }

    //------------------------------------GETCWD SYSCALL------------------------------------
    
    pub fn getcwd_syscall(&self, buf: *mut u8, bufsize: u32) -> i32 {
        let cwd = self.cwd.read().unwrap().into_os_string().into_string();

        //+1 foor null terminator
        if bufsize < cwd.len() + 1 {
            return syscall_error(Errno::ERANGE, "getcwd", "the length (in bytes) of the absolute pathname of the current working directory exceeds the given size");
        }

        *buf = cwd;
        0 //getcwd has succeeded!;
    }
}
