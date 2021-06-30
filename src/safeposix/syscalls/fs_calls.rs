// File system related system calls

use crate::interface;

use super::fs_constants::*;
use crate::safeposix::cage::{CAGE_TABLE, Cage, FileDescriptor::*, FileDesc};
use crate::safeposix::filesystem::*;
use super::errnos::*;

impl Cage {

    //------------------OPEN SYSCALL------------------

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
                    let fobjtable = FILEOBJECTTABLE.read().unwrap();
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
                Inode::File(f) => {size = f.size; mode = f.mode; f.refcount += 1}
                Inode::Dir(f) => {size = f.size; mode = f.mode; f.refcount += 1}
                Inode::CharDev(f) => {size = f.size; mode = f.mode; f.refcount += 1}
                _ => {panic!("How did you even manage to open another kind of file like that?");}
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
            let newfd = File(FileDesc {position: position, inode: inodenum, flags: flags & O_RDWRFLAGS});
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

                let effective_mode = S_IFREG as u32 | mode;

                //assert sane mode bits
                if mode & (S_IRWXA | S_FILETYPEFLAGS as u32) != mode {
                    return syscall_error(Errno::EPERM, "mkdir", "Mode bits were not sane");
                }

                let time = interface::timestamp(); //We do a real timestamp now
                let newinode = Inode::Dir(DirectoryInode {
                    size: 0, uid: DEFAULT_UID, gid: DEFAULT_GID,
                    mode: effective_mode, linkcount: 2, refcount: 0,
                    atime: time, ctime: time, mtime: time, 
                    filename_to_inode_dict: init_filename_to_inode_dict(pardirinode)
                });

                let newinodenum = mutmetadata.nextinode;
                mutmetadata.nextinode += 1;
                if let Inode::Dir(parentdir) = mutmetadata.inodetable.get_mut(&pardirinode).unwrap() {
                    parentdir.filename_to_inode_dict.insert(filename, newinodenum);
                } //insert a reference to the file in the parent directory
                mutmetadata.inodetable.insert(newinodenum, newinode);

                persist_metadata(&mutmetadata);
                0 //mknod has succeeded
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

                let effective_mode = S_IFREG as u32 | mode;

                //assert sane mode bits
                if mode & (S_IRWXA | S_FILETYPEFLAGS as u32) != mode {
                    return syscall_error(Errno::EPERM, "mknod", "Mode bits were not sane");
                }
                if mode as i32 & S_IFCHR == 0 {
                    return syscall_error(Errno::EINVAL, "mknod", "only character files are supported");
                }

                let time = interface::timestamp(); //We do a real timestamp now
                let newinode = Inode::CharDev(DeviceInode {
                    size: 0, uid: DEFAULT_UID, gid: DEFAULT_GID,
                    mode: effective_mode, linkcount: 1, refcount: 0,
                    atime: time, ctime: time, mtime: time, dev: devtuple(dev)
                });

                let newinodenum = mutmetadata.nextinode;
                mutmetadata.nextinode += 1;
                if let Inode::Dir(parentdir) = mutmetadata.inodetable.get_mut(&pardirinode).unwrap() {
                    parentdir.filename_to_inode_dict.insert(filename, newinodenum);
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

    //------------------LINK SYSCALL------------------

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
                                0 //link has succeeded
                            }

                            (Some(_), ..) => {syscall_error(Errno::EEXIST, "link", "newpath already exists")}
                        }
                    }

                    Inode::Dir(_) => {syscall_error(Errno::EPERM, "link", "oldpath is a directory")}
                    _ => {panic!("How did you even manage to refer to a pipe/socket using a path?");}
                }
            }
        }
    }

    //------------------UNLINK SYSCALL------------------

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

            //If both the file and the root directory exists
            (Some(inodenum), Some(parentinodenum)) => {
                let inodeobj = mutmetadata.inodetable.get_mut(&inodenum).unwrap();

                let (currefcount, curlinkcount) = match inodeobj {
                    Inode::File(f) => {f.linkcount -= 1; (f.refcount, f.linkcount)},
                    Inode::CharDev(f) => {f.linkcount -= 1; (f.refcount, f.linkcount)},
                    Inode::Dir(_) => {return syscall_error(Errno::EISDIR, "unlink", "cannot unlink directory");},
                    _ => {panic!("How did you even manage to refer to socket or pipe with a path?");},
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
                        let sysfilename = format!("{}{}", FILEDATAPREFIX, inodenum);
                        interface::removefile(sysfilename).unwrap();

                    } //we don't need a separate unlinked flag, we can just check that refcount is 0
                }

                0 //unlink has succeeded
            }

        }
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
            0 //stat has succeeded!
        } else {
            syscall_error(Errno::ENOENT, "stat", "path refers to an invalid file")
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
                        _ => {panic!("A file fd points to a socket or pipe");}
                    }
                }
                Socket(_) => {
                    return syscall_error(Errno::EOPNOTSUPP, "fstat", "we don't support fstat on sockets yet");
                    }
                Stream(_) => {self._stat_alt_helper(statbuf, STREAMINODE, &metadata);}
                Pipe(_) => {self._stat_alt_helper(statbuf, 0xfeef0000, &metadata);}
            }
            0 //fstat has succeeded!
        } else {
            syscall_error(Errno::ENOENT, "fstat", "invalid file descriptor")
        }
    }

    //------------------READ SYSCALL------------------

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
                        _ => {panic!("Wonky file descriptor shenanigains");}
                    }
                }
                Socket(_) => {syscall_error(Errno::EOPNOTSUPP, "read", "recv not implemented yet")}
                Stream(_) => {syscall_error(Errno::EOPNOTSUPP, "read", "reading from stdin not implemented yet")}
                Pipe(pipe_filedesc_obj) => {
                    if is_wronly(pipe_filedesc_obj.flags) {
                        return syscall_error(Errno::EBADF, "read", "specified file not open for reading");
                    }
                    //self._read_from_pipe...
                    syscall_error(Errno::EOPNOTSUPP, "read", "reading from a pipe not implemented yet")
                }
            }
        } else {
            syscall_error(Errno::EBADF, "read", "invalid file descriptor")
        }
    }

    //------------------PREAD SYSCALL------------------
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
                        _ => {panic!("Wonky file descriptor shenanigains");}
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

    //------------------WRITE SYSCALL------------------

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
                                        panic!("Write of blank bytes for pwrite failed!");
                                    }
                                } else {
                                    panic!("Write of blank bytes for pwrite failed!");
                                }
                            }

                            let newposition;
                            if let Ok(byteswritten) = fileobject.writeat(buf, count, position) {
                                //move position forward by the number of bytes we've written
                                normalfile_filedesc_obj.position = position + byteswritten;
                                newposition = normalfile_filedesc_obj.position;
                                if newposition > normalfile_inode_obj.size {
                                    normalfile_inode_obj.size = newposition;
                                } //update file size if necessary
                                persist_metadata(&metadata);

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
                        _ => {panic!("Wonky file descriptor shenanigains");}
                    }
                }
                Socket(_) => {syscall_error(Errno::EOPNOTSUPP, "write", "send not implemented yet")}
                Stream(stream_filedesc_obj) => {
                    //if it's stdout or stderr, print out and we're done
                    if stream_filedesc_obj.stream == 1 || stream_filedesc_obj.stream == 2 {
                        interface::log_from_ptr(buf);
                        count as i32
                    } else {
                        return syscall_error(Errno::EBADF, "write", "specified stream not open for writing");
                    }
                }
                Pipe(pipe_filedesc_obj) => {
                    if is_rdonly(pipe_filedesc_obj.flags) {
                        return syscall_error(Errno::EBADF, "write", "specified pipe not open for writing");
                    }
                    //self._write_to_pipe...
                    syscall_error(Errno::EOPNOTSUPP, "write", "writing to a pipe not implemented yet")
                }
            }
        } else {
            syscall_error(Errno::EBADF, "write", "invalid file descriptor")
        }
    }

    //------------------PWRITE SYSCALL------------------

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
                            } //update file size if necessary
                            persist_metadata(&metadata);

                            retval
                        }

                        Inode::CharDev(char_inode_obj) => {
                            self._write_chr_file(char_inode_obj, buf, count)
                        }

                        Inode::Dir(_) => {
                            syscall_error(Errno::EISDIR, "pwrite", "attempted to write to a directory")
                        }
                        _ => {panic!("Wonky file descriptor shenanigains");}
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

    //------------------LSEEK SYSCALL------------------
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
                        _ => {panic!("Wonky file descriptor shenanigains");}
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
            }
        } else {
            syscall_error(Errno::EBADF, "lseek", "invalid file descriptor")
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
            if mode & newmode == newmode {
                0
            } else {
                syscall_error(Errno::EACCES, "access", "the requested access would be denied to the file")
            }
        } else {
            syscall_error(Errno::ENOENT, "access", "path does not refer to an existing file")
        }
    }

    //------------------CHDIR SYSCALL------------------
    
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

    //------------------MMAP SYSCALL------------------
    
    pub fn mmap_syscall(&self, addr: *mut u8, len: usize, prot: i32, flags: i32, fildes: i32, off: i64) -> i32 {
        if len == 0 {syscall_error(Errno::EINVAL, "mmap", "the value of len is 0");}

        if 0 == flags & (MAP_PRIVATE | MAP_SHARED) {
            syscall_error(Errno::EINVAL, "mmap", "The value of flags is invalid (neither MAP_PRIVATE nor MAP_SHARED is set)");
        }

        if 0 != flags & MAP_ANONYMOUS {
            return interface::libc_mmap(addr, len, prot, flags, 0, -1);
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

                        _ => {syscall_error(Errno::EACCES, "mmap", "the fildes argument refers to a file whose type is not supported by mmap")}
                    }
                }
                _ => {syscall_error(Errno::EACCES, "mmap", "the fildes argument refers to a file whose type is not supported by mmap")}
            }
        } else {
            syscall_error(Errno::EBADF, "mmap", "invalid file descriptor")
        }
    }

    //------------------MUNMAP SYSCALL------------------
    
    pub fn munmap_syscall(&self, addr: *mut u8, len: usize) -> i32 {
        if len == 0 {syscall_error(Errno::EINVAL, "mmap", "the value of len is 0");}
        //NaCl's munmap implementation actually just writes over the previously mapped data with PROT_NONE
        //This frees all of the resources except page table space, and is put inside safeposix for consistency
        interface::libc_mmap(addr, len, PROT_NONE, MAP_PRIVATE | MAP_ANONYMOUS | MAP_FIXED, -1, 0)
    }

}
