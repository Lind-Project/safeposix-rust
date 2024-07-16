//! This module contains all filesystem-related system calls.
//!
//! ## Notes:
//!
//! - These calls are implementations of the [`Cage`] struct in the
//!   [`safeposix`](crate::safeposix) crate. See the
//!   [`safeposix`](crate::safeposix) crate for more information.
//! They have been structed as different modules for better maintainability and
//! related functions. since they are tied to the `Cage` struct This module's
//! rustdoc may turn up empty, thus they have been explicitly listed below for
//! documentation purposes.
//!
//!
//! ## File System Calls
//!
//! Cages have methods for filesystem-related calls. They return a code or an
//! error from the `errno` enum.
//!
//!
//! - [open_syscall](crate::safeposix::cage::Cage::open_syscall)
//! - [mkdir_syscall](crate::safeposix::cage::Cage::mkdir_syscall)
//! - [mknod_syscall](crate::safeposix::cage::Cage::mknod_syscall)
//! - [link_syscall](crate::safeposix::cage::Cage::link_syscall)
//! - [unlink_syscall](crate::safeposix::cage::Cage::unlink_syscall)
//! - [creat_syscall](crate::safeposix::cage::Cage::creat_syscall)
//! - [stat_syscall](crate::safeposix::cage::Cage::stat_syscall)
//! - [fstat_syscall](crate::safeposix::cage::Cage::fstat_syscall)
//! - [statfs_syscall](crate::safeposix::cage::Cage::statfs_syscall)
//! - [fstatfs_syscall](crate::safeposix::cage::Cage::fstatfs_syscall)
//! - [_istatfs_helper](crate::safeposix::cage::Cage::_istatfs_helper)
//! - [read_syscall](crate::safeposix::cage::Cage::read_syscall)
//! - [pread_syscall](crate::safeposix::cage::Cage::pread_syscall)
//! - [write_syscall](crate::safeposix::cage::Cage::write_syscall)
//! - [pwrite_syscall](crate::safeposix::cage::Cage::pwrite_syscall)
//! - [writev_syscall](crate::safeposix::cage::Cage::writev_syscall)
//! - [lseek_syscall](crate::safeposix::cage::Cage::lseek_syscall)
//! - [access_syscall](crate::safeposix::cage::Cage::access_syscall)
//! - [fchdir_syscall](crate::safeposix::cage::Cage::fchdir_syscall)
//! - [chdir_syscall](crate::safeposix::cage::Cage::chdir_syscall)
//! - [dup_syscall](crate::safeposix::cage::Cage::dup_syscall)
//! - [dup2_syscall](crate::safeposix::cage::Cage::dup2_syscall)
//! - [_dup2_helper](crate::safeposix::cage::Cage::_dup2_helper)
//! - [close_syscall](crate::safeposix::cage::Cage::close_syscall)
//! - [_close_helper_inner](crate::safeposix::cage::Cage::_close_helper_inner)
//! - [_close_helper](crate::safeposix::cage::Cage::_close_helper)
//! - [fcntl_syscall](crate::safeposix::cage::Cage::fcntl_syscall)
//! - [ioctl_syscall](crate::safeposix::cage::Cage::ioctl_syscall)
//! - [_chmod_helper](crate::safeposix::cage::Cage::_chmod_helper)
//! - [chmod_syscall](crate::safeposix::cage::Cage::chmod_syscall)
//! - [fchmod_syscall](crate::safeposix::cage::Cage::fchmod_syscall)
//! - [mmap_syscall](crate::safeposix::cage::Cage::mmap_syscall)
//! - [munmap_syscall](crate::safeposix::cage::Cage::munmap_syscall)
//! - [flock_syscall](crate::safeposix::cage::Cage::flock_syscall)
//! - [remove_from_parent_dir](crate::safeposix::cage::Cage::remove_from_parent_dir)
//! - [rmdir_syscall](crate::safeposix::cage::Cage::rmdir_syscall)
//! - [rename_syscall](crate::safeposix::cage::Cage::rename_syscall)
//! - [fsync_syscall](crate::safeposix::cage::Cage::fsync_syscall)
//! - [fdatasync_syscall](crate::safeposix::cage::Cage::fdatasync_syscall)
//! - [sync_file_range_syscall](crate::safeposix::cage::Cage::sync_file_range_syscall)
//! - [ftruncate_syscall](crate::safeposix::cage::Cage::ftruncate_syscall)
//! - [truncate_syscall](crate::safeposix::cage::Cage::truncate_syscall)
//! - [pipe_syscall](crate::safeposix::cage::Cage::pipe_syscall)
//! - [pipe2_syscall](crate::safeposix::cage::Cage::pipe2_syscall)
//! - [getdents_syscall](crate::safeposix::cage::Cage::getdents_syscall)
//! - [getcwd_syscall](crate::safeposix::cage::Cage::getcwd_syscall)
//! - [rev_shm_find_index_by_addr](crate::safeposix::cage::Cage::rev_shm_find_index_by_addr)
//! - [rev_shm_find_addrs_by_shmid](crate::safeposix::cage::Cage::rev_shm_find_addrs_by_shmid)
//! - [search_for_addr_in_region](crate::safeposix::cage::Cage::search_for_addr_in_region)
//! - [shmget_syscall](crate::safeposix::cage::Cage::shmget_syscall)
//! - [shmat_syscall](crate::safeposix::cage::Cage::shmat_syscall)
//! - [shmdt_syscall](crate::safeposix::cage::Cage::shmdt_syscall)
//! - [shmctl_syscall](crate::safeposix::cage::Cage::shmctl_syscall)
//! - [mutex_create_syscall](crate::safeposix::cage::Cage::mutex_create_syscall)
//! - [mutex_destroy_syscall](crate::safeposix::cage::Cage::mutex_destroy_syscall)
//! - [mutex_lock_syscall](crate::safeposix::cage::Cage::mutex_lock_syscall)
//! - [mutex_trylock_syscall](crate::safeposix::cage::Cage::mutex_trylock_syscall)
//! - [mutex_unlock_syscall](crate::safeposix::cage::Cage::mutex_unlock_syscall)
//! - [cond_create_syscall](crate::safeposix::cage::Cage::cond_create_syscall)
//! - [cond_destroy_syscall](crate::safeposix::cage::Cage::cond_destroy_syscall)
//! - [cond_signal_syscall](crate::safeposix::cage::Cage::cond_signal_syscall)
//! - [cond_broadcast_syscall](crate::safeposix::cage::Cage::cond_broadcast_syscall)
//! - [cond_wait_syscall](crate::safeposix::cage::Cage::cond_wait_syscall)
//! - [cond_timedwait_syscall](crate::safeposix::cage::Cage::cond_timedwait_syscall)
//! - [sem_init_syscall](crate::safeposix::cage::Cage::sem_init_syscall)
//! - [sem_wait_syscall](crate::safeposix::cage::Cage::sem_wait_syscall)
//! - [sem_post_syscall](crate::safeposix::cage::Cage::sem_post_syscall)
//! - [sem_destroy_syscall](crate::safeposix::cage::Cage::sem_destroy_syscall)
//! - [sem_getvalue_syscall](crate::safeposix::cage::Cage::sem_getvalue_syscall)
//! - [sem_trywait_syscall](crate::safeposix::cage::Cage::sem_trywait_syscall)
//! - [sem_timedwait_syscall](crate::safeposix::cage::Cage::sem_timedwait_syscall)

#![allow(dead_code)]

// File system related system calls
use super::fs_constants::*;
use super::sys_constants::*;
use crate::interface;
use crate::safeposix::cage::Errno::EINVAL;
use crate::safeposix::cage::{FileDescriptor::*, *};
use crate::safeposix::filesystem::*;
use crate::safeposix::net::NET_METADATA;
use crate::safeposix::shm::*;

impl Cage {
    /// ## ------------------OPEN SYSCALL------------------
    /// ### Description
    ///
    /// The `open_syscall()` creates an open file description that refers to a
    /// file and a file descriptor that refers to that open file description.
    /// The file descriptor is used by other I/O functions to refer to that
    /// file. There are generally two cases which occur when this function
    /// is called. Case 1: If the file to be opened doesn't exist, then a
    /// new file is created at the given location and a new file descriptor is
    /// created. Case 2: If the file already exists, then a few conditions
    /// are checked and based on them, file is updated accordingly.

    /// ### Function Arguments
    ///
    /// The `open_syscall()` receives three arguments:
    /// * `path` - This argument points to a pathname naming the file. For
    ///   example: "/parentdir/file1" represents a file which will be either
    ///   opened if exists or will be created at the given path.
    /// * `flags` - This argument contains the file status flags and file access
    ///   modes which will be alloted to the open file description. The flags
    ///   are combined together using a bitwise-inclusive-OR and the result is
    ///   passed as an argument to the function. Some of the most common flags
    ///   used are: O_CREAT | O_TRUNC | O_RDWR | O_EXCL | O_RDONLY | O_WRONLY,
    ///   with each representing a different file mode.
    /// * `mode` - This represents the permission of the newly created file. The
    ///   general mode used is "S_IRWXA": which represents the read, write, and
    ///   search permissions on the new file.

    /// ### Returns
    ///
    /// Upon successful completion of this call, a file descriptor is returned
    /// which points the file which is opened. Otherwise, errors or panics
    /// are returned for different scenarios.
    ///
    /// ### Errors
    ///
    /// * ENFILE - no available file descriptor number could be found
    /// * ENOENT - tried to open a file that did not exist
    /// * EINVAL - the input flags contain S_IFCHR flag representing a special
    ///   character file
    /// * EPERM - the mode bits for a file are not sane
    /// * ENOTDIR - tried to create a file as a child of something that isn't a
    ///   directory
    /// * EEXIST - the file already exists and O_CREAT and O_EXCL flags were
    ///   passed
    /// * ENXIO - the file is of type UNIX domain socket
    ///
    /// ### Panics
    ///
    /// * If truepath.file_name() returns None or if to_str() fails, causing
    ///   unwrap() to panic.
    /// * If the parent inode does not exist in the inode table, causing
    ///   unwrap() to panic.
    /// * When there is some other issue fetching the file descriptor.
    ///
    /// For more detailed description of all the commands and return values, see
    /// [open(2)](https://man7.org/linux/man-pages/man2/open.2.html)

    // This function is used to create a new File Descriptor Object and return it.
    // This file descriptor object is then inserted into the File Descriptor Table
    // of the associated cage in the open_syscall() function
    fn _file_initializer(&self, inodenum: usize, flags: i32, size: usize) -> FileDesc {
        let position = if 0 != flags & O_APPEND { size } else { 0 };

        // While creating a new FileDescriptor, there are two important things that need
        // to be present: O_RDWRFLAGS:- This flag determines whether the file is
        // opened for reading, writing, or both. O_CLOEXEC - This flag indicates
        // that the file descriptor should be automatically closed during an exec family
        // function. It’s needed for managing file descriptors across different
        // processes, ensuring that they do not unintentionally remain open.
        let allowmask = O_RDWRFLAGS | O_CLOEXEC;
        FileDesc {
            position: position,
            inode: inodenum,
            flags: flags & allowmask,
            advlock: interface::RustRfc::new(interface::AdvisoryLock::new()),
        }
    }

    pub fn open_syscall(&self, path: &str, flags: i32, mode: u32) -> i32 {
        // Check that the given input path is not empty
        if path.len() == 0 {
            return syscall_error(Errno::ENOENT, "open", "given path was null");
        }

        // Retrieve the absolute path from the root directory. The absolute path is then
        // used to validate directory paths while navigating through
        // subdirectories and creating a new file or open existing file at the given
        // location.
        let truepath = normpath(convpath(path), self);

        // Fetch the next file descriptor and its lock write guard to ensure the file
        // can be associated with the file descriptor
        let (fd, guardopt) = self.get_next_fd(None);
        match fd {
            // If the file descriptor is invalid, the return value is always an error with value
            // (ENFILE).
            fd if fd == (Errno::ENFILE as i32) => {
                return syscall_error(
                    Errno::ENFILE,
                    "open_helper",
                    "no available file descriptor number could be found",
                );
            }
            // When the file descriptor is valid, we proceed with performing the remaining checks
            // for open_syscall.
            fd if fd > 0 => {
                // File Descriptor Write Lock Guard
                let fdoption = &mut *guardopt.unwrap();

                // Walk through the absolute path which returns a tuple consisting of inode
                // number of file (if it exists), and inode number of parent (if it exists)
                match metawalkandparent(truepath.as_path()) {
                    // Case 1: When the file doesn't exist but the parent directory exists
                    (None, Some(pardirinode)) => {
                        // Check if O_CREAT flag is not present, then a file can not be created and
                        // error is returned.
                        if 0 == (flags & O_CREAT) {
                            return syscall_error(
                                Errno::ENOENT,
                                "open",
                                "tried to open a file that did not exist, and O_CREAT was not specified",
                            );
                        }

                        // Error is thrown when the input flags contain S_IFCHR flag representing a
                        // special character file.
                        if S_IFCHR == (S_IFCHR & flags) {
                            return syscall_error(Errno::EINVAL, "open", "Invalid value in flags");
                        }

                        // S_FILETYPEFLAGS represents a bitmask that can be used to extract the file
                        // type information from a file's mode. This code is
                        // referenced from Lind-Repy codebase. Here, we are
                        // checking whether the mode bits are sane by ensuring that only valid file
                        // permission bits (S_IRWXA) and file type bits (S_FILETYPEFLAGS) are set.
                        // Else, we return the error.
                        if mode & (S_IRWXA | S_FILETYPEFLAGS as u32) != mode {
                            return syscall_error(Errno::EPERM, "open", "Mode bits were not sane");
                        }

                        let filename = truepath.file_name().unwrap().to_str().unwrap().to_string(); //for now we assume this is sane, but maybe this should be checked later
                        let time = interface::timestamp(); //We do a real timestamp now

                        // S_IFREG is the flag for a regular file, so it's added to the mode to
                        // indicate that the new file being created is a regular file.
                        let effective_mode = S_IFREG as u32 | mode;

                        // Create a new inode of type "File" representing a file and set the
                        // required attributes
                        let newinode = Inode::File(GenericInode {
                            size: 0,
                            uid: DEFAULT_UID,
                            gid: DEFAULT_GID,
                            mode: effective_mode,
                            linkcount: 1, /* because when a new file is created, it has a single
                                           * hard link, which is the directory entry that points
                                           * to this file's inode. */
                            refcount: 1, /* Because a new file descriptor will open and refer to
                                          * this file */
                            atime: time,
                            ctime: time,
                            mtime: time,
                        });

                        // Fetch the next available inode number using the FileSystem MetaData table
                        let newinodenum = FS_METADATA
                            .nextinode
                            .fetch_add(1, interface::RustAtomicOrdering::Relaxed); //fetch_add returns the previous value, which is the inode number we want

                        // Fetch the inode of the parent directory and only proceed when its type is
                        // directory.
                        if let Inode::Dir(ref mut ind) =
                            *(FS_METADATA.inodetable.get_mut(&pardirinode).unwrap())
                        {
                            ind.filename_to_inode_dict.insert(filename, newinodenum);
                            ind.linkcount += 1; // Since the parent is now associated to the new file, its linkcount
                                                // will increment by 1
                            ind.ctime = time; // Here, update the ctime and mtime for the parent directory as well
                            ind.mtime = time;
                        } else {
                            return syscall_error(
                                Errno::ENOTDIR,
                                "open",
                                "tried to create a file as a child of something that isn't a directory",
                            );
                        }
                        // Update the inode table by inserting the newly formed inode mapped with
                        // its inode number.
                        FS_METADATA.inodetable.insert(newinodenum, newinode);
                        log_metadata(&FS_METADATA, pardirinode);
                        log_metadata(&FS_METADATA, newinodenum);

                        // FileObjectTable stores the entries of the currently opened files in the
                        // system Since, a new file is being opened here, an
                        // entry corresponding to that newinode is made in the FileObjectTable
                        // An entry in the table has the following representation:
                        // Key - inode number
                        // Value - Opened file with its size as 0
                        if let interface::RustHashEntry::Vacant(vac) =
                            FILEOBJECTTABLE.entry(newinodenum)
                        {
                            let sysfilename = format!("{}{}", FILEDATAPREFIX, newinodenum);
                            vac.insert(interface::openfile(sysfilename, 0).unwrap());
                            // new file of size 0
                        }

                        // The file object of size 0, associated with the newinode number is
                        // inserted into the FileDescriptorTable associated with the cage using the
                        // guard lock.
                        let _insertval =
                            fdoption.insert(File(self._file_initializer(newinodenum, flags, 0)));
                    }

                    // Case 2: When the file exists (we don't need to look at parent here)
                    (Some(inodenum), ..) => {
                        //If O_CREAT and O_EXCL flags are set in the input parameters,
                        // open_syscall() fails if the file exists.
                        // This is because the check for the existence of the file and the creation
                        // of the file if it does not exist is atomic,
                        // with respect to other threads executing open() naming the same filename
                        // in the same directory with O_EXCL and O_CREAT set.
                        if (O_CREAT | O_EXCL) == (flags & (O_CREAT | O_EXCL)) {
                            return syscall_error(
                                Errno::EEXIST,
                                "open",
                                "file already exists and O_CREAT and O_EXCL were used",
                            );
                        }
                        let size;

                        // Fetch the Inode Object associated with the inode number of the existing
                        // file. There are different Inode types supported
                        // by the open_syscall (i.e., File, Directory, Socket, CharDev).
                        let mut inodeobj = FS_METADATA.inodetable.get_mut(&inodenum).unwrap();
                        match *inodeobj {
                            Inode::File(ref mut f) => {
                                //This is a special case when the input flags contain "O_TRUNC"
                                // flag, This flag truncates the
                                // file size to 0, and the mode and owner are unchanged
                                // and is only used when the file exists and is a regular file
                                if O_TRUNC == (flags & O_TRUNC) {
                                    // Close the existing file object and remove it from the
                                    // FileObject Hashtable using the inodenumber
                                    let entry = FILEOBJECTTABLE.entry(inodenum);
                                    if let interface::RustHashEntry::Occupied(occ) = &entry {
                                        occ.get().close().unwrap();
                                    }

                                    f.size = 0;

                                    // Update the timestamps as well
                                    let latest_time = interface::timestamp();
                                    f.ctime = latest_time;
                                    f.mtime = latest_time;

                                    // Remove the previous file and add a new one of 0 length
                                    if let interface::RustHashEntry::Occupied(occ) = entry {
                                        occ.remove_entry();
                                    }

                                    // The current file is removed from the filesystem
                                    let sysfilename = format!("{}{}", FILEDATAPREFIX, inodenum);
                                    interface::removefile(sysfilename.clone()).unwrap();
                                }

                                // Once the metadata for the file is reset, a new file is inserted
                                // in file system. Also, it is
                                // inserted back to the FileObjectTable and associated with same
                                // inodeNumber representing that the file is currently in open
                                // state.
                                if let interface::RustHashEntry::Vacant(vac) =
                                    FILEOBJECTTABLE.entry(inodenum)
                                {
                                    let sysfilename = format!("{}{}", FILEDATAPREFIX, inodenum);
                                    vac.insert(interface::openfile(sysfilename, f.size).unwrap());
                                }

                                // Update the final size and reference count for the file
                                size = f.size;
                                f.refcount += 1;

                                // Current Implementation for File Truncate: The
                                // previous entry of the file is removed from
                                // the FileObjectTable, with a new file of size
                                // 0 inserted back into the table.
                                // Possible Bug: Why are we not simply adjusting
                                // the file size and pointer of the existing
                                // file?
                            }

                            // When the existing file type is of Directory or Character Device, only
                            // the file size and the reference count is updated.
                            Inode::Dir(ref mut f) => {
                                size = f.size;
                                f.refcount += 1;
                            }
                            Inode::CharDev(ref mut f) => {
                                size = f.size;
                                f.refcount += 1;
                            }

                            // If the existing file type is a socket, error is thrown as socket type
                            // files are not supported by open_syscall
                            Inode::Socket(_) => {
                                return syscall_error(
                                    Errno::ENXIO,
                                    "open",
                                    "file is a UNIX domain socket",
                                );
                            }
                        }

                        // The file object of size 0, associated with the existing inode number is
                        // inserted into the FileDescriptorTable associated with the cage using the
                        // guard lock.
                        let _insertval =
                            fdoption.insert(File(self._file_initializer(inodenum, flags, size)));
                    }

                    // Case 3: When neither the file directory nor the parent directory exists
                    (None, None) => {
                        // O_CREAT flag is used to create a file if it doesn't exist.
                        // If this flag is not present, then a file can not be created and error is
                        // returned.
                        if 0 == (flags & O_CREAT) {
                            return syscall_error(
                                Errno::ENOENT,
                                "open",
                                "tried to open a file that did not exist, and O_CREAT was not specified",
                            );
                        }
                        // O_CREAT flag is set but the path doesn't exist, so return an error with a
                        // different message string.
                        return syscall_error(Errno::ENOENT, "open", "a directory component in pathname does not exist or is a dangling symbolic link");
                    }
                }

                // Once all the updates are done, the file descriptor value is returned
                fd
            }
            // Panic when there is some other issue fetching the file descriptor.
            _ => {
                panic!("File descriptor couldn't be fetched!");
            }
        }
    }

    /// ### Description
    ///
    /// The `mkdir_syscall()` creates a new directory named by the path name
    /// pointed to by a path as the input parameter in the function.
    /// The mode of the new directory is initialized from the "mode" provided as
    /// the input parameter in the function. The newly created directory is
    /// empty with size 0 and is associated with a new inode of type "DIR".
    /// On successful completion, the timestamps for both the newly formed
    /// directory and its parent are updated along with their linkcounts.

    /// ### Arguments
    ///
    /// * `path` - This represents the path at which the new directory will be
    ///   created. For example: `/parentdir/dir` represents the new directory
    ///   name as `dir`, which will be created at this path (`/parentdir/dir`).
    /// * `mode` - This represents the permission of the newly created
    ///   directory. The general mode used is `S_IRWXA`: which represents the
    ///   read, write, and search permissions on the new directory.
    ///
    /// ### Returns
    ///
    /// Upon successful creation of the directory, 0 is returned.
    ///
    /// ### Errors
    ///
    /// * ENOENT - if given path was null or the parent directory does not exist
    ///   in the inode table.
    /// * EPERM - if mode bits were not set.
    /// * EEXIST - if a directory with the same name already exists at the given
    ///   path.
    ///
    /// ### Panics
    ///
    /// * If truepath.file_name() returns None or if to_str() fails, causing
    ///   unwrap() to panic.
    /// * If the parent inode does not exist in the inode table, causing
    ///   unwrap() to panic.
    /// * If the code execution reaches the unreachable!() macro, indicating a
    ///   logical inconsistency in the program.
    ///
    /// for more detailed description of all the commands and return values, see
    /// [mkdir(2)](https://man7.org/linux/man-pages/man2/mkdir.2.html)
    pub fn mkdir_syscall(&self, path: &str, mode: u32) -> i32 {
        // Check that the given input path is not empty
        if path.len() == 0 {
            return syscall_error(Errno::ENOENT, "mkdir", "given path was null");
        }

        // Store the FileMetadata into a helper variable which is used for fetching the
        // metadata of a given inode from the Inode Table.
        let metadata = &FS_METADATA;

        // Retrieve the absolute path from the root directory. The absolute path is then
        // used to validate directory paths while navigating through
        // subdirectories and establishing new directory at the given location.
        let truepath = normpath(convpath(path), self);

        // Walk through the absolute path which returns a tuple consisting of inode
        // number of file (if it exists), and inode number of parent (if it exists)
        match metawalkandparent(truepath.as_path()) {
            // Case 1: When neither the file directory nor the parent directory exists
            (None, None) => syscall_error(
                Errno::ENOENT,
                "mkdir",
                "a directory component in pathname does not exist or is a dangling symbolic link",
            ),

            // Case 2: When the file doesn't exist but the parent directory exists
            (None, Some(pardirinode)) => {
                let filename = truepath.file_name().unwrap().to_str().unwrap().to_string(); //for now we assume this is sane, but maybe this should be checked later

                let effective_mode = S_IFDIR as u32 | mode;
                // Check for the condition if the mode bits are correct and have the required
                // permissions to create a directory
                if mode & (S_IRWXA | S_FILETYPEFLAGS as u32) != mode {
                    return syscall_error(Errno::EPERM, "mkdir", "Mode bits were not sane");
                }

                // Fetch the next available inode number using the FileSystem MetaData table
                // Create a new inode of type "Dir" representing a directory and set the
                // required attributes
                let newinodenum = FS_METADATA
                    .nextinode
                    .fetch_add(1, interface::RustAtomicOrdering::Relaxed); //fetch_add returns the previous value, which is the inode number we want
                let time = interface::timestamp(); //We do a real timestamp now
                let newinode = Inode::Dir(DirectoryInode {
                    size: 0, //initial size of a directory is 0 as it is empty
                    uid: DEFAULT_UID,
                    gid: DEFAULT_GID,
                    mode: effective_mode,
                    linkcount: 3, /* because of the directory name(.), itself, and reference to
                                   * the parent directory(..) */
                    refcount: 0, //because no file descriptors are pointing to it currently
                    atime: time,
                    ctime: time,
                    mtime: time,
                    filename_to_inode_dict: init_filename_to_inode_dict(newinodenum, pardirinode), /* Establish a mapping between the newly created inode and the parent directory inode for easy retrieval and linking */
                });

                // Insert a reference to the file in the parent directory and update the inode
                // attributes Fetch the inode of the parent directory and only
                // proceed when its type is directory.
                if let Inode::Dir(ref mut parentdir) =
                    *(metadata.inodetable.get_mut(&pardirinode).unwrap())
                {
                    parentdir
                        .filename_to_inode_dict
                        .insert(filename, newinodenum);
                    parentdir.linkcount += 1; // Since the parent is now associated to the new directory, its linkcount will
                                              // increment by 1
                    parentdir.ctime = time; // Here, update the ctime and mtime for the parent directory as well
                    parentdir.mtime = time;
                } else {
                    unreachable!();
                }
                // Update the inode table by inserting the newly formed inode mapped with its
                // inode number.
                metadata.inodetable.insert(newinodenum, newinode);
                log_metadata(&metadata, pardirinode);
                log_metadata(&metadata, newinodenum);

                // Return 0 when mkdir has succeeded
                0
            }

            // Case 3: When the file directory name already exists, then return the error.
            (Some(_), ..) => syscall_error(
                Errno::EEXIST,
                "mkdir",
                "pathname already exists, cannot create directory",
            ),
        }
    }

    /// ## ------------------MKNOD SYSCALL------------------
    /// ### Description
    ///
    /// The `mknod_syscall()` creates a filesystem node (file, device special
    /// file or pipe) named by a path as the input parameter.
    /// The file type and the permissions of the new file are initialized from
    /// the "mode" provided as the input parameter.
    /// There are 5 different file types: S_IFREG, S_IFCHR, S_IFBLK, S_IFIFO, or
    /// S_IFSOCK representing a regular file, character special file, block
    /// special file, FIFO (named pipe), or UNIX domain socket,
    /// respectively. The newly created file is empty with size 0.
    /// On successful completion, the timestamps for both the newly created file
    /// and its parent are updated along with their linkcounts.
    ///
    /// ### Function Arguments
    ///
    /// The `mknod_syscall()` receives three arguments:
    /// * `path` - This argument points to a pathname naming the file.
    /// For example: "/parentdir/file" represents the new file name as "file",
    /// which will be created at this path (/parentdir/file).
    ///
    /// * `mode` - The mode argument specifies both the permissions to use and
    ///   the
    /// type of node to be created. It is a combination (using bitwise OR) of
    /// one of the file types and the permissions for the new node.
    /// FileType - In LIND, we have only implemented the file type of "Character
    /// Device" represented by S_IFCHR flag.
    /// FilePermission - The general permission mode used is "S_IRWXA": which
    /// represents the read, write, and search permissions on the new file.
    /// The final file mode is represented by the bitwise-OR of FileType and
    /// FilePermission Flags.
    ///
    /// * `dev` - It is a configuration-dependent specification of a character
    ///   or
    /// block I/O device. If mode does not indicate a block special or character
    /// special device, dev is ignored.
    /// Since "CharDev" is the only supported type, 'dev' is represented using
    /// makedev() function; that returns a formatted device number   
    /// For example: "makedev(&DevNo { major: majorId, minor: minorId })"
    /// accepts a Device Number that consists of a MajorID, identifying the
    /// class of the device, and a minor ID, identifying a specific instance
    /// of a device in that class.
    ///
    /// ### Returns
    ///
    /// Upon successful creation of the file, 0 is returned.
    /// Otherwise, errors or panics are returned for different scenarios.
    ///
    /// ### Errors
    ///
    /// * `ENOENT` - occurs when a directory component in the absolute path does
    /// not exist
    /// * `EPERM` - the mode bits for the new file are not sane
    /// * `EINVAL` - when any other file type (regular, socket, block, fifo)
    ///   instead
    /// of character file type is passed
    /// * `EEXIST` - when the file to be created already exists
    ///
    /// ### Panics
    ///
    /// We don't have panics for mknod_syscall() as of now.
    ///
    /// For more detailed description of all the commands and return values, see
    /// [mknod(2)](https://man7.org/linux/man-pages/man2/mknod.2.html)
    pub fn mknod_syscall(&self, path: &str, mode: u32, dev: u64) -> i32 {
        // Return an error if the provided path is empty
        if path.len() == 0 {
            return syscall_error(Errno::ENOENT, "mknod", "given path was null");
        }
        // Retrieve the absolute path from the root directory. The absolute path is
        // then used to validate directory paths while navigating through
        // subdirectories and establishing new directory at the given location.
        let truepath = normpath(convpath(path), self);

        // Store the FileMetadata into a helper variable which is used for fetching
        // the metadata of a given inode from the Inode Table.
        let metadata = &FS_METADATA;

        // Walk through the absolute path which returns a tuple consisting of inode
        // number of file (if it exists), and inode number of parent (if it exists)
        match metawalkandparent(truepath.as_path()) {
            // Case: When the file doesn't exist but the parent directory exists
            (None, Some(pardirinode)) => {
                // for now we assume this is sane, but maybe this should be checked later
                let filename = truepath.file_name().unwrap().to_str().unwrap().to_string();

                // S_FILETYPEFLAGS represents a bitmask that can be used to extract
                // the file type information from a file's mode.
                // This code is referenced from Lind-Repy codebase.
                // Here, we are checking whether the mode bits are sane by ensuring
                // that only valid file permission bits (S_IRWXA) and file type bits
                // (S_FILETYPEFLAGS) are set. Else, we return the error.
                if mode & (S_IRWXA | S_FILETYPEFLAGS as u32) != mode {
                    return syscall_error(Errno::EPERM, "mknod", "Mode bits were not sane");
                }

                // As of now, the only file type in LIND supported by mknod_syscall
                // is "Char Device" represented by S_IFCHR flag.
                // In order to check for Char file type, a bitwise-AND operation for
                // S_IFCHR flag is performed with the "mode" bits and an error is returned
                // when the result is 0 denoting the support for only character files.
                if mode as i32 & S_IFCHR == 0 {
                    return syscall_error(
                        Errno::EINVAL,
                        "mknod",
                        "only character files are supported",
                    );
                }
                // New Inode of type CharDev is created with file size 0
                let time = interface::timestamp(); // We do a real timestamp now
                let newinode = Inode::CharDev(DeviceInode {
                    size: 0,
                    uid: DEFAULT_UID,
                    gid: DEFAULT_GID,
                    mode: mode,
                    linkcount: 1,
                    refcount: 0,
                    atime: time,
                    ctime: time,
                    mtime: time,
                    dev: devtuple(dev),
                });

                // fetch_add returns the previous value, which is the inode number we want
                let newinodenum = FS_METADATA
                    .nextinode
                    .fetch_add(1, interface::RustAtomicOrdering::Relaxed);

                // Insert a reference to the file in the parent directory and update
                // the inode attributes.
                // Fetch the inode of the parent directory and only proceed when its
                // type is directory.
                if let Inode::Dir(ref mut parentdir) =
                    *(FS_METADATA.inodetable.get_mut(&pardirinode).unwrap())
                {
                    parentdir
                        .filename_to_inode_dict
                        .insert(filename, newinodenum);
                    parentdir.linkcount += 1;
                    // Update the ctime and mtime for the parent directory as well
                    // since the new file is linked with it.
                    parentdir.ctime = time;
                    parentdir.mtime = time;
                }

                // Update the inode table by inserting the newly formed inode mapped
                // with its inode number.
                metadata.inodetable.insert(newinodenum, newinode);
                log_metadata(metadata, pardirinode);
                log_metadata(metadata, newinodenum);
                0 // mknod has succeeded
            }

            // Case: When the file directory name already exists, then return the error.
            (Some(_), ..) => syscall_error(
                Errno::EEXIST,
                "mknod",
                "pathname already exists, cannot create device file",
            ),

            // Case: When neither the file directory nor the parent directory exists
            (None, None) => syscall_error(
                Errno::ENOENT,
                "mknod",
                "a directory component in pathname does not exist or is a dangling symbolic link",
            ),
        }
    }

    /// ## ------------------LINK SYSCALL------------------
    /// ### Description
    ///
    /// The `link_syscall()` creates a new link (directory entry) for the
    /// existing file represented by oldpath and increments its link count
    /// by one. Since, we are creating hard links between the files, both of
    /// them must exist on the same file system. Both the old and the new
    /// link share equal access and rights to the underlying object.
    /// On successful completion, the timestamps for both the newly created file
    /// and its parent are updated along with their linkcounts.
    /// If it fails, no link is created and the link count of the file remains
    /// unchanged.
    ///
    /// ### Function Arguments
    ///
    /// The `link_syscall()` receives two arguments:
    /// * `oldpath` - This argument points to a pathname naming an existing
    ///   file.
    /// * `newpath` - This argument points to a pathname naming the new
    ///   directory
    /// entry and the link to be created.
    ///
    /// ### Returns
    ///
    /// Upon successful linking of the files, 0 is returned.
    /// Otherwise, −1 is returned, no link is created, and errno is set to
    /// indicate the error.
    ///
    /// ### Errors
    ///
    /// * `ENOENT` - The oldpath or newpath argument is a null pathname;
    /// a component of either path prefix does not exist; or the file
    /// named by oldpath does not exist.
    /// * `EPERM` - The file named by oldpath is a directory; current
    /// implementation probibits links to directories.
    /// * `EEXIST` - The link named by newpath already exists
    ///
    /// ### Panics
    ///
    /// * If the parent inode does not exist in the inode table, causing
    ///   unwrap() to panic.
    /// * If the parent inode is not of the type `directory`, causing code to
    ///   panic.
    ///
    /// For more detailed description of all the commands and return values, see
    /// [link(2)](https://man7.org/linux/man-pages/man2/link.2.html)
    pub fn link_syscall(&self, oldpath: &str, newpath: &str) -> i32 {
        // Return an error if the provided oldpath is empty
        if oldpath.len() == 0 {
            return syscall_error(Errno::ENOENT, "link", "given oldpath was null");
        }
        // Return an error if the provided newpath is empty
        if newpath.len() == 0 {
            return syscall_error(Errno::ENOENT, "link", "given newpath was null");
        }
        // Retrieve the absolute path from the root directory for both oldpath and
        // newpath. The absolute path is then used to validate directory paths
        // while navigating through subdirectories.
        let trueoldpath = normpath(convpath(oldpath), self);
        let truenewpath = normpath(convpath(newpath), self);
        //for now we assume this is sane, but maybe this should be checked later
        let filename = truenewpath
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();

        // TODO BUG: Man-page contains a check for the directories in the path
        // to have search/read permissions, which is not implemented in this syscall.

        // Walk through the absolute path for the oldpath file which returns the inode
        // number of file (if it exists).
        match metawalk(trueoldpath.as_path()) {
            // Case: If the directory component doesn't exist, return an error.
            None => syscall_error(
                Errno::ENOENT,
                "link",
                "a directory component in pathname does not exist",
                // Currently, we don't support the symbolic links
            ),
            // Case: Get the inode number and increment the link count of the existing
            // directory component i.e., (File, CharDev, and Socket).
            // "Directory" type is not supported for this implementation.
            Some(inodenum) => {
                // Get the mutable instance of the inode object from the FileMetaData table.
                let mut inodeobj = FS_METADATA.inodetable.get_mut(&inodenum).unwrap();

                // Match the inode object with the correct inode type and increment link count
                match *inodeobj {
                    // Directory type inode is not supported for linking, so return an error.
                    Inode::Dir(_) => {
                        return syscall_error(Errno::EPERM, "link", "oldpath is a directory")
                    }

                    Inode::File(ref mut normalfile_inode_obj) => {
                        normalfile_inode_obj.linkcount += 1; //add link to
                                                             // inode
                    }

                    Inode::CharDev(ref mut chardev_inode_obj) => {
                        chardev_inode_obj.linkcount += 1; //add link to inode
                    }

                    // The Sockets only have an inode if they are a unix type
                    // socket which has a corresponding inode. Regular sockets
                    // do not have inodes.
                    Inode::Socket(ref mut socket_inode_obj) => {
                        socket_inode_obj.linkcount += 1; //add link to inode
                    }
                }

                // the mutable reference to the inode has to be dropped because
                //`log_metadata` will need to acquire an immutable reference to
                // the same inode
                drop(inodeobj);

                // Walk the newpath and once the parent directory inode is found, insert a
                // reference of this oldpath inode in the inode table
                let retval = match metawalkandparent(truenewpath.as_path()) {
                    // If both the file and the parent doesn't exist, newpath can't be created
                    (None, None) => {
                        syscall_error(Errno::ENOENT, "link", "newpath cannot be created")
                    }

                    // If the newpath exists, linking can't be perfomed and an error is returned.
                    (Some(_), ..) => syscall_error(Errno::EEXIST, "link", "newpath already exists"),

                    // If the parent directory inode exists, make a reference of the oldpath inode
                    // in the parent directory to make a link between the two directory paths.
                    (None, Some(pardirinode)) => {
                        // Get the mutable instance of the parent inode object
                        let mut parentinodeobj =
                            FS_METADATA.inodetable.get_mut(&pardirinode).unwrap();
                        //insert a reference to the inode in the parent directory
                        if let Inode::Dir(ref mut parentdirinodeobj) = *parentinodeobj {
                            parentdirinodeobj
                                .filename_to_inode_dict
                                .insert(filename, inodenum);
                            // Increment the link count of the parent inode as well because
                            // when a link is created, a new directory entry is added to
                            // the parent directory of the new link.
                            parentdirinodeobj.linkcount += 1;
                            //drop the mutable instance of the parent inode object
                            drop(parentinodeobj);
                            log_metadata(&FS_METADATA, pardirinode);
                            log_metadata(&FS_METADATA, inodenum);
                        } else {
                            // If the parent inode is not of type "Directory", panic occurs.
                            panic!("Parent directory was not a directory!");
                        }
                        // If the linking is successful, 0 is returned.
                        0
                    }
                };

                // If the linking fails, an error with a value < 0 is returned from above.
                // The following cases lead to the failing of the linking of files:
                // 1. When both the file and the parent doesn't exist, newpath can't be created
                // 2. When the the parent inode is not of type "Directory".
                // 3. When the newpath already exists.
                // So, we revert the link count updates made to the oldpath inode.
                if retval != 0 {
                    // Fetch the inode object from the FileMetadata Table
                    let mut inodeobj = FS_METADATA.inodetable.get_mut(&inodenum).unwrap();

                    // Match the relevant inode object type and decrement link count
                    match *inodeobj {
                        Inode::File(ref mut normalfile_inode_obj) => {
                            normalfile_inode_obj.linkcount -= 1;
                        }

                        Inode::CharDev(ref mut chardev_inode_obj) => {
                            chardev_inode_obj.linkcount -= 1;
                        }

                        Inode::Socket(ref mut socket_inode_obj) => {
                            socket_inode_obj.linkcount -= 1;
                        }

                        Inode::Dir(_) => {
                            panic!("Known non-directory file has been replaced with a directory!");
                        }
                    }
                }

                return retval;
            }
        }
    }

    /// ## ------------------UNLINK SYSCALL------------------
    /// ### Description
    ///
    /// The `unlink_syscall()` removes a link to a file. It removes the link
    /// named by the pathname pointed to by path and decrements the link
    /// count of the file referenced by the link.
    /// If that name was the last link to a file and no processes have the file
    /// open, the file is deleted and the space it was using is made
    /// available for reuse. If the name was the last link to a file but any
    /// processes still have the file open, the file will remain in
    /// existence until the last file descriptor referring to it is closed.
    /// On successful completion, the timestamp for the parent directory is
    /// updated along with its linkcounts.
    ///
    /// ### Function Arguments
    ///
    /// The `unlink_syscall()` receives one argument:
    /// * `path` - This argument points to a pathname which needs to be unlinked
    ///
    /// ### Returns
    ///
    /// Upon successful unlinking of the file, 0 is returned.
    /// Otherwise, −1 is returned, and errno is set to indicate the error.
    ///
    /// ### Errors
    ///
    /// * `ENOENT` - The path argument is a null pathname;
    /// a component of path prefix does not exist; or the file
    /// named by oldpath does not exist.
    /// * `EISDIR` - When the unlinking is done on a directory
    ///
    /// ### Panics
    ///
    /// * If the parent inode does not exist in the inode table, causing
    ///   unwrap() to panic.
    ///
    /// For more detailed description of all the commands and return values, see
    /// [unlink(2)](https://man7.org/linux/man-pages/man2/unlink.2.html)
    pub fn unlink_syscall(&self, path: &str) -> i32 {
        // Return an error if the provided path is empty
        if path.len() == 0 {
            return syscall_error(Errno::ENOENT, "unlink", "given path was null");
        }
        // Retrieve the absolute path from the root directory for the given path.
        // The absolute path is then used to validate directory paths while navigating
        // through subdirectories.
        let truepath = normpath(convpath(path), self);

        // Walk through the absolute path which returns a tuple consisting of inode
        // number of file (if it exists), and inode number of parent (if it exists)
        match metawalkandparent(truepath.as_path()) {
            // Return an error if the given file does not exist
            (None, ..) => syscall_error(Errno::ENOENT, "unlink", "path does not exist"),

            // If the file exists but has no parent, it's the root directory
            // No unlinking is done on the root, and an error is returned
            (Some(_), None) => {
                syscall_error(Errno::EISDIR, "unlink", "cannot unlink root directory")
            }

            // If both the file and the parent directory exists
            (Some(inodenum), Some(parentinodenum)) => {
                // Get the mutable instance of the file from the Inode table
                let mut inodeobj = FS_METADATA.inodetable.get_mut(&inodenum).unwrap();

                // For the inode object, we update 4 parameters:
                // reference count: refers to the active file descriptors pointing to the
                // file.
                // link count: refers to the number of hard links pointing to the file.
                // linkcount is decremented by 1 for all inode types except "Dir" type.
                // file object: indicates whether the inode being unlinked has an associated
                // file object. This is relevant for managing the physical deletion of the file
                // data from the filesystem. It is only "true" for "File" type inode.
                // log: indicates whether the FileMetaData will be updated for the inode.
                let (currefcount, curlinkcount, has_fobj, log) = match *inodeobj {
                    Inode::Dir(_) => {
                        // Unlinking of a directory is not supported
                        return syscall_error(Errno::EISDIR, "unlink", "cannot unlink directory");
                    }
                    Inode::File(ref mut f) => {
                        // "File" type inode has an associated File Object, so is set to "True"
                        f.linkcount -= 1;
                        (f.refcount, f.linkcount, true, true)
                    }
                    Inode::CharDev(ref mut f) => {
                        f.linkcount -= 1;
                        (f.refcount, f.linkcount, false, true)
                    }
                    Inode::Socket(ref mut f) => {
                        f.linkcount -= 1;
                        // Sockets only exist as long as the cages using them are running.
                        // After these cages are closed, no changes to sockets' inodes
                        // need to be persisted, thus using log is unnecessary and is set to "false"
                        (f.refcount, f.linkcount, false, false)
                    }
                }; //count current number of links and references

                drop(inodeobj);

                // Once the link count for the file has been decremented, we need to remove the
                // reference of file from the parent directory. If the removal is successful,
                // 0 is returned, otherwise an error with value!=0 is returned by the function.
                let removal_result = Self::remove_from_parent_dir(parentinodenum, &truepath);
                if removal_result != 0 {
                    return removal_result;
                }

                // When the file's link count becomes 0 (no hard links present),
                // we check for two scenarios:
                // If the reference count is 0 (no open file descriptors) pointing to
                // the file, then we remove the file from filesystem and free the space.
                // If the reference count is > 0, then file contents are not removed
                // from the system.
                if curlinkcount == 0 {
                    // Remove the file from the system when no references to the file
                    // exists.
                    if currefcount == 0 {
                        // remove the reference of the inode from the inodetable
                        FS_METADATA.inodetable.remove(&inodenum);
                        // only "File" type inode has this flag set to "true",
                        // so, the file is removed from the FileSystem
                        if has_fobj {
                            // FILEDATAPREFIX represents the common prefix of the name
                            // of the file which combined with the inode number represents
                            // a unique entity. It stores the data of the inode object.
                            // Since the file is of no use, we are removing its entry
                            // from the system.
                            let sysfilename = format!("{}{}", FILEDATAPREFIX, inodenum);
                            interface::removefile(sysfilename).unwrap();
                        }
                    }
                }
                // Remove any domain socket paths associated with the file
                NET_METADATA.domsock_paths.remove(&truepath);

                // the log boolean will be false if we are working on a domain socket
                if log {
                    log_metadata(&FS_METADATA, parentinodenum);
                    log_metadata(&FS_METADATA, inodenum);
                }
                0 //unlink has succeeded
            }
        }
    }

    /// ## ------------------CREAT SYSCALL------------------
    /// ### Description
    ///
    /// The `creat_syscall()` is similar to `open_syscall()` with the "flags"
    /// parameter for open_syscall set to representing create, truncate or write
    /// only for the file. It simplifies the process of creating a new file or
    /// truncating an existing one by combining the O_CREAT, O_TRUNC, and
    /// O_WRONLY flags.
    /// There are generally two cases which occur when this syscall happens:
    /// Case 1: If the file to be opened doesn't exist, then due to O_CREAT
    /// flag, a new file is created at the given location and a new file
    /// descriptor is created and returned.
    /// Case 2: If the file already exists, then due to O_TRUNC flag, the file
    /// size gets reduced to 0, and the existing file descriptor is returned.
    ///
    /// ### Function Arguments
    ///
    /// The `creat_syscall()` receives two arguments:
    /// * `path` - This argument points to a pathname naming the file. For
    ///   example: "/parentdir/file1" represents a file which will be either
    ///   opened if exists or will be created at the given path.
    /// * `mode` - This represents the permission of the newly created file. The
    ///   general mode used is "S_IRWXA": which represents the read, write, and
    ///   search permissions on the new file.
    ///
    /// ### Returns
    ///
    /// Upon successful completion of this call, a file descriptor is returned
    /// which points the file which is opened. Otherwise, errors or panics
    /// are returned for different scenarios.
    ///
    /// ### Errors
    ///
    /// * ENFILE - no available file descriptor number could be found
    /// * ENOENT - tried to open a file that did not exist
    /// * EPERM - the mode bits for a file are not sane
    /// * ENOTDIR - tried to create a file as a child of something that isn't a
    ///   directory
    /// * EEXIST - the given file already exists
    /// * ENXIO - the file is of type UNIX domain socket
    ///
    /// ### Panics
    ///
    /// * If truepath.file_name() returns None or if to_str() fails, causing
    ///   unwrap() to panic.
    /// * If the parent inode does not exist in the inode table, causing
    ///   unwrap() to panic.
    /// * When there is some other issue fetching the file descriptor.
    ///
    /// For more detailed description of all the commands and return values, see
    /// [creat(3p)](https://man7.org/linux/man-pages/man3/creat.3p.html)
    pub fn creat_syscall(&self, path: &str, mode: u32) -> i32 {
        // These flags represent that the given file is either newly created
        // (if it doesn't exist) or truncated to zero length (if it does exist),
        // and it is opened for write-only access.
        self.open_syscall(path, O_CREAT | O_TRUNC | O_WRONLY, mode)
    }

    //------------------------------------STAT SYSCALL------------------------------------

    pub fn stat_syscall(&self, path: &str, statbuf: &mut StatData) -> i32 {
        let truepath = normpath(convpath(path), self);

        //Walk the file tree to get inode from path
        if let Some(inodenum) = metawalk(truepath.as_path()) {
            let inodeobj = FS_METADATA.inodetable.get(&inodenum).unwrap();

            //populate those fields in statbuf which depend on things other than the inode
            // object
            statbuf.st_dev = FS_METADATA.dev_id;
            statbuf.st_ino = inodenum;

            //delegate the rest of populating statbuf to the relevant helper
            match &*inodeobj {
                Inode::File(f) => {
                    Self::_istat_helper(&f, statbuf);
                }
                Inode::CharDev(f) => {
                    Self::_istat_helper_chr_file(&f, statbuf);
                }
                Inode::Socket(f) => {
                    Self::_istat_helper_sock(&f, statbuf);
                }
                Inode::Dir(f) => {
                    Self::_istat_helper_dir(&f, statbuf);
                }
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

    fn _istat_helper_sock(inodeobj: &SocketInode, statbuf: &mut StatData) {
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

    //Streams and pipes don't have associated inodes so we populate them from
    // mostly dummy information
    fn _stat_alt_helper(&self, statbuf: &mut StatData, inodenum: usize) {
        statbuf.st_dev = FS_METADATA.dev_id;
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

    //------------------------------------FSTAT SYSCALL------------------------------------

    pub fn fstat_syscall(&self, fd: i32, statbuf: &mut StatData) -> i32 {
        let checkedfd = self.get_filedescriptor(fd).unwrap();
        let unlocked_fd = checkedfd.read();
        if let Some(filedesc_enum) = &*unlocked_fd {
            //Delegate populating statbuf to the relevant helper depending on the file
            // type. First we check in the file descriptor to handle sockets,
            // streams, and pipes, and if it is a normal file descriptor we
            // handle regular files, dirs, and char files based on the
            // information in the inode.
            match filedesc_enum {
                File(normalfile_filedesc_obj) => {
                    let inode = FS_METADATA
                        .inodetable
                        .get(&normalfile_filedesc_obj.inode)
                        .unwrap();

                    //populate those fields in statbuf which depend on things other than the inode
                    // object
                    statbuf.st_ino = normalfile_filedesc_obj.inode;
                    statbuf.st_dev = FS_METADATA.dev_id;

                    match &*inode {
                        Inode::File(f) => {
                            Self::_istat_helper(&f, statbuf);
                        }
                        Inode::CharDev(f) => {
                            Self::_istat_helper_chr_file(&f, statbuf);
                        }
                        Inode::Socket(f) => {
                            Self::_istat_helper_sock(&f, statbuf);
                        }
                        Inode::Dir(f) => {
                            Self::_istat_helper_dir(&f, statbuf);
                        }
                    }
                }
                Socket(_) => {
                    return syscall_error(
                        Errno::EOPNOTSUPP,
                        "fstat",
                        "we don't support fstat on sockets yet",
                    );
                }
                Stream(_) => {
                    self._stat_alt_helper(statbuf, STREAMINODE);
                }
                Pipe(_) => {
                    self._stat_alt_helper(statbuf, 0xfeef0000);
                }
                Epoll(_) => {
                    self._stat_alt_helper(statbuf, 0xfeef0000);
                }
            }
            0 //fstat has succeeded!
        } else {
            syscall_error(Errno::EBADF, "fstat", "invalid file descriptor")
        }
    }

    //------------------------------------STATFS SYSCALL------------------------------------

    pub fn statfs_syscall(&self, path: &str, databuf: &mut FSData) -> i32 {
        let truepath = normpath(convpath(path), self);

        //Walk the file tree to get inode from path
        if let Some(inodenum) = metawalk(truepath.as_path()) {
            let _inodeobj = FS_METADATA.inodetable.get(&inodenum).unwrap();

            //populate the dev id field -- can be done outside of the helper
            databuf.f_fsid = FS_METADATA.dev_id;

            //delegate the rest of populating statbuf to the relevant helper
            return Self::_istatfs_helper(self, databuf);
        } else {
            syscall_error(Errno::ENOENT, "stat", "path refers to an invalid file")
        }
    }

    //------------------------------------FSTATFS SYSCALL------------------------------------

    pub fn fstatfs_syscall(&self, fd: i32, databuf: &mut FSData) -> i32 {
        let checkedfd = self.get_filedescriptor(fd).unwrap();
        let unlocked_fd = checkedfd.read();
        if let Some(filedesc_enum) = &*unlocked_fd {
            //populate the dev id field -- can be done outside of the helper
            databuf.f_fsid = FS_METADATA.dev_id;

            match filedesc_enum {
                File(normalfile_filedesc_obj) => {
                    let _inodeobj = FS_METADATA
                        .inodetable
                        .get(&normalfile_filedesc_obj.inode)
                        .unwrap();

                    return Self::_istatfs_helper(self, databuf);
                }
                Socket(_) | Pipe(_) | Stream(_) | Epoll(_) => {
                    return syscall_error(
                        Errno::EBADF,
                        "fstatfs",
                        "can't fstatfs on socket, stream, pipe, or epollfd",
                    );
                }
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
        databuf.f_files = 1024 * 1024 * 1024;
        databuf.f_ffiles = 1024 * 1024 * 515;
        databuf.f_namelen = 254;
        databuf.f_frsize = 4096;
        databuf.f_spare = [0; 32];

        0 //success!
    }

    /// ## ------------------READ SYSCALL------------------
    /// ### Description
    ///
    /// The `read_syscall()` attempts to read `count` bytes from the file
    /// associated with the open file descriptor, `fd`, into the buffer
    /// pointed to by `buf`. On files that support seeking (for example, a
    /// regular file), the read operation commences at the file offset, and
    /// the file offset is incremented by the number of bytes read. If the
    /// file offset is at or past the end of file, no bytes are read,
    /// and read_syscall() returns zero. If the `count` of the bytes to be read
    /// is 0, the read_syscall() returns 0. No data transfer will occur past
    /// the current end-of-file. If the starting position is at or after the
    /// end-of-file, 0 will be returned The reading mechanism is different
    /// for each type of file descriptor, which is discussed in the
    /// implementation.
    ///
    /// ### Function Arguments
    ///
    /// The `read_syscall()` receives three arguments:
    /// * `fd` - This argument refers to the file descriptor from which the data
    ///   is to be read.
    /// * `buf` - This argument refers to the mutable buffer into which the file
    ///   data is to be stored and then returned back. This value is greater
    ///   than or equal to zero.
    /// * `count` - This argument refers to the number of bytes of data to be
    ///   read from the file. This value should be greater than or equal to
    ///   zero.
    ///
    /// ### Returns
    ///
    /// Upon successful completion of this call, we return the number of bytes
    /// read. This number will never be greater than `count`. The value returned
    /// may be less than `count` if the number of bytes left in the file is less
    /// than `count`, if the read_syscall() was interrupted by a signal, or if
    /// the file is a pipe or FIFO or special file and has fewer than
    /// `count` bytes immediately available for reading.
    ///
    /// ### Errors
    ///
    /// * EBADF - Given file descriptor in the arguments is invalid; the file is
    ///   not opened for reading.
    /// * EISDIR - The file descriptor opened for reading is a directory.
    /// * EOPNOTSUPP - Reading from streams is not supported.
    /// * EINVAL - File descriptor is attached to an object which is unsuitable
    ///   for reading
    ///
    /// ### Panics
    ///
    /// * If the parent inode does not exist in the inode table, causing
    ///   unwrap() to panic.
    /// * When the file type filedescriptor contains a Socket as an inode.
    /// * When there is some other issue fetching the file descriptor.
    ///
    /// For more detailed description of all the commands and return values, see
    /// [read(2)](https://man7.org/linux/man-pages/man2/read.2.html)
    pub fn read_syscall(&self, fd: i32, buf: *mut u8, count: usize) -> i32 {
        // Attempt to get the file descriptor
        let checkedfd = self.get_filedescriptor(fd).unwrap();
        // Acquire a write lock on the file descriptor to ensure exclusive access.
        let mut unlocked_fd = checkedfd.write();

        // Check if the file descriptor object is valid
        if let Some(filedesc_enum) = &mut *unlocked_fd {
            // There are different types of file descriptors (File, Sockets, Stream, PIPE),
            // Based on the enum type, each file descriptor has a different implementation
            // for reading data from the file.
            match filedesc_enum {
                // We must borrow the filedesc object as a mutable reference to update the position
                File(ref mut normalfile_filedesc_obj) => {
                    // Return an error if the file cannot be not opened for reading.
                    // The function `is_wronly` checks for write only permissions, for a file
                    // which if true, returns an error, else the file can be opened for reading.
                    if is_wronly(normalfile_filedesc_obj.flags) {
                        return syscall_error(
                            Errno::EBADF,
                            "read",
                            "specified file not open for reading",
                        );
                    }

                    // Get the inode object from the inode table associated with the file
                    // descriptor.
                    let inodeobj = FS_METADATA
                        .inodetable
                        .get(&normalfile_filedesc_obj.inode)
                        .unwrap();

                    // Match the type of inode object with the type (File, Socket, CharDev, Dir)
                    match &*inodeobj {
                        // For `File` type inode, the reading happens from the current position
                        // of the object pointed by `position`. The fileobject is fetched from
                        // the FileObjectTable and we start reading into the buffer `buf` until
                        // `count` number of bytes.
                        Inode::File(_) => {
                            // Get the current position of the File Descriptor Object.
                            let position = normalfile_filedesc_obj.position;
                            // Get the file object associated with the file descriptor object
                            let fileobject =
                                FILEOBJECTTABLE.get(&normalfile_filedesc_obj.inode).unwrap();

                            // `readat` function reads from file at specified offset into provided
                            // C-buffer. If successful, then the position in the file descriptor
                            // object is updated by adding the number of bytes read (bytesread).
                            // This ensures that the next read operation will start from the correct
                            // position.
                            let bytesread =
                                fileobject.readat(buf, count, position as usize).unwrap();
                            //move position forward by the number of bytes we've read
                            normalfile_filedesc_obj.position += bytesread;
                            // Return the number of bytes read.
                            bytesread as i32
                        }

                        // For `CharDev` type inode, the reading happens from the Character Device
                        // file, with each device type returning different results returned
                        // from the `_read_chr_file` file.
                        Inode::CharDev(char_inode_obj) => {
                            // reads from character devices by matching the device number (DevNo) of
                            // the DeviceInode.
                            self._read_chr_file(&char_inode_obj, buf, count)
                        }

                        // A Sanity check where the File type fd should not have a `Socket` type
                        // inode and should panic.
                        Inode::Socket(_) => {
                            panic!("read(): Socket inode found on a filedesc fd.")
                        }

                        // For `Dir` type inode, an error is returned as reading from a directory is
                        // not allowed
                        Inode::Dir(_) => syscall_error(
                            Errno::EISDIR,
                            "read",
                            "attempted to read from a directory",
                        ),
                    }
                }
                // For `Socket` type file descriptor, a read is equivalent to a `recv_syscall` so we
                // transfer control there. A `recv_syscall` is used for receiving message from a
                // socket.
                Socket(_) => {
                    drop(unlocked_fd);
                    self.recv_common(fd, buf, count, 0, &mut None)
                }
                // Reading from `Stream` type file descriptors is not supported.
                Stream(_) => syscall_error(
                    Errno::EOPNOTSUPP,
                    "read",
                    "reading from stdin not implemented yet",
                ),
                // Reading from `Epoll` type file descriptors is not supported.
                Epoll(_) => syscall_error(
                    Errno::EINVAL,
                    "read",
                    "fd is attached to an object which is unsuitable for reading",
                ),
                // The `Pipe` type file descriptor handles read through blocking and non-blocking
                // modes differently to ensure appropriate behavior based on the flags set on the
                // pipe. In blocking mode, the read_from_pipe function will wait until data is
                // available to read. This means that if the pipe is empty, the read operation will
                // block (wait) until data is written to the pipe. In non-blocking mode, the
                // read_from_pipe function will return immediately with an EAGAIN error if there is
                // no data available to read. This prevents the function from blocking.
                Pipe(pipe_filedesc_obj) => {
                    // Return an error if the pipe cannot be not opened for reading.
                    if is_wronly(pipe_filedesc_obj.flags) {
                        return syscall_error(
                            Errno::EBADF,
                            "read",
                            "specified file not open for reading",
                        );
                    }
                    // Check if the `O_NONBLOCK` flag is set in the pipe's flags.
                    // If `O_NONBLOCK` is set, we set nonblocking to true, indicating that the pipe
                    // operates in non-blocking mode.
                    let mut nonblocking = false;
                    if pipe_filedesc_obj.flags & O_NONBLOCK != 0 {
                        nonblocking = true;
                    }

                    // Ensures that the function keeps trying to read data until it either succeeds
                    // or encounters a non-retryable error.
                    loop {
                        // loop over pipe reads so we can periodically check for cancellation
                        let ret = pipe_filedesc_obj
                            .pipe
                            .read_from_pipe(buf, count, nonblocking)
                            as i32;
                        // Check if the pipe is in blocking mode and the read returned EAGAIN.
                        // It means that no data is currently available and the read should be tried
                        // again later.
                        if pipe_filedesc_obj.flags & O_NONBLOCK == 0
                            && ret == -(Errno::EAGAIN as i32)
                        {
                            // Check if the cancel status is set
                            if self
                                .cancelstatus
                                .load(interface::RustAtomicOrdering::Relaxed)
                            {
                                // If the cancel status is set in the cage, we trap around a cancel
                                // point until the individual thread is signaled to cancel itself
                                loop {
                                    interface::cancelpoint(self.cageid);
                                }
                            }
                            // Received `EAGAIN` and no cancellation is requested, continue the loop
                            // to try reading from the pipe again
                            continue;
                        }
                        // If the read was successful, return the result
                        return ret;
                    }
                }
            }
        } else {
            syscall_error(Errno::EBADF, "read", "invalid file descriptor")
        }
    }

    /// ## ------------------PREAD SYSCALL------------------
    /// ### Description
    ///
    /// The `pread_syscall()` attempts to read `count` bytes from the file
    /// associated with the open file descriptor, `fd`, into the buffer
    /// pointed to by `buf`, starting at the given `offset`. Unlike `read()`,
    /// `pread()` does not change the file offset.
    ///
    /// ### Function Arguments
    ///
    /// The `pread_syscall()` receives four arguments:
    /// * `fd` - This argument refers to the file descriptor from which the data
    ///   is to be read.
    /// * `buf` - This argument refers to the mutable buffer into which the file
    ///   data is to be stored and then returned back. This value is greater
    ///   than or equal to zero.
    /// * `count` - This argument refers to the number of bytes of data to be
    ///   read from the file. This value should be greater than or equal to
    ///   zero.
    /// * `offset` - This argument specifies the file offset at which the read
    ///   is to begin. The file offset is not changed by this operation.
    ///
    /// ### Returns
    ///
    /// Upon successful completion of this call, we return the number of bytes
    /// read. This number will never be greater than `count`. The value returned
    /// may be less than `count` if the number of bytes left in the file is less
    /// than `count`.
    ///
    /// ### Errors
    ///
    /// * EBADF - Given file descriptor in the arguments is invalid; the file is
    ///   not opened for reading.
    /// * EISDIR - The file descriptor opened for reading is a directory.
    /// * EOPNOTSUPP - Reading from streams is not supported.
    /// * ESPIPE - The file descriptor opened for reading is either of type
    ///   Socket, Stream, Pipe, or Epoll.
    ///
    /// ### Panics
    ///
    /// * If the parent inode does not exist in the inode table, causing
    ///   unwrap() to panic.
    /// * When the file type file descriptor contains a Socket as an inode.
    /// * When there is some other issue fetching the file descriptor.
    ///
    /// For more detailed description of all the commands and return values, see
    /// [pread(2)](https://man7.org/linux/man-pages/man2/pread.2.html)
    pub fn pread_syscall(&self, fd: i32, buf: *mut u8, count: usize, offset: isize) -> i32 {
        // Attempt to get the file descriptor
        let checkedfd = self.get_filedescriptor(fd).unwrap();
        // Acquire a write lock on the file descriptor to ensure exclusive access.
        let mut unlocked_fd = checkedfd.write();

        // Check if the file descriptor object is valid
        if let Some(filedesc_enum) = &mut *unlocked_fd {
            // There are different types of file descriptors (File, Sockets, Stream, PIPE),
            // Based on the enum type, each file descriptor has a different implementation
            // for reading data from the file.
            match filedesc_enum {
                //we must borrow the filedesc object as a mutable reference to update the position
                File(ref mut normalfile_filedesc_obj) => {
                    // Return an error if the file cannot be not opened for reading.
                    // The function `is_wronly` checks for write only permissions, for a file
                    // which if true, returns an error, else the file can be opened for reading.
                    if is_wronly(normalfile_filedesc_obj.flags) {
                        return syscall_error(
                            Errno::EBADF,
                            "pread",
                            "specified file not open for reading",
                        );
                    }

                    // Get the inode object from the inode table associated with the file
                    // descriptor.
                    let inodeobj = FS_METADATA
                        .inodetable
                        .get(&normalfile_filedesc_obj.inode)
                        .unwrap();

                    // Match the type of inode object with the type (File, Socket, CharDev, Dir)
                    match &*inodeobj {
                        Inode::File(_) => {
                            // Fetch the file object from the file object table
                            let fileobject =
                                FILEOBJECTTABLE.get(&normalfile_filedesc_obj.inode).unwrap();

                            // `readat` function reads from file at specified offset into provided
                            // C-buffer.
                            let bytesread = fileobject.readat(buf, count, offset as usize).unwrap();
                            // Return the number of bytes read.
                            bytesread as i32
                        }

                        // For `CharDev` type inode, the reading happens from the Character Device
                        // file, with each device type returning different results returned
                        // from the `_read_chr_file` file.
                        Inode::CharDev(char_inode_obj) => {
                            // reads from character devices by matching the device number (DevNo) of
                            // the DeviceInode. This function returns the number of bytes read from
                            // the character device and updates the buffer `buf` with them.
                            self._read_chr_file(&char_inode_obj, buf, count)
                        }
                        // A Sanity check where the File type fd should not have a `Socket` type
                        // inode and should panic.
                        Inode::Socket(_) => {
                            panic!("pread(): Socket inode found on a filedesc fd")
                        }
                        // For `Dir` type inode, an error is returned as reading from a directory is
                        // not allowed
                        Inode::Dir(_) => syscall_error(
                            Errno::EISDIR,
                            "pread",
                            "attempted to read from a directory",
                        ),
                    }
                }
                // Return an error for Sockets, as they do not support the concept of seeking to a
                // specific offset because data arrives in a continuous stream from the network.
                Socket(_) => syscall_error(
                    Errno::ESPIPE,
                    "pread",
                    "file descriptor is associated with a socket, cannot seek",
                ),
                // Return an error for Streams, as like sockets, streams are sequential and do not
                // support seeking to an offset.
                Stream(_) => syscall_error(
                    Errno::ESPIPE,
                    "pread",
                    "file descriptor is associated with a stream, cannot seek",
                ),
                // Return an error for Pipes, as they are designed for sequential reads and writes
                // between processes. Seeking within a pipe would not make sense because data is
                // read in the order it was written, making pread inapplicable.
                Pipe(_) => syscall_error(
                    Errno::ESPIPE,
                    "pread",
                    "file descriptor is associated with a pipe, cannot seek",
                ),
                // Return an error for Epoll, as Epoll file descriptors are for event notification
                // and do not hold any data themselves.
                Epoll(_) => syscall_error(
                    Errno::ESPIPE,
                    "pread",
                    "file descriptor is associated with an epollfd, cannot seek",
                ),
            }
        } else {
            syscall_error(Errno::EBADF, "pread", "invalid file descriptor")
        }
    }

    /// ### Description
    ///
    /// The `_read_chr_file()` helper function is used by `read_syscall()` and
    /// `pread_syscall()` for reading from character device type files.
    /// It reads from character devices by matching the device number (DevNo)
    /// of the DeviceInode. It handles `/dev/null`, `/dev/zero`, `/dev/random`,
    /// and `/dev/urandom` by performing the appropriate actions for each
    /// device. If the device is unsupported, it returns an error indicating
    /// that the operation is not supported. This function is used for
    /// interacting with special character files in a Unix-like filesystem.
    ///
    /// ### Function Arguments
    ///
    /// The `_read_chr_file()` receives three arguments:
    /// * `inodeobj` - This argument refers to the DeviceInode object, which
    ///   contains metadata about the character device.
    /// * `buf` - This argument refers to the mutable buffer into which the data
    ///   is to be stored and then returned back.
    /// * `count` - This argument refers to the number of bytes of data to be
    ///   read from the file. This value should be greater than or equal to
    ///   zero.
    ///
    /// ### Returns
    ///
    /// Upon successful completion of this call, we return the number of bytes
    /// read. This number will never be greater than `count`. The value returned
    /// may be less than `count` if the number of bytes left in the file is less
    /// than `count`, if the read_syscall() was interrupted by a signal, or if
    /// the file is a pipe or FIFO or special file and has fewer than
    /// `count` bytes immediately available for reading.
    ///
    /// ### Errors
    ///
    /// * EOPNOTSUPP - When read from the unspecified object is not permitted
    ///
    /// ### Panics
    ///
    /// * This function does not panic in any cases.
    fn _read_chr_file(&self, inodeobj: &DeviceInode, buf: *mut u8, count: usize) -> i32 {
        // Determine which character device is being read from, based on the device
        // number (dev) in the DeviceInode object.
        match inodeobj.dev {
            // reading from /dev/null always reads 0 bytes indicating an end-of-file condition.
            NULLDEVNO => 0,
            // reading from /dev/zero fills the buffer with zeroes
            ZERODEVNO => interface::fillzero(buf, count),
            // reading from /dev/random fills the buffer with random bytes
            RANDOMDEVNO => interface::fillrandom(buf, count),
            // reading from /dev/urandom also fills the buffer with random bytes
            // Note: This might have to be changed in future.
            URANDOMDEVNO => interface::fillrandom(buf, count),
            // for any device number not specifically handled above,
            // we return an error
            _ => syscall_error(
                Errno::EOPNOTSUPP,
                "read or pread",
                "read from specified device not implemented",
            ),
        }
    }

    /// ## ------------------WRITE SYSCALL------------------
    /// ### Description
    ///
    /// The `write_syscall()` attempts to write `count` bytes from the buffer
    /// pointed to by `buf` to the file associated with the open file
    /// descriptor, `fd`. The number of bytes written may be less than count
    /// if, for example, there is insufficient space on the underlying
    /// physical medium, or when the syscall gets interrupted by a signal,
    /// which we have not implemented yet. On files that support seeking
    /// (for example, a regular file), the write operation commences at the
    /// file offset, and the file offset is incremented by the number of bytes
    /// written. For files that do not support seeking, writing starts from the
    /// logical end of the file (for pipes and streams) or is handled based on
    /// the specific characteristics of the file type (for character devices).
    ///
    /// ### Function Arguments
    ///
    /// The `write_syscall()` receives three arguments:
    /// * `fd` - This argument refers to the file descriptor to which the data
    ///   is to be written.
    /// * `buf` - This argument refers to the buffer from which the file data is
    ///   to be written. This value is greater than or equal to zero.
    /// * `count` - This argument refers to the number of bytes of data to be
    ///   written to the file. This value should be greater than or equal to
    ///   zero.
    ///
    /// ### Returns
    ///
    /// Upon successful completion of this call, we return the number of bytes
    /// written. This number will never be greater than `count`. The value
    /// returned may be less than `count` if the write_syscall() was
    /// interrupted by a signal, or if the file is a pipe or FIFO or special
    /// file and has fewer than `count` bytes immediately available for
    /// writing.
    ///
    /// ### Errors
    ///
    /// * EBADF - Given file descriptor in the arguments is invalid; the
    ///   file/stream/ pipe are not open for writing.
    /// * EISDIR - The file descriptor opened for writing is a directory.
    /// * EINVAL - File descriptor is attached to an object which is unsuitable
    ///   for writing.
    ///
    /// ### Panics
    ///
    /// * If the parent inode does not exist in the inode table, causing
    ///   unwrap() to panic.
    /// * When the file type file descriptor contains a Socket as an inode.
    /// * When there is some other issue fetching the file descriptor.
    /// # When writing the blank bytes in the file fails.
    ///
    /// For more detailed description of all the commands and return values, see
    /// [write(2)](https://man7.org/linux/man-pages/man2/write.2.html)
    pub fn write_syscall(&self, fd: i32, buf: *const u8, count: usize) -> i32 {
        //BUG
        //If the provided file descriptor is out of bounds, get_filedescriptor returns
        //Err(), unwrapping on which  produces a 'panic!'
        //otherwise, file descriptor table entry is stored in 'checkedfd'
        let checkedfd = self.get_filedescriptor(fd).unwrap();
        // Acquire a write lock on the file descriptor to ensure exclusive access.
        let mut unlocked_fd = checkedfd.write();

        // Check if the file descriptor object is valid
        if let Some(filedesc_enum) = &mut *unlocked_fd {
            // There are different types of file descriptors (File, Sockets, Stream, PIPE),
            // Based on the enum type, each file descriptor has a different implementation
            // for writing data to the file.
            match filedesc_enum {
                // We must borrow the filedesc object as a mutable reference to update the position
                File(ref mut normalfile_filedesc_obj) => {
                    // Return an error if the file cannot be opened for writing.
                    // The function `is_rdonly` checks for read only permissions, for a file
                    // which if true, returns an error, else the file can be opened for writing.
                    if is_rdonly(normalfile_filedesc_obj.flags) {
                        return syscall_error(
                            Errno::EBADF,
                            "write",
                            "specified file not open for writing",
                        );
                    }

                    // Get the inode object from the inode table associated with the file
                    // descriptor.
                    let mut inodeobj = FS_METADATA
                        .inodetable
                        .get_mut(&normalfile_filedesc_obj.inode)
                        .unwrap();

                    // Match the type of inode object with the type (File, Socket, CharDev, Dir)
                    match *inodeobj {
                        // For `File` type inode, the writing happens at the current position
                        // of the object pointed by `position`. The fileobject is fetched from
                        // the FileObjectTable and we start writing from the buffer `buf` until
                        // `count` number of bytes.
                        Inode::File(ref mut normalfile_inode_obj) => {
                            // Get the current position of the File Descriptor Object.
                            let position = normalfile_filedesc_obj.position;

                            // Calculate the number of blank bytes needed to pad the file
                            // if the current position is past the end of the file, because
                            // the space between the end of the file and the new write position
                            // should be filled with zeroes to maintain data integrity.
                            let filesize = normalfile_inode_obj.size;
                            let blankbytecount = position as isize - filesize as isize;

                            // Get the mutable file object associated with the file descriptor
                            // object
                            let mut fileobject = FILEOBJECTTABLE
                                .get_mut(&normalfile_filedesc_obj.inode)
                                .unwrap();

                            // Pad the file with blank bytes if we are at a position past
                            // the end of the file. `zerofill_at` function fills `blankbytecount`
                            // bytes in the file from the offset
                            // starting from `filesize`.
                            if blankbytecount > 0 {
                                if let Ok(byteswritten) =
                                    fileobject.zerofill_at(filesize, blankbytecount as usize)
                                {
                                    if byteswritten != blankbytecount as usize {
                                        panic!("Write of blank bytes for write failed!");
                                    }
                                } else {
                                    panic!("Write of blank bytes for write failed!");
                                }
                            }

                            // Write `count` bytes from `buf` to the file at `position` using
                            // `writeat` function, which returns the number of bytes written when
                            // successful, else panics.
                            let byteswritten = fileobject.writeat(buf, count, position).unwrap();
                            // Move position forward by the number of bytes we've written
                            normalfile_filedesc_obj.position = position + byteswritten;
                            // Update the file size if necessary
                            if normalfile_filedesc_obj.position > normalfile_inode_obj.size {
                                normalfile_inode_obj.size = normalfile_filedesc_obj.position;
                                drop(inodeobj);
                                drop(fileobject);
                                log_metadata(&FS_METADATA, normalfile_filedesc_obj.inode);
                            }
                            // Return the number of bytes written
                            byteswritten as i32
                        }

                        // For `CharDev` type inode, the writing happens to the Character Device
                        // file, with each device type returning the `count` number of bytes that
                        // are to be written.
                        Inode::CharDev(ref char_inode_obj) => {
                            // The `_write_chr_file` helper function typically does not write
                            // anything to the device and simply returns the bytes count.
                            self._write_chr_file(&char_inode_obj, buf, count)
                        }

                        // A Sanity check is added to make sure that there is no such case when the
                        // fd type is "File" and the inode type is "Socket". This state is ideally
                        // not possible, so we panic in such cases.
                        Inode::Socket(_) => {
                            panic!("write(): Socket inode found on a filedesc fd")
                        }

                        // For `Dir` type inode, an error is returned as writing to a directory is
                        // not allowed.
                        Inode::Dir(_) => syscall_error(
                            Errno::EISDIR,
                            "write",
                            "attempted to write to a directory",
                        ),
                    }
                }

                // For `Socket` type file descriptor, a write is equivalent to a `send_syscall` so
                // we transfer control there. A `send_syscall` is used for sending
                // message through a socket.
                Socket(_) => {
                    drop(unlocked_fd);
                    self.send_syscall(fd, buf, count, 0)
                }

                // For `Stream` type file descriptors (stdout or stderr), the data is printed out
                // and the number of bytes written is returned.
                Stream(stream_filedesc_obj) => {
                    // Stream 1 represents `stdout` and 2 represents `stderr`.
                    if stream_filedesc_obj.stream == 1 || stream_filedesc_obj.stream == 2 {
                        // `log_from_ptr` simply prints out the data to stdout
                        interface::log_from_ptr(buf, count);
                        count as i32
                    } else {
                        syscall_error(
                            Errno::EBADF,
                            "write",
                            "specified stream not open for writing",
                        )
                    }
                }

                // The `Pipe` type file descriptor handles write through blocking and non-blocking
                // modes differently to ensure appropriate behavior based on the flags set on the
                // pipe.
                Pipe(pipe_filedesc_obj) => {
                    // Return an error if the pipe cannot be opened for writing.
                    if is_rdonly(pipe_filedesc_obj.flags) {
                        return syscall_error(
                            Errno::EBADF,
                            "write",
                            "specified pipe not open for writing",
                        );
                    }

                    // Check if the `O_NONBLOCK` flag is set in the pipe's flags.
                    // If `O_NONBLOCK` is set, we set nonblocking to true, indicating that the pipe
                    // operates in non-blocking mode.
                    let mut nonblocking = false;
                    if pipe_filedesc_obj.flags & O_NONBLOCK != 0 {
                        nonblocking = true;
                    }

                    // Attempt to write `count` bytes from `buf` to the pipe.
                    // write_to_pipe writes a specified number of bytes starting at the given
                    // pointer to a circular buffer.
                    let retval = pipe_filedesc_obj
                        .pipe
                        .write_to_pipe(buf, count, nonblocking)
                        as i32;

                    // If the write fails with `EPIPE`, send a `SIGPIPE` signal to the process.
                    if retval == -(Errno::EPIPE as i32) {
                        // BUG: Need to add the check for processing a signal.
                        interface::lind_kill_from_id(self.cageid, SIGPIPE);
                    }
                    retval
                }

                // Writing to `Epoll` type file descriptors is not supported.
                Epoll(_) => syscall_error(
                    Errno::EINVAL,
                    "write",
                    "fd is attached to an object which is unsuitable for writing",
                ),
            }
        } else {
            syscall_error(Errno::EBADF, "write", "invalid file descriptor")
        }
    }

    /// ## ------------------PWRITE SYSCALL------------------
    /// ### Description
    ///
    /// The `pwrite_syscall()` attempts to write `count` bytes from the buffer
    /// pointed to by `buf` to the file associated with the open file
    /// descriptor, `fd`, starting at the given `offset`. Unlike `write()`,
    /// `pwrite()` does not change the file offset.
    ///
    /// ### Function Arguments
    ///
    /// The `pwrite_syscall()` receives four arguments:
    /// * `fd` - This argument refers to the file descriptor to which the data
    ///   is to be written.
    /// * `buf` - This argument refers to the buffer containing the data to be
    ///   written to the file.
    /// * `count` - This argument refers to the number of bytes of data to be
    ///   written to the file.
    /// * `offset` - This argument specifies the file offset at which the write
    ///   is to begin. The file offset is not changed by this operation.
    ///
    /// ### Returns
    ///
    /// Upon successful completion of this call, we return the number of bytes
    /// written. This number will never be greater than `count`. The value
    /// returned may be less than `count` if there is insufficient space in
    /// the file system or if the pwrite is interrupted by a signal.
    ///
    /// ### Errors
    ///
    /// * `EBADF` - Given file descriptor in the arguments is invalid; the file
    ///   is not opened for writing.
    /// * `EISDIR` - The file descriptor opened for writing is a directory.
    /// * `ESPIPE` - The file descriptor opened for writing is either of type
    ///   Socket, Stream, Pipe, or Epoll.
    ///
    /// ### Panics
    ///
    /// * If the parent inode does not exist in the inode table, causing
    ///   unwrap() to panic.
    /// * When the file type file descriptor contains a Socket as an inode.
    /// * When there is some other issue fetching the file descriptor.
    ///
    /// For more detailed description of all the commands and return values, see
    /// [pwrite(2)](https://man7.org/linux/man-pages/man2/pwrite.2.html)
    pub fn pwrite_syscall(&self, fd: i32, buf: *const u8, count: usize, offset: isize) -> i32 {
        //BUG
        //If the provided file descriptor is out of bounds, get_filedescriptor returns
        //Err(), unwrapping on which  produces a 'panic!'
        //otherwise, file descriptor table entry is stored in 'checkedfd'
        let checkedfd = self.get_filedescriptor(fd).unwrap();
        // Acquire a write lock on the file descriptor to ensure exclusive access.
        let mut unlocked_fd = checkedfd.write();

        // Check if the file descriptor object is valid
        if let Some(filedesc_enum) = &mut *unlocked_fd {
            // Match the type of file descriptor (File, Sockets, Stream, Pipe, Epoll)
            match filedesc_enum {
                // We must borrow the filedesc object as a mutable reference to update the position
                File(ref mut normalfile_filedesc_obj) => {
                    // Return an error if the file cannot be not opened for writing.
                    if is_rdonly(normalfile_filedesc_obj.flags) {
                        return syscall_error(
                            Errno::EBADF,
                            "pwrite",
                            "specified file not open for writing",
                        );
                    }

                    // Get the inode object from the inode table associated with the file
                    // descriptor.
                    let mut inodeobj = FS_METADATA
                        .inodetable
                        .get_mut(&normalfile_filedesc_obj.inode)
                        .unwrap();

                    // Match the type of inode object with the type (File, Socket, CharDev, Dir)
                    match *inodeobj {
                        Inode::File(ref mut normalfile_inode_obj) => {
                            // Calculate the number of blank bytes needed to pad the file
                            // if the current offset is past the end of the file, because
                            // the space between the end of the file and the new write position
                            // should be filled with zeroes to maintain data integrity.
                            let position = offset as usize;
                            let filesize = normalfile_inode_obj.size;
                            let blankbytecount = offset - filesize as isize;

                            let mut fileobject = FILEOBJECTTABLE
                                .get_mut(&normalfile_filedesc_obj.inode)
                                .unwrap();

                            // Pad the file with blank bytes if we are at a position past
                            // the end of the file. `zerofill_at` function fills `blankbytecount`
                            // bytes in the file from the offset
                            // starting from `filesize`.
                            if blankbytecount > 0 {
                                if let Ok(byteswritten) =
                                    fileobject.zerofill_at(filesize, blankbytecount as usize)
                                {
                                    if byteswritten != blankbytecount as usize {
                                        panic!("Write of blank bytes for pwrite failed!");
                                    }
                                } else {
                                    panic!("Write of blank bytes for pwrite failed!");
                                }
                            }

                            // Write `count` bytes from `buf` to the file at `position` using
                            // `writeat` function, which returns the number of bytes written.
                            let retval = fileobject.writeat(buf, count, position).unwrap();
                            let newposition = position + retval;

                            // Update the file size once data is written to the file
                            if newposition > filesize {
                                normalfile_inode_obj.size = newposition;
                                // Drop the mutable instances of fileobject and inodeobj, before
                                // writing to the metadata.
                                drop(fileobject);
                                drop(inodeobj);
                                log_metadata(&FS_METADATA, normalfile_filedesc_obj.inode);
                            }

                            // Return the final value of the bytes written in the file
                            retval as i32
                        }

                        // For `CharDev` type inode, the writing happens to the Character Device
                        // file, with each device type returning the `count` number of bytes that
                        // are to be written.
                        Inode::CharDev(ref char_inode_obj) => {
                            // The `_write_chr_file` helper function typically does not write
                            // anything to the device and simply returns the bytes count.
                            self._write_chr_file(&char_inode_obj, buf, count)
                        }

                        // A Sanity check is added to make sure that there is no such case when the
                        // fd type is "File" and the inode type is "Socket". This state is ideally
                        // not possible, so we panic in such cases.
                        Inode::Socket(_) => {
                            panic!("pwrite: socket fd and inode don't match types")
                        }

                        // For `Dir` type inode, an error is returned as writing to a directory is
                        // not allowed
                        Inode::Dir(_) => syscall_error(
                            Errno::EISDIR,
                            "pwrite",
                            "attempted to write to a directory",
                        ),
                    }
                }
                // Return an error for Sockets, as they do not support the concept of seeking to a
                // specific offset because data arrives in a continuous stream from the network.
                Socket(_) => syscall_error(
                    Errno::ESPIPE,
                    "pwrite",
                    "file descriptor is associated with a socket, cannot seek",
                ),
                // Return an error for Streams, as like sockets, streams are sequential and do not
                // support seeking to an offset.
                Stream(_) => syscall_error(
                    Errno::ESPIPE,
                    "pwrite",
                    "file descriptor is associated with a stream, cannot seek",
                ),
                // Return an error for Pipes, as they are designed for sequential reads and writes
                // between processes. Seeking within a pipe would not make sense because data is
                // read in the order it was written, making pwrite inapplicable.
                Pipe(_) => syscall_error(
                    Errno::ESPIPE,
                    "pwrite",
                    "file descriptor is associated with a pipe, cannot seek",
                ),
                // Return an error for Epoll, as Epoll file descriptors are for event notification
                // and do not hold any data themselves.
                Epoll(_) => syscall_error(
                    Errno::ESPIPE,
                    "pwrite",
                    "file descriptor is associated with an epollfd, cannot seek",
                ),
            }
        } else {
            syscall_error(Errno::EBADF, "pwrite", "invalid file descriptor")
        }
    }

    /// ## ------------------WRITE CHARACTER DEVICE HELPER FUNCTION------------------
    /// ### Description
    ///
    /// The `_write_chr_file()` helper function handles the writing to character
    /// device files. Depending on the specific character device being
    /// written to, the function may either succeed without performing any
    /// action or return an error indicating that writing to the specified
    /// device is not supported. This function is currently referenced in
    /// "write" and "pwrite" syscalls.
    ///
    /// ### Function Arguments
    ///
    /// The `_write_chr_file()` receives three arguments:
    /// * `inodeobj` - This argument refers to the inode object of the character
    ///   device file.
    /// * `_buf` - This argument refers to the buffer containing the data to be
    ///   written to the file.
    /// * `count` - This argument refers to the number of bytes of data to be
    ///   written to the file.
    ///
    /// ### Returns
    ///
    /// Upon successful completion of this call, we return the number of bytes
    /// written. For specific character devices, the function transparently
    /// succeeds while doing nothing, returning `count` as the number of
    /// bytes written. If writing to the specified device is not
    /// implemented, an error is returned.
    ///
    /// ### Errors
    ///
    /// * `EOPNOTSUPP` - The write operation is not supported for the specified
    ///   device.
    ///
    /// ### Panics
    ///
    /// This function does not cause any panics.
    fn _write_chr_file(&self, inodeobj: &DeviceInode, _buf: *const u8, count: usize) -> i32 {
        // Writes to any of these device files transparently succeed while doing
        // nothing. The data passed to them for writing is simply discarded.
        match inodeobj.dev {
            // Represented by "/dev/null", it is a virtual null device used to discard any output
            // redirected to it.
            NULLDEVNO => count as i32,
            // Represented by "/dev/zero", it provides as many null bytes (zero value) as are read
            // from it.
            ZERODEVNO => count as i32,
            // Represented by "/dev/random", it provides random output.
            RANDOMDEVNO => count as i32,
            // Represented by "/dev/urandom", it also provides random output.
            // Writes behave identically for both random and urandom devices and will not block.
            // Currently, we are not doing anything on "write" for these devices.
            URANDOMDEVNO => count as i32,
            // For other devices, return an error indicating the operation is not supported.
            _ => syscall_error(
                Errno::EOPNOTSUPP,
                "write or pwrite",
                "write to specified device not implemented",
            ),
        }
    }

    //------------------------------------WRITEV SYSCALL------------------------------------

    pub fn writev_syscall(
        &self,
        fd: i32,
        iovec: *const interface::IovecStruct,
        iovcnt: i32,
    ) -> i32 {
        let checkedfd = self.get_filedescriptor(fd).unwrap();
        let mut unlocked_fd = checkedfd.write();
        if let Some(filedesc_enum) = &mut *unlocked_fd {
            // we're only implementing this for INET/tcp sockets right now
            match filedesc_enum {
                Socket(socket_filedesc_obj) => {
                    let sock_tmp = socket_filedesc_obj.handle.clone();
                    let sockhandle = sock_tmp.write();

                    match sockhandle.domain {
                        AF_INET | AF_INET6 => match sockhandle.protocol {
                            IPPROTO_TCP => {
                                // to be able to send here we either need to be fully connected, or
                                // connected for write only
                                if (sockhandle.state != ConnState::CONNECTED)
                                    && (sockhandle.state != ConnState::CONNWRONLY)
                                {
                                    return syscall_error(
                                        Errno::ENOTCONN,
                                        "writev",
                                        "The descriptor is not connected",
                                    );
                                }

                                //because socket must be connected it must have an inner raw socket
                                // lets call the kernel writev on that socket
                                let retval = sockhandle
                                    .innersocket
                                    .as_ref()
                                    .unwrap()
                                    .writev(iovec, iovcnt);
                                if retval < 0 {
                                    match Errno::from_discriminant(interface::get_errno()) {
                                        Ok(i) => {
                                            return syscall_error(
                                                i,
                                                "writev",
                                                "The libc call to writev failed!",
                                            );
                                        }
                                        Err(()) => panic!(
                                            "Unknown errno value from socket writev returned!"
                                        ),
                                    };
                                } else {
                                    return retval;
                                }
                            }
                            _ => {
                                return syscall_error(
                                    Errno::EOPNOTSUPP,
                                    "writev",
                                    "System call not implemented for this socket protocol",
                                );
                            }
                        },
                        AF_UNIX => {
                            match sockhandle.protocol {
                                IPPROTO_TCP => {
                                    // to be able to send here we either need to be fully connected,
                                    // or connected for write only
                                    if (sockhandle.state != ConnState::CONNECTED)
                                        && (sockhandle.state != ConnState::CONNWRONLY)
                                    {
                                        return syscall_error(
                                            Errno::ENOTCONN,
                                            "writev",
                                            "The descriptor is not connected",
                                        );
                                    }
                                    // get the socket pipe, write to it, and return bytes written
                                    let sockinfo = &sockhandle.unix_info.as_ref().unwrap();
                                    let mut nonblocking = false;
                                    if socket_filedesc_obj.flags & O_NONBLOCK != 0 {
                                        nonblocking = true;
                                    }
                                    let retval = match sockinfo.sendpipe.as_ref() {
                                        Some(sendpipe) => sendpipe.write_vectored_to_pipe(
                                            iovec,
                                            iovcnt,
                                            nonblocking,
                                        )
                                            as i32,
                                        None => {
                                            return syscall_error(Errno::EAGAIN, "writev", "there is no data available right now, try again later");
                                        }
                                    };
                                    if retval == -(Errno::EPIPE as i32) {
                                        interface::lind_kill_from_id(self.cageid, SIGPIPE);
                                    } // Trigger SIGPIPE
                                    retval
                                }
                                _ => {
                                    return syscall_error(
                                        Errno::EOPNOTSUPP,
                                        "send",
                                        "Unkown protocol in send",
                                    );
                                }
                            }
                        }
                        _ => {
                            return syscall_error(
                                Errno::EOPNOTSUPP,
                                "writev",
                                "System call not implemented for this socket domain",
                            );
                        }
                    }
                }
                Pipe(pipe_filedesc_obj) => {
                    if is_rdonly(pipe_filedesc_obj.flags) {
                        return syscall_error(
                            Errno::EBADF,
                            "write",
                            "specified pipe not open for writing",
                        );
                    }

                    let mut nonblocking = false;
                    if pipe_filedesc_obj.flags & O_NONBLOCK != 0 {
                        nonblocking = true;
                    }

                    let retval =
                        pipe_filedesc_obj
                            .pipe
                            .write_vectored_to_pipe(iovec, iovcnt, nonblocking)
                            as i32;
                    if retval == -(Errno::EPIPE as i32) {
                        interface::lind_kill_from_id(self.cageid, SIGPIPE);
                    } // Trigger SIGPIPE
                    retval
                }
                _ => {
                    // we currently don't support writev for files/streams
                    return syscall_error(
                        Errno::EOPNOTSUPP,
                        "writev",
                        "System call not implemented for this fd type",
                    );
                }
            }
        } else {
            syscall_error(Errno::EBADF, "write", "invalid file descriptor")
        }
    }

    //------------------------------------LSEEK SYSCALL------------------------------------
    pub fn lseek_syscall(&self, fd: i32, offset: isize, whence: i32) -> i32 {
        let checkedfd = self.get_filedescriptor(fd).unwrap();
        let mut unlocked_fd = checkedfd.write();
        if let Some(filedesc_enum) = &mut *unlocked_fd {
            //confirm fd type is seekable
            match filedesc_enum {
                File(ref mut normalfile_filedesc_obj) => {
                    let inodeobj = FS_METADATA
                        .inodetable
                        .get(&normalfile_filedesc_obj.inode)
                        .unwrap();

                    //handle files/directories differently
                    match &*inodeobj {
                        Inode::File(normalfile_inode_obj) => {
                            let eventualpos = match whence {
                                SEEK_SET => offset,
                                SEEK_CUR => normalfile_filedesc_obj.position as isize + offset,
                                SEEK_END => normalfile_inode_obj.size as isize + offset,
                                _ => {
                                    return syscall_error(Errno::EINVAL, "lseek", "unknown whence");
                                }
                            };

                            if eventualpos < 0 {
                                return syscall_error(
                                    Errno::EINVAL,
                                    "lseek",
                                    "seek to before position 0 in file",
                                );
                            }
                            //subsequent writes to the end of the file must zero pad up until this
                            // point if we overran the end of our file
                            // when seeking

                            normalfile_filedesc_obj.position = eventualpos as usize;
                            //return the location that we sought to
                            eventualpos as i32
                        }

                        Inode::CharDev(_) => {
                            0 //for character files, rather than seeking, we
                              // transparently do nothing
                        }

                        Inode::Socket(_) => {
                            panic!("lseek: socket fd and inode don't match types")
                        }

                        Inode::Dir(dir_inode_obj) => {
                            //for directories we seek between entries, and thus our end position is
                            // the total number of entries
                            let eventualpos = match whence {
                                SEEK_SET => offset,
                                SEEK_CUR => normalfile_filedesc_obj.position as isize + offset,
                                SEEK_END => {
                                    dir_inode_obj.filename_to_inode_dict.len() as isize + offset
                                }
                                _ => {
                                    return syscall_error(Errno::EINVAL, "lseek", "unknown whence");
                                }
                            };

                            //confirm that the location we want to seek to is valid
                            if eventualpos < 0 {
                                return syscall_error(
                                    Errno::EINVAL,
                                    "lseek",
                                    "seek to before position 0 in directory",
                                );
                            }
                            if eventualpos > dir_inode_obj.filename_to_inode_dict.len() as isize {
                                return syscall_error(
                                    Errno::EINVAL,
                                    "lseek",
                                    "seek to after last position in directory",
                                );
                            }

                            normalfile_filedesc_obj.position = eventualpos as usize;
                            //return the location that we sought to
                            eventualpos as i32
                        }
                    }
                }
                Socket(_) => syscall_error(
                    Errno::ESPIPE,
                    "lseek",
                    "file descriptor is associated with a socket, cannot seek",
                ),
                Stream(_) => syscall_error(
                    Errno::ESPIPE,
                    "lseek",
                    "file descriptor is associated with a stream, cannot seek",
                ),
                Pipe(_) => syscall_error(
                    Errno::ESPIPE,
                    "lseek",
                    "file descriptor is associated with a pipe, cannot seek",
                ),
                Epoll(_) => syscall_error(
                    Errno::ESPIPE,
                    "lseek",
                    "file descriptor is associated with an epollfd, cannot seek",
                ),
            }
        } else {
            syscall_error(Errno::EBADF, "lseek", "invalid file descriptor")
        }
    }

    //------------------------------------ACCESS SYSCALL------------------------------------

    pub fn access_syscall(&self, path: &str, amode: u32) -> i32 {
        let truepath = normpath(convpath(path), self);

        //Walk the file tree to get inode from path
        if let Some(inodenum) = metawalk(truepath.as_path()) {
            let inodeobj = FS_METADATA.inodetable.get(&inodenum).unwrap();

            //Get the mode bits if the type of the inode is sane
            let mode = match &*inodeobj {
                Inode::File(f) => f.mode,
                Inode::CharDev(f) => f.mode,
                Inode::Socket(f) => f.mode,
                Inode::Dir(f) => f.mode,
            };

            //We assume that the current user owns the file

            //Construct desired access bits (i.e. 0777) based on the amode parameter
            let mut newmode: u32 = 0;
            if amode & X_OK == X_OK {
                newmode |= S_IXUSR;
            }
            if amode & W_OK == W_OK {
                newmode |= S_IWUSR;
            }
            if amode & R_OK == R_OK {
                newmode |= S_IRUSR;
            }

            //if the desired access bits are compatible with the actual access bits
            //of the file, return a success result, else return a failure result
            if mode & newmode == newmode {
                0
            } else {
                syscall_error(
                    Errno::EACCES,
                    "access",
                    "the requested access would be denied to the file",
                )
            }
        } else {
            syscall_error(
                Errno::ENOENT,
                "access",
                "path does not refer to an existing file",
            )
        }
    }

    /// ### Description
    ///
    /// The `fchdir_syscall()` function changes the current working
    /// directory of the calling process to the directory specified
    /// by an open file descriptor `fd`.
    ///
    /// ### Arguments
    ///
    /// The `fchdir_syscall()` accepts one argument:
    /// * `fd` - an open file descriptor that specifies the directory
    /// to which we want to change the current working directory.
    ///
    /// ### Returns
    ///
    /// Upon successful completion, zero is returned.
    /// In case of a failure, an error is returned, and `errno` is set depending
    /// on the error, e.g. `ENOTDIR`, `EBADF`, etc.
    ///
    /// ### Errors
    ///
    /// * `EBADF` - fd is not a valid file descriptor.
    /// * `ENOTDIR` - the open file descriptor fildes does not refer to a
    ///   directory.
    /// Other errors, like `EACCES`, `ENOMEM`, etc. are not supported.
    ///
    /// ### Panics
    ///
    /// * invalid or out-of-bounds file descriptor, calling unwrap() on which
    /// causes a panic.
    ///
    /// To learn more about the syscall and possible error values, see
    /// [fchdir(2)](https://linux.die.net/man/2/fchdir)

    pub fn fchdir_syscall(&self, fd: i32) -> i32 {
        //BUG
        //if the provided file descriptor is out of bounds, get_filedescriptor returns
        //Err(), unwrapping on which  produces a 'panic!'
        //otherwise, file descriptor table entry is stored in 'checkedfd'
        let checkedfd = self.get_filedescriptor(fd).unwrap();
        let unlocked_fd = checkedfd.read();
        //If a table descriptor entry corresponds to a file, we check if it is
        //a directory file type. If it is not, we return `A component of path is
        //not a directory` error.
        //If it is one of the special file types, we return `Cannot change working
        //directory on this file descriptor` error.
        //Finally, if it does not correspond to any file type, we return `Invalid file
        //descriptor` error.
        let path_string = match &*unlocked_fd {
            None => return syscall_error(Errno::EBADF, "fchdir", "invalid file descriptor"),
            Some(File(normalfile_filedesc_obj)) => {
                let inodenum = normalfile_filedesc_obj.inode;
                //`pathnamefrominodenum` resolves the absolute path of a directory
                //from its inode and returns None in case any of the path components
                //is not a directory
                match pathnamefrominodenum(inodenum) {
                    Some(name) => name,
                    None => {
                        return syscall_error(
                            Errno::ENOTDIR,
                            "fchdir",
                            "the file descriptor does not refer to a directory",
                        )
                    }
                }
            }
            Some(_) => {
                return syscall_error(
                    Errno::ENOTDIR,
                    "fchdir",
                    "the file descriptor does not refer to a directory",
                )
            }
        };
        //Obtain the write lock on the current working directory of the cage
        //and change it to the new directory
        let mut cwd_container = self.cwd.write();
        *cwd_container = interface::RustRfc::new(normpath(convpath(path_string.as_str()), self));

        0 // fchdir success
    }

    /// ### Description
    ///
    /// The `chdir_syscall()` function changes the current working
    /// directory of the calling process to the directory specified
    /// in `path`, which can be an absolute or a relative pathname.
    ///
    /// ### Arguments
    ///
    /// The `chdir_syscall()` accepts one argument:
    /// * `path` - the pathname, to which the current working
    /// directory shall be changed.
    ///
    /// ### Returns
    ///
    /// Upon successful completion, zero is returned.
    /// In case of a failure, an error is returned, and `errno` is set depending
    /// on the error, e.g. EACCES, ENOENT, etc.
    ///
    /// ### Errors
    ///
    /// * `ENOTDIR` - a component of `path` is not a directory.
    /// * `ENOENT` - the directory specified in path does not exist.
    /// Other errors, like `EACCES`, `ENOMEM`, etc. are not supported.
    ///
    /// ### Panics
    ///
    /// * when the previous working directory does not exist or does not
    /// have the directory file type flag, the function panics
    ///
    /// To learn more about the syscall and possible error values, see
    /// [chdir(2)](https://man7.org/linux/man-pages/man2/chdir.2.html)

    pub fn chdir_syscall(&self, path: &str) -> i32 {
        //Convert the provided pathname into an absolute path without `.` or `..`
        //components.
        let truepath = normpath(convpath(path), self);
        //Perfrom a walk down the file tree starting from the root directory to
        //obtain an inode number of the file whose pathname was specified.
        //`None` is returned if one of the following occurs while moving down
        //the tree:
        // 1. Accessing a child of a non-directory inode
        // 2. Accessing a child of a nonexistent parent directory
        // 3. Accessing a nonexistent child
        // 4. Accessing an unexpected component, like `.` or `..` directory reference.
        //In this case, `The file does not exist` error is returned.
        //Otherwise, a `Some()` option containing the inode number is returned.
        if let Some(inodenum) = metawalk(&truepath) {
            //A sanity check to make sure that the last component of the
            //specified path is indeed a directory
            if let Inode::Dir(ref mut _dir) = *(FS_METADATA.inodetable.get_mut(&inodenum).unwrap())
            {
                //Obtain the write lock on the current working directory of the cage
                //and change it to the new directory
                let mut cwd_container = self.cwd.write();
                *cwd_container = interface::RustRfc::new(truepath);
                0 //chdir has succeeded!;
            } else {
                return syscall_error(
                    Errno::ENOTDIR,
                    "chdir",
                    "the last component in path is not a directory",
                );
            }
        } else {
            return syscall_error(
                Errno::ENOENT,
                "chdir",
                "the directory referred to in path does not exist",
            );
        }
    }

    ///##------------------------------------DUP & DUP2 SYSCALLS------------------------------------
    /// ## `dup_syscall`
    ///
    /// ### Description
    /// This function duplicates a file descriptor. It creates a new file
    /// descriptor that refers to the same open file description as the original
    /// file descriptor.
    /// * Finding the Next Available File Descriptor: If `start_desc` is
    ///   provided and it is already in use, the function will continue
    ///   searching for the next available file descriptor starting from
    ///   `start_desc`. If no file descriptors are available, it will return an
    ///   error (`ENFILE`).
    /// * If `fd` is equal to `start_fd`, the function returns `start_fd` as the
    ///   new file descriptor. This is because in this scenario, the original
    ///   and new file descriptors would point to the same file description.
    /// * The `_dup2_helper` function is called to perform the actual file
    ///   descriptor duplication, handling the allocation of a new file
    ///   descriptor, updating the file descriptor table, and incrementing the
    ///   reference count of the file object.
    /// * The function modifies the global `filedescriptortable` array, adding a
    ///   new entry for the duplicated file descriptor. It also increments the
    ///   reference count of the file object associated with the original file
    ///   descriptor.
    /// * The `false` argument passed to `_dup2_helper` indicates that this call
    ///   is from the `dup_syscall` function, not the `dup2_syscall` function.
    ///
    /// ### Function Arguments
    /// * `fd`: The original file descriptor to duplicate.
    /// * `start_desc`:  An optional starting file descriptor number. If
    ///   provided, the new file descriptor will be
    ///  assigned the first available file descriptor number starting from this
    /// value. If not provided, it defaults to  `STARTINGFD`,which is the
    /// minimum designated file descriptor value for new file descriptors.
    ///
    /// ### Returns
    /// * The new file descriptor on success.
    /// * `EBADF`: If the original file descriptor is invalid.
    /// * `ENFILE`: If there are no available file descriptors.
    ///
    /// ### Errors
    /// * `EBADF(9)`: If the original file descriptor is invalid.
    /// * `ENFILE(23)`: If there are no available file descriptors.
    ///  ###Panics
    /// * There are no panics for this syscall
    ///[dup(2)](https://man7.org/linux/man-pages/man2/dup.2.html)

    pub fn dup_syscall(&self, fd: i32, start_desc: Option<i32>) -> i32 {
        //if a starting fd was passed, then use that as the starting point, but
        // otherwise, use the designated minimum of STARTINGFD
        let start_fd = match start_desc {
            Some(start_desc) => start_desc,
            None => STARTINGFD,
        };

        if start_fd == fd {
            return start_fd;
        } //if the file descriptors are equal, return the new one

        // get the filedesc_enum
        // Attempt to get the file descriptor; handle error if it does not exist
        let checkedfd = match self.get_filedescriptor(fd) {
            Ok(fd) => fd,
            Err(_) => return syscall_error(Errno::EBADF, "dup", "Invalid old file descriptor."),
        };
        let filedesc_enum = checkedfd.write();
        let filedesc_enum = if let Some(f) = &*filedesc_enum {
            f
        } else {
            return syscall_error(Errno::EBADF, "dup", "Invalid old file descriptor.");
        };

        //checking whether the fd exists in the file table
        return Self::_dup2_helper(&self, filedesc_enum, start_fd, false);
    }

    /// ## `dup2_syscall`
    ///
    /// ### Description
    /// This function implements the `dup2` system call, which duplicates a file
    /// descriptor and assigns it to a new file descriptor number. If the
    /// new file descriptor already exists, it is closed before the duplication
    /// takes place.
    /// * File Descriptor Reuse:  If the new file descriptor (`newfd`) is
    ///   already open, the function will first close the existing file
    ///   descriptor silently (without returning an error) before allocating a
    ///   new file descriptor and updating the file descriptor table.
    /// * If `oldfd` and `newfd` are the same, the function returns `newfd`
    ///   without closing it. This is because in this scenario, the original and
    ///   new file descriptors would already point to the same file description.
    /// * the global `filedescriptortable` array, replacing the entry for the
    ///   new file descriptor with a new entry for the duplicated file
    ///   descriptor. It also increments the reference count of the file object
    ///   associated with the original file descriptor.
    ///
    /// ### Function Arguments
    /// * `oldfd`: The original file descriptor to duplicate.
    /// * `newfd`: The new file descriptor number to assign to the duplicated
    ///   file descriptor.
    ///
    /// ### Returns
    /// * The new file descriptor on success.
    ///
    /// ### Errors
    /// * `EBADF(9)`: If the original file descriptor (`oldfd`) is invalid or
    ///   the new file descriptor (`newfd`) number is out of range.
    ///  ###Panics
    /// * There are no panics for this syscall
    ///[dup2(2)](https://linux.die.net/man/2/dup2)

    pub fn dup2_syscall(&self, oldfd: i32, newfd: i32) -> i32 {
        //checking if the new fd is out of range
        if newfd >= MAXFD || newfd < 0 {
            return syscall_error(
                Errno::EBADF,
                "dup2",
                "provided file descriptor is out of range",
            );
        }

        if newfd == oldfd {
            return newfd;
        } //if the file descriptors are equal, return the new one

        // get the filedesc_enum
        let checkedfd = match self.get_filedescriptor(oldfd) {
            Ok(fd) => fd,
            Err(_) => return syscall_error(Errno::EBADF, "dup2", "Invalid old file descriptor."),
        };
        let filedesc_enum = checkedfd.write();
        let filedesc_enum = if let Some(f) = &*filedesc_enum {
            f
        } else {
            return syscall_error(Errno::EBADF, "dup2", "Invalid old file descriptor.");
        };

        //if the old fd exists, execute the helper, else return error
        return Self::_dup2_helper(&self, filedesc_enum, newfd, true);
    }

    /// ## `_dup2_helper`
    ///
    /// ### Description
    /// This helper function performs the actual file descriptor duplication
    /// process for both `dup` and `dup2` system calls. It handles the
    /// allocation of a new file descriptor, updates the file descriptor table,
    /// and increments the reference count of the associated file object.
    /// * Duplication from `dup2_syscall`: If `fromdup2` is true, the function
    ///   first closes the existing file descriptor at `newfd` (if any) before
    ///   allocating a new file descriptor and updating the file descriptor
    ///   table.
    /// * Duplication from `dup_syscall`: If `fromdup2` is false, the function
    ///   allocates a new file descriptor, finds the first available file
    ///   descriptor number starting from `newfd`, and updates the file
    ///   descriptor table.
    /// * Reference Counting: The function increments the reference count of the
    ///   file object associated with the original file descriptor. This ensures
    ///   that the file object is not deleted until all its associated file
    ///   descriptors are closed.
    /// * Socket Handling: For domain sockets, the function increments the
    ///   reference count of both the send and receive pipes associated with the
    ///   socket.
    /// * Stream Handling: Streams are not currently supported for duplication
    /// * Unhandled File Types: If the file descriptor is associated with a file
    ///   type that is not handled by the function (i.e., not a File, Pipe,
    ///   Socket, or Stream), the function returns an error (`EACCES`).
    /// * The function does not handle streams.
    /// * Socket Handling: If the file descriptor is associated with a socket,
    ///   the function handles domain sockets differently by incrementing the
    ///   reference count of both the send and receive pipes.
    /// ### Function Arguments
    /// * `self`:  A reference to the `FsCalls` struct, which contains the file
    ///   descriptor table and other system-related data.
    /// * `filedesc_enum`: A reference to the `FileDescriptor` object
    ///   representing the file descriptor to be duplicated.
    /// * `newfd`: The new file descriptor number to assign to the duplicated
    ///   file descriptor.
    /// * `fromdup2`: A boolean flag indicating whether the call is from
    ///   `dup2_syscall` (true) or `dup_syscall` (false).
    ///
    /// ### Returns
    /// * The new file descriptor on success.
    ///
    /// ### Errors
    /// * `ENFILE(23)`: If there are no available file descriptors.
    /// * `EACCES(13)`: If the file descriptor cannot be duplicated.
    /// ###Panics
    /// * If the file descriptor is associated with a socket, and the inode does
    ///   not match the file descriptor.

    pub fn _dup2_helper(&self, filedesc_enum: &FileDescriptor, newfd: i32, fromdup2: bool) -> i32 {
        let (dupfd, mut dupfdguard) = if fromdup2 {
            let mut fdguard = self.filedescriptortable[newfd as usize].write();
            let closebool = fdguard.is_some();
            drop(fdguard);
            // close the fd in the way of the new fd. mirror the implementation of linux,
            // ignore the potential error of the close here
            if closebool {
                let _close_result = Self::_close_helper_inner(&self, newfd);
            }

            // re-grab clean fd
            fdguard = self.filedescriptortable[newfd as usize].write();
            (newfd, fdguard)
        } else {
            let (newdupfd, guardopt) = self.get_next_fd(Some(newfd));
            if newdupfd < 0 {
                // The function allocates a new file descriptor and updates the file descriptor
                // table, handling the potential for file descriptor table
                // overflow (resulting in an `ENFILE` error).
                return syscall_error(
                    Errno::ENFILE,
                    "dup2_helper",
                    "no available file descriptor number could be found",
                );
            }
            (newdupfd, guardopt.unwrap())
        };

        let dupfdoption = &mut *dupfdguard;

        match filedesc_enum {
            File(normalfile_filedesc_obj) => {
                let inodenum = normalfile_filedesc_obj.inode;
                let mut inodeobj = FS_METADATA.inodetable.get_mut(&inodenum).unwrap();
                //incrementing the ref count so that when close is executed on the dup'd file
                //the original file does not get a negative ref count
                match *inodeobj {
                    // increments the reference count of the file object associated with the
                    // original file descriptor to ensure that the file object
                    // is not deleted until all its associated file descriptors are closed.
                    Inode::File(ref mut normalfile_inode_obj) => {
                        normalfile_inode_obj.refcount += 1;
                    }
                    Inode::Dir(ref mut dir_inode_obj) => {
                        dir_inode_obj.refcount += 1;
                    }
                    Inode::CharDev(ref mut chardev_inode_obj) => {
                        chardev_inode_obj.refcount += 1;
                    }
                    Inode::Socket(_) => panic!("dup: fd and inode do not match."),
                }
            }
            Pipe(pipe_filedesc_obj) => {
                pipe_filedesc_obj.pipe.incr_ref(pipe_filedesc_obj.flags);
            }
            Socket(ref socket_filedesc_obj) => {
                //we handle the closing of sockets on drop
                // checking whether this is a domain socket

                let sock_tmp = socket_filedesc_obj.handle.clone();
                let sockhandle = sock_tmp.write();
                let socket_type = sockhandle.domain;
                if socket_type == AF_UNIX {
                    if let Some(sockinfo) = &sockhandle.unix_info {
                        if let Some(sendpipe) = sockinfo.sendpipe.as_ref() {
                            sendpipe.incr_ref(O_WRONLY);
                        }
                        if let Some(receivepipe) = sockinfo.receivepipe.as_ref() {
                            receivepipe.incr_ref(O_RDONLY);
                        }
                    }
                }
            }
            Stream(_normalfile_filedesc_obj) => {
                // no stream refs
            }
            _ => {
                return syscall_error(Errno::EACCES, "dup or dup2", "can't dup the provided file");
            }
        }

        let mut dupd_fd_enum = filedesc_enum.clone(); //clones the arc for sockethandle

        // get and clone fd, wrap and insert into table.
        match dupd_fd_enum {
            // we don't want to pass on the CLOEXEC flag
            File(ref mut normalfile_filedesc_obj) => {
                normalfile_filedesc_obj.flags = normalfile_filedesc_obj.flags & !O_CLOEXEC;
            }
            Pipe(ref mut pipe_filedesc_obj) => {
                pipe_filedesc_obj.flags = pipe_filedesc_obj.flags & !O_CLOEXEC;
            }
            Socket(ref mut socket_filedesc_obj) => {
                // can do this for domainsockets and sockets
                socket_filedesc_obj.flags = socket_filedesc_obj.flags & !O_CLOEXEC;
            }
            Stream(ref mut stream_filedesc_obj) => {
                stream_filedesc_obj.flags = stream_filedesc_obj.flags & !O_CLOEXEC;
            }
            _ => {
                return syscall_error(Errno::EACCES, "dup or dup2", "can't dup the provided file");
            }
        }

        let _insertval = dupfdoption.insert(dupd_fd_enum);

        return dupfd;
    }

    //------------------------------------CLOSE SYSCALL------------------------------------

    pub fn close_syscall(&self, fd: i32) -> i32 {
        //check that the fd is valid
        return Self::_close_helper(self, fd);
    }

    pub fn _close_helper_inner(&self, fd: i32) -> i32 {
        let checkedfd = self.get_filedescriptor(fd).unwrap();
        let mut unlocked_fd = checkedfd.write();
        if let Some(filedesc_enum) = &mut *unlocked_fd {
            //Decide how to proceed depending on the fd type.
            //First we check in the file descriptor to handle sockets (no-op), sockets
            // (clean the socket), and pipes (clean the pipe), and if it is a
            // normal file descriptor we decrement the refcount to reflect
            // one less reference to the file.
            match filedesc_enum {
                //if we are a socket, we dont change disk metadata
                Stream(_) => {}
                Epoll(_) => {} //Epoll closing not implemented yet
                Socket(ref mut socket_filedesc_obj) => {
                    let sock_tmp = socket_filedesc_obj.handle.clone();
                    let mut sockhandle = sock_tmp.write();

                    // we need to do the following if UDS
                    if let Some(ref mut ui) = sockhandle.unix_info {
                        let inodenum = ui.inode;
                        if let Some(sendpipe) = ui.sendpipe.as_ref() {
                            sendpipe.decr_ref(O_WRONLY);
                            //last reference, lets remove it
                            if sendpipe.is_pipe_closed() {
                                ui.sendpipe = None;
                            }
                        }
                        if let Some(receivepipe) = ui.receivepipe.as_ref() {
                            receivepipe.decr_ref(O_RDONLY);
                            //last reference, lets remove it
                            if receivepipe.is_pipe_closed() {
                                ui.receivepipe = None;
                            }
                        }
                        let mut inodeobj = FS_METADATA.inodetable.get_mut(&ui.inode).unwrap();
                        if let Inode::Socket(ref mut sock) = *inodeobj {
                            sock.refcount -= 1;
                            if sock.refcount == 0 {
                                if sock.linkcount == 0 {
                                    drop(inodeobj);
                                    let path = normpath(
                                        convpath(sockhandle.localaddr.unwrap().path()),
                                        self,
                                    );
                                    FS_METADATA.inodetable.remove(&inodenum);
                                    NET_METADATA.domsock_paths.remove(&path);
                                }
                            }
                        }
                    }
                }
                Pipe(ref pipe_filedesc_obj) => {
                    // lets decrease the pipe objects internal ref count for the corresponding end
                    // depending on what flags are set
                    pipe_filedesc_obj.pipe.decr_ref(pipe_filedesc_obj.flags);
                }
                File(ref normalfile_filedesc_obj) => {
                    let inodenum = normalfile_filedesc_obj.inode;
                    let mut inodeobj = FS_METADATA.inodetable.get_mut(&inodenum).unwrap();

                    match *inodeobj {
                        Inode::File(ref mut normalfile_inode_obj) => {
                            normalfile_inode_obj.refcount -= 1;

                            //if it's not a reg file, then we have nothing to close
                            //Inode::File is a regular file by default
                            if normalfile_inode_obj.refcount == 0 {
                                FILEOBJECTTABLE
                                    .remove(&inodenum)
                                    .unwrap()
                                    .1
                                    .close()
                                    .unwrap();
                                if normalfile_inode_obj.linkcount == 0 {
                                    drop(inodeobj);
                                    //removing the file from the entire filesystem (interface,
                                    // metadata, and object table)
                                    FS_METADATA.inodetable.remove(&inodenum);
                                    let sysfilename = format!("{}{}", FILEDATAPREFIX, inodenum);
                                    interface::removefile(sysfilename).unwrap();
                                    log_metadata(&FS_METADATA, inodenum);
                                } else {
                                    drop(inodeobj);
                                }
                            }
                        }
                        Inode::Dir(ref mut dir_inode_obj) => {
                            dir_inode_obj.refcount -= 1;

                            //if it's not a reg file, then we have nothing to close
                            match FILEOBJECTTABLE.get(&inodenum) {
                                Some(_) => {
                                    return syscall_error(
                                        Errno::ENOEXEC,
                                        "close or dup",
                                        "Non-regular file in file object table",
                                    );
                                }
                                None => {}
                            }
                            if dir_inode_obj.linkcount == 2 && dir_inode_obj.refcount == 0 {
                                //The reference to the inode has to be dropped to avoid
                                //deadlocking because the remove() method will need to
                                //acquire a reference to the same inode from the
                                //filesystem's inodetable.
                                //The inodetable represents a Rust DashMap that deadlocks
                                //when trying to get a reference to its entry while holding any
                                // sort of reference into it.
                                drop(inodeobj);
                                FS_METADATA.inodetable.remove(&inodenum);
                                log_metadata(&FS_METADATA, inodenum);
                            }
                        }
                        Inode::CharDev(ref mut char_inode_obj) => {
                            char_inode_obj.refcount -= 1;

                            //if it's not a reg file, then we have nothing to close
                            match FILEOBJECTTABLE.get(&inodenum) {
                                Some(_) => {
                                    return syscall_error(
                                        Errno::ENOEXEC,
                                        "close or dup",
                                        "Non-regular file in file object table",
                                    );
                                }
                                None => {}
                            }
                            if char_inode_obj.linkcount == 0 && char_inode_obj.refcount == 0 {
                                //removing the file from the metadata
                                drop(inodeobj);
                                FS_METADATA.inodetable.remove(&inodenum);
                            } else {
                                drop(inodeobj);
                            }
                            log_metadata(&FS_METADATA, inodenum);
                        }
                        Inode::Socket(_) => {
                            panic!("close(): Socket inode found on a filedesc fd.")
                        }
                    }
                }
            }
            0
        } else {
            return syscall_error(Errno::EBADF, "close", "invalid file descriptor");
        }
    }

    pub fn _close_helper(&self, fd: i32) -> i32 {
        let inner_result = self._close_helper_inner(fd);
        if inner_result < 0 {
            return inner_result;
        }

        //removing descriptor from fd table
        let checkedfd = self.get_filedescriptor(fd).unwrap();
        let mut unlocked_fd = checkedfd.write();
        if unlocked_fd.is_some() {
            let _discarded_fd = unlocked_fd.take();
        }
        0 //_close_helper has succeeded!
    }

    /// ### Description
    ///
    /// `fcntl_syscall` performs operations, like returning or setting file
    /// status flags, duplicating a file descriptor, etc., on an open file
    /// descriptor
    ///
    /// ### Arguments
    ///
    /// it accepts three parameters:
    /// * `fd` - an open file descriptor
    /// * `cmd` - an operation to be performed on fd
    /// * `arg` - an optional argument (whether or not arg is required is
    ///   determined by cmd)
    ///
    /// ### Returns
    ///
    /// for a successful call, the return value depends on the operation and can
    /// be one of: zero, the new file descriptor, value of file descriptor
    /// flags, value of status flags, etc.
    ///
    /// ### Errors
    ///
    /// * EBADF - fd is not a valid file descriptor
    /// * EINVAL - doesnt match implementation parameters
    ///
    /// ### Panics
    ///
    /// * invalid or out-of-bounds file descriptor), calling unwrap() on it will
    ///   cause a panic.
    /// * Unknown errno value from fcntl returned, will cause panic.
    ///
    /// for more detailed description of all the commands and return values, see
    /// [fcntl(2)](https://linux.die.net/man/2/fcntl)

    pub fn fcntl_syscall(&self, fd: i32, cmd: i32, arg: i32) -> i32 {
        //BUG
        //if the provided file descriptor is out of bounds, get_filedescriptor returns
        // Err(), unwrapping on which  produces a 'panic!'
        //otherwise, file descriptor table entry is stored in 'checkedfd'
        let checkedfd = self.get_filedescriptor(fd).unwrap();
        let mut unlocked_fd = checkedfd.write();
        if let Some(filedesc_enum) = &mut *unlocked_fd {
            //'flags' consists of bitwise-or'd access mode, file creation, and file status flags
            //to retrieve a particular flag, it can bitwise-and'd with 'flags'
            let flags = match filedesc_enum {
                Epoll(obj) => &mut obj.flags,
                Pipe(obj) => &mut obj.flags,
                Stream(obj) => &mut obj.flags,
                File(obj) => &mut obj.flags,
                //not clear why running F_SETFL on Socket type requires special treatment
                Socket(ref mut sockfdobj) => {
                    if cmd == F_SETFL && arg >= 0 {
                        let sock_tmp = sockfdobj.handle.clone();
                        let mut sockhandle = sock_tmp.write();

                        if let Some(ins) = &mut sockhandle.innersocket {
                            let fcntlret;
                            if arg & O_NONBLOCK == O_NONBLOCK {
                                //set non-blocking I/O
                                fcntlret = ins.set_nonblocking();
                            } else {
                                //set blocking I/O
                                fcntlret = ins.set_blocking();
                            }
                            if fcntlret < 0 {
                                match Errno::from_discriminant(interface::get_errno()) {
                                    Ok(i) => {
                                        return syscall_error(
                                            i,
                                            "fcntl",
                                            "The libc call to fcntl failed!",
                                        );
                                    }
                                    Err(()) => panic!("Unknown errno value from fcntl returned!"),
                                };
                            }
                        }
                    }

                    &mut sockfdobj.flags
                }
            };

            //matching the tuple
            match (cmd, arg) {
                //because the arg parameter is not used in certain commands, it can be anything (..)
                //F_GETFD returns file descriptor flags only, meaning that access mode flags
                //and file status flags are excluded
                //F_SETFD is used to set file descriptor flags only, meaning that any changes to access mode flags
                //or file status flags should be ignored
                //currently, O_CLOEXEC is the only defined file descriptor flag, thus only this flag is
                //masked when using F_GETFD or F_SETFD
                (F_GETFD, ..) => *flags & O_CLOEXEC,
                (F_SETFD, arg) if arg >= 0 => {
                    if arg & O_CLOEXEC != 0 {
                        //if O_CLOEXEC flag is set to 1 in 'arg', 'flags' is updated by setting its O_CLOEXEC bit to 1
                        *flags |= O_CLOEXEC;
                    } else {
                        //if O_CLOEXEC flag is set to 0 in 'arg', 'flags' is updated by setting its O_CLOEXEC bit to 0
                        *flags &= !O_CLOEXEC;
                    }
                    0
                }
                //F_GETFL should return file access mode and file status flags, which means that
                //file creation flags should be masked out
                (F_GETFL, ..) => *flags & !(O_CREAT | O_EXCL | O_NOCTTY | O_TRUNC),
                //F_SETFL is used to set file status flags, thus any changes to file access mode and file
                //creation flags should be ignored (see F_SETFL command in the man page for fcntl for the reference)
                (F_SETFL, arg) if arg >= 0 => {
                    //valid changes are extracted by ignoring changes to file access mode and file creation flags
                    let valid_changes =
                        arg & !(O_RDWRFLAGS | O_CREAT | O_EXCL | O_NOCTTY | O_TRUNC);
                    //access mode and creation flags are extracted and other flags are set to 0 to update them
                    let acc_and_creation_flags =
                        *flags & (O_RDWRFLAGS | O_CREAT | O_EXCL | O_NOCTTY | O_TRUNC);
                    //valid changes are combined with the old file access mode and file creation flags
                    *flags = valid_changes | acc_and_creation_flags;
                    0
                }
                (F_DUPFD, arg) if arg >= 0 => self._dup2_helper(&filedesc_enum, arg, false),
                //TO DO: F_GETOWN and F_SETOWN commands are not implemented yet
                (F_GETOWN, ..) => 0,
                (F_SETOWN, arg) if arg >= 0 => 0,
                _ => {
                    let err_msg = format!(
                        "Arguments pair ({}, {}) does not match implemented parameters",
                        cmd, arg
                    );
                    syscall_error(Errno::EINVAL, "fcntl", &err_msg)
                }
            }
        } else {
            syscall_error(Errno::EBADF, "fcntl", "File descriptor is out of range")
        }
    }

    /// ### Description
    ///
    /// The `ioctl_syscall()` manipulates the underlying device parameters of
    /// special files. In particular, it is used as a way for user-space
    /// applications to interface with device drivers.
    ///
    /// ### Arguments
    ///
    /// The `ioctl_syscall()` accepts three arguments:
    /// * `fd` - an open file descriptor that refers to a device.
    /// * `request` - the control function to be performed. The set of valid
    ///   request values depends entirely on the device being addressed.
    ///   MEDIA_IOC_DEVICE_INFO is an example of an ioctl control function to
    ///   query device information that all media devices must support.
    /// * `ptrunion` - additional information needed by the addressed device to
    ///   perform the selected control function. In the example of
    ///   MEDIA_IOC_DEVICE_INFO request, a valid ptrunion value is a pointer to
    ///   a struct media_device_info, from which the device information is
    ///   obtained.
    ///
    /// ### Returns
    ///
    /// Upon successful completion, a value other than -1 that depends on the
    /// selected control function is returned. In case of a failure, -1 is
    /// returned with errno set to a particular value, like EBADF, EINVAL, etc.
    ///
    /// ### Errors and Panics
    ///
    /// * `EBADF` - fd is not a valid file descriptor
    /// * `EFAULT` - ptrunion references an inaccessible memory area
    /// * `EINVAL` - request or ptrunion is not valid
    /// * `ENOTTY` - fd is not associated with a character special device
    /// When `ioctl_syscall() is called on a Socket with `FIONBIO` control
    /// function, an underlying call to `libc::fcntl()` is made,
    /// which can return with an error. For a complete list of possible erorrs,
    /// see [fcntl(2)](https://linux.die.net/man/2/fcntl)
    ///
    /// A panic occurs either when a provided file descriptor is out of bounds
    /// or when an underlying call to `libc::fcntl()` for Socket type is
    /// returned with an unknown error.
    ///
    /// To learn more about the syscall, control functions applicable to all the
    /// devices, and possible error values, see [ioctl(2)](https://man.openbsd.org/ioctl)

    pub fn ioctl_syscall(&self, fd: i32, request: u32, ptrunion: IoctlPtrUnion) -> i32 {
        //BUG
        //if the provided file descriptor is out of bounds, 'get_filedescriptor'
        // returns Err(), unwrapping on which  produces a 'panic!'
        //otherwise, file descriptor table entry is stored in 'checkedfd'
        let checkedfd = self.get_filedescriptor(fd).unwrap();
        let mut unlocked_fd = checkedfd.write();
        //if a table descriptor entry is non-empty, a valid request is performed
        if let Some(filedesc_enum) = &mut *unlocked_fd {
            //For now, the only implemented control function is FIONBIO command used with
            // sockets
            match request {
                //for FIONBIO, 'ptrunion' stores a pointer to an integer. If the integer is 0, the
                // socket's nonblocking I/O is cleared. Otherwise, the socket is set
                // for nonblocking I/O
                FIONBIO => {
                    //if 'ptrunion' stores a Null pointer, a 'Bad address' error is returned
                    //otheriwse, the integer value stored in that address is returned and saved
                    // into 'arg_result'
                    let arg_result = interface::get_ioctl_int(ptrunion);
                    match (arg_result, filedesc_enum) {
                        (Err(arg_result), ..)=> {
                            return arg_result;
                        }
                        //since FIONBIO command is used with sockets, we need to make sure that the provided
                        //file descriptor addresses a socket
                        //otherwise, a 'Not a typewriter' error designating that the specified command
                        //is only applicable to sockets is returned 
                        (Ok(arg_result), Socket(ref mut sockfdobj)) => {
                            let sock_tmp = sockfdobj.handle.clone();
                            let mut sockhandle = sock_tmp.write();
                            let flags = &mut sockfdobj.flags;
                            let arg: i32 = arg_result;
                            let mut ioctlret = 0;
                            //clearing nonblocking I/O on the socket if the integer is 0
                            if arg == 0 {
                                *flags &= !O_NONBLOCK;
                                //libc::fcntl is called under the hood with F_SETFL command and 0 as an argument
                                //to set blocking I/O, and the result of the call is stored in ioctlret
                                if let Some(ins) = &mut sockhandle.innersocket {

                                    ioctlret = ins.set_blocking();
                                }
                            } else {
                                *flags |= O_NONBLOCK;
                                //libc::fcntl is called under the hood with F_SETFL command ans O_NONBLOCK as an argument
                                //to set nonblocking I/O, and the result of the call is stored in ioctlret
                                if let Some(ins) = &mut sockhandle.innersocket {
                                    ioctlret = ins.set_nonblocking();
                                }
                            }
                            //if ioctlret is negative, it means that the call to fcntl returned with an error
                            if ioctlret < 0 {
                                match Errno::from_discriminant(interface::get_errno()) {
                                    Ok(i) => {return syscall_error(i, "ioctl", "The libc call to ioctl failed!");},
                                    Err(()) => panic!("Unknown errno value from ioctl returned!"),
                                };
                            }

                            0
                        }
                        _ => {syscall_error(Errno::ENOTTY, "ioctl", "The specified request does not apply to the kind of object that the file descriptor fd references.")}
                    }
                }
                FIOASYNC => {
                    //not implemented
                    interface::log_verbose(
                        "ioctl(FIOASYNC) is not implemented, and just returns 0.",
                    );
                    0
                }
                _ => syscall_error(
                    Errno::EINVAL,
                    "ioctl",
                    "Arguments provided do not match implemented parameters",
                ),
            }
        } else {
            syscall_error(Errno::EBADF, "ioctl", "Invalid file descriptor")
        }
    }

    /// ### Description
    ///
    /// The `_chmod_helper()` is a helper function used by both
    /// `chmod_syscall()` and `fchmod_syscall()` to change mode bits that
    /// consist of read, write, and execute file permission bits of a file
    /// specified by an inode obtained from the corresponding caller
    /// syscall.
    ///
    /// ### Arguments
    ///
    /// The `_chmod_helper()` accepts two arguments:
    /// * `inodenum` - an inode of a file whose mode bits we are willing to
    /// change obtained from the caller syscall.
    /// * `mode` - the new file mode, which is a bit mask created by
    /// bitwise-or'ing zero or more valid mode bits. Some of the examples of
    /// such bits are `S_IRUSR` (read by owner), `S_IWUSR` (write by owner),
    /// etc.
    ///
    /// ### Returns
    ///
    /// Upon successful completion, zero is returned.
    /// In case of a failure, an error is returned, and `errno` is set depending
    /// on the error, e.g. EACCES, ENOENT, etc.
    ///
    /// ### Errors
    ///
    /// Currently, only one error is supported:
    /// * `EINVAL` - the value of the mode argument is invalid.
    /// Other errors, like `EFAULT`, `ENOTDIR`, etc. are not supported.
    ///
    /// ### Panics
    ///
    /// There are no cases where this helper function panics.

    pub fn _chmod_helper(inodenum: usize, mode: u32) -> i32 {
        //S_IRWXA is a result of bitwise-or'ing read, write, and execute or search
        //permissions for the file owner, group owners,
        //and other users. It encompasses all the mode bits that can be changed
        //via `chmod_syscall()` and is used as a bitmask to make sure that no
        //other invalid bit change is being made.
        if (mode & S_IRWXA) == mode {
            //getting a mutable reference to an inode struct that corresponds to
            //the file whose mode bits we want to change
            let mut thisinode = FS_METADATA.inodetable.get_mut(&inodenum).unwrap();
            //log is used to store all the changes made to the filesystem. After
            //the cage is closed, all the collected changes are serialized and
            //the state of the underlying filsystem is persisted. This allows us
            //to avoid serializing and persisting filesystem state after every
            //`chmod_syscall()`.
            let mut log = true;
            //We obtain the mode bits that should remain intact by bitwise-and'ing
            //the inode's mode bits with the set of bits that can be changed via
            //`chmod_syscall`. The changes are applied by bitwise-or'ing
            //the intact mode bits with the changed mode bits.
            match *thisinode {
                Inode::File(ref mut general_inode) => {
                    general_inode.mode = (general_inode.mode & !S_IRWXA) | mode;
                }
                Inode::CharDev(ref mut dev_inode) => {
                    dev_inode.mode = (dev_inode.mode & !S_IRWXA) | mode;
                }
                Inode::Socket(ref mut sock_inode) => {
                    sock_inode.mode = (sock_inode.mode & !S_IRWXA) | mode;
                    //Sockets only exist as long as the cages using them are running.
                    //After these cages are closed, no changes to sockets' inodes
                    //need to be persisted, thus using log is unnecessary.
                    log = false;
                }
                Inode::Dir(ref mut dir_inode) => {
                    dir_inode.mode = (dir_inode.mode & !S_IRWXA) | mode;
                }
            }
            //the mutable reference to the inode has to be dropped because
            //`log_metadata` will need to acquire an immutable reference to
            //the same inode
            drop(thisinode);
            //changes to an inode are saved into the log for all file types
            //except for Sockets
            if log {
                log_metadata(&FS_METADATA, inodenum);
            };
            //return 0 on success
            0
        } else {
            return syscall_error(
                Errno::EINVAL,
                "chmod",
                "The value of the mode argument is invalid",
            );
        }
    }

    /// ### Description
    ///
    /// The `chmod_syscall()` changes a file's mode bits that consist of read,
    /// write, and execute file permission bits.
    /// Changing `set-user-ID`, `set-group-ID`, and sticky bits is currently
    /// not supported.
    ///
    /// ### Arguments
    ///
    /// The `chmod_syscall()` accepts two arguments:
    /// * `path` - pathname of the file whose mode bits we are willing to
    /// change (symbolic links are currently not supported). If the
    /// pathname is relative, then it is interpreted relative to the
    /// current working directory of the calling process.
    /// * `mode` - the new file mode, which is a bit mask created by
    /// bitwise-or'ing zero or more valid mode bits. Some of the examples
    /// of such bits are `S_IRUSR` (read by owner), `S_IWUSR` (write by owner),
    /// etc.
    ///
    /// ### Returns
    ///
    /// Upon successful completion, zero is returned.
    /// In case of a failure, an error is returned, and `errno` is set depending
    /// on the error, e.g. `EACCES`, `ENOENT`, etc.
    ///
    /// ### Errors
    ///
    /// Currently, only two errors are supposrted:
    /// * `EINVAL` - the value of the mode argument is invalid
    /// * `ENOENT` - a component of path does not name an existing file
    /// Other errors, like `EFAULT`, `ENOTDIR`, etc. are not supported.
    ///
    /// ### Panics
    ///
    /// There are no cases where this syscall panics.
    ///
    /// To learn more about the syscall, valid mode bits, and error values, see
    /// [chmod(2)](https://man7.org/linux/man-pages/man2/chmod.2.html)

    pub fn chmod_syscall(&self, path: &str, mode: u32) -> i32 {
        //Convert the provided pathname into an absolute path without `.` or `..`
        //components.
        let truepath = normpath(convpath(path), self);
        //Perfrom a walk down the file tree starting from the root directory to
        //obtain an inode number of the file whose pathname was specified.
        //`None` is returned if one of the following occurs while moving down
        //the tree: accessing a child of a non-directory inode, accessing a
        //child of a nonexistent parent directory, accessing a nonexistent child,
        //accessing an unexpected component, like `.` or `..` directory reference.
        //In this case, `The file does not exist` error is returned.
        //Otherwise, a `Some()` option containing the inode number is returned.
        if let Some(inodenum) = metawalk(truepath.as_path()) {
            Self::_chmod_helper(inodenum, mode)
        } else {
            return syscall_error(
                Errno::ENOENT,
                "chmod",
                "A component of path does not name an existing file",
            );
        }
    }

    /// ### Description
    ///
    /// The `fchmod_syscall()` is equivalent to `chmod_syscall()` in that
    /// it is used to change a file's mode bits that consist of read,
    /// write, and execute file permission bits except that the file
    /// is specified by the file descriptor. Changing `set-user-ID`,
    /// `set-group-ID`, and sticky bits is currently not supported.
    ///
    /// ### Arguments
    ///
    /// The `fchmod_syscall()` accepts two arguments:
    /// * `fd` - an open file descriptor.
    /// * `mode` - the new file mode, which is a bit mask created by
    /// bitwise-or'ing zero or more valid mode bits. Some of the examples
    /// of such bits are `S_IRUSR` (read by owner), `S_IWUSR`
    /// (write by owner), etc.
    ///
    /// ### Returns
    ///
    /// Upon successful completion, zero is returned.
    /// In case of a failure, an error is returned, and `errno` is set
    /// depending on the error, e.g. `EACCES`, `ENOENT`, etc.
    ///
    /// ### Errors
    ///
    /// * `EBADF` - the file descriptor `fd` is not valid.
    /// * `EINVAL` - the value of the `mode` argument is invalid or
    /// mode bits cannot be changed on this file type
    /// Other errors, like `EFAULT`, `ENOTDIR`, etc. are not supported.
    ///
    /// ### Panics
    ///
    /// A panic occurs when a provided file descriptor is out of bounds
    ///
    /// To learn more about the syscall, valid mode bits, and error values, see
    /// [fchmod(2)](https://linux.die.net/man/2/fchmod)

    pub fn fchmod_syscall(&self, fd: i32, mode: u32) -> i32 {
        //BUG
        //if the provided file descriptor is out of bounds, 'get_filedescriptor'
        //returns `Err()`, unwrapping on which  produces a `panic!`
        //otherwise, file descriptor table entry is stored in `checkedfd`
        let checkedfd = self.get_filedescriptor(fd).unwrap();
        let unlocked_fd = checkedfd.read();
        //if a table descriptor entry is non-empty, a valid request is performed
        if let Some(filedesc_enum) = &*unlocked_fd {
            //Regular file type is the only type that supports `fchmod_syscall()`
            match filedesc_enum {
                File(normalfile_filedesc_obj) => {
                    let inodenum = normalfile_filedesc_obj.inode;
                    Self::_chmod_helper(inodenum, mode)
                }
                Socket(_) => {
                    return syscall_error(
                        Errno::EINVAL,
                        "fchmod",
                        "Mode bits cannot be changed on this file type",
                    );
                }
                Stream(_) => {
                    return syscall_error(
                        Errno::EINVAL,
                        "fchmod",
                        "Mode bits cannot be changed on this file type",
                    );
                }
                Pipe(_) => {
                    return syscall_error(
                        Errno::EINVAL,
                        "fchmod",
                        "Mode bits cannot be changed on this file type",
                    );
                }
                Epoll(_) => {
                    return syscall_error(
                        Errno::EINVAL,
                        "fchmod",
                        "Mode bits cannot be changed on this file type",
                    );
                }
            }
        } else {
            return syscall_error(Errno::EBADF, "ioctl", "Invalid file descriptor");
        }
    }

    /// ### Description
    ///
    /// The `mmap_syscall()` function creates a new mapping in the
    /// virtual address space of the calling process.
    ///
    /// ### Arguments
    ///
    /// The `mmap_syscall()` accepts six arguments:
    /// * `addr` - the starting address for the new mapping. If `addr`
    /// is NULL, then the kernel chooses the (page-aligned) address at
    /// which to create the mapping. If addr is not NULL, then the
    /// kernel takes it as a hint about where to place the mapping.
    /// * `len` - specifies the length of the mapping (which must be
    /// greater than 0).
    /// * `prot` - describes the desired memory protection of the
    /// mapping, which must not conflict with the open mode of the file.
    /// It is either `PROT_NONE` or the bitwise OR of one or more of the
    /// following flags: `PROT_EXEC` (Pages may be executed), `PROT_READ`
    /// (Pages may be read), `PROT_WRITE` (Pages may be written), `PROT_NONE`
    /// (Pages may not be accessed).
    /// * `flags` - determines whether updates to the mapping are visible
    /// to other processes mapping the same region, and whether updates are
    /// carried through to the underlying file. This behavior is determined
    /// by including exactly one of the following values in flags: `MAP_SHARED`
    /// (Share this mapping. Updates to the mapping are visible to other
    /// processes mapping the same region, and in the case of file-backed
    /// mappings are carried through to the underlying file) or `MAP_PRIVATE`
    /// (Create a private copy-on-write mapping. Updates to the mapping are
    /// not visible to other processes mapping the same file, and are not
    /// carried through to the underlying file). `MAP_SHARED_VALIDATE` and
    /// other flags are not validated in the current implementation but
    /// are supported by the underlying `libc_mmap()` syscall.
    /// * `filedes` - a file descriptor specifying the file that shall be
    /// mapped.
    /// * `off` - designates the offset in the file from which the mapping
    /// should start.
    ///
    /// ### Returns
    ///
    /// On success, `mmap_syscall()` returns a pointer to the mapped area.
    /// In case of a failure, an error is returned, and `errno` is set depending
    /// on the error, e.g. EINVAL, ERANGE, etc.
    ///
    /// ### Errors
    ///
    /// * `EINVAL` - the value of len is 0 or `flags` contained neither
    /// `MAP_PRIVATE` nor `MAP_SHARED` or `flags` contained both
    /// `MAP_PRIVATE` and `MAP_SHARED`.
    /// * `EACCES` - `fildes` is not open for reading or `MAP_SHARED`
    /// was requested and PROT_WRITE is set, but fd is not open in
    /// read/write (`O_RDWR`) mode or `fildes` refers to a non-regular file.
    /// * `ENXIO` - addresses in the range [`off`, `off`+`len`) are invalid
    /// for the object specified by `fildes`.
    /// * `EOPNOTSUPP` - Lind currently does not support mapping character
    ///   files.
    /// * `EBADF` - invalid file descriptor.
    /// Other errors, like `ENOMEM`, `EOVERFLOW`, etc. are not supported.
    ///
    /// ### Panics
    ///
    /// A panic occurs when a provided file descriptor is out of bounds.
    ///
    /// To learn more about the syscall, flags, possible error values, etc., see
    /// [mmap(2)](https://man7.org/linux/man-pages/man2/mmap.2.html)

    pub fn mmap_syscall(
        &self,
        addr: *mut u8,
        len: usize,
        prot: i32,
        flags: i32,
        fildes: i32,
        off: i64,
    ) -> i32 {
        if len == 0 {
            return syscall_error(Errno::EINVAL, "mmap", "the value of len is 0");
        }
        //Exactly one of the two flags (either `MAP_PRIVATE` or `MAP_SHARED`) must be
        // set
        if 0 == (flags & (MAP_PRIVATE | MAP_SHARED)) {
            return syscall_error(
                Errno::EINVAL,
                "mmap",
                "The value of flags is invalid (neither MAP_PRIVATE nor MAP_SHARED is set)",
            );
        }
        if ((flags & MAP_PRIVATE) != 0) && ((flags & MAP_SHARED) != 0) {
            return syscall_error(
                Errno::EINVAL,
                "mmap",
                "The value of flags is invalid (MAP_PRIVATE and MAP_SHARED cannot be both set)",
            );
        }
        //The `MAP_ANONYMOUS` flag specifies that the mapping
        //is not backed by any file, so the `fildes` and `off`
        //arguments should be ignored; however, some implementations
        //require `fildes` to be -1, which we follow for the
        //sake of portability.
        if 0 != (flags & MAP_ANONYMOUS) {
            return interface::libc_mmap(addr, len, prot, flags, -1, 0);
        }
        //BUG
        //If the provided file descriptor is out of bounds, get_filedescriptor returns
        //Err(), unwrapping on which  produces a 'panic!'
        //otherwise, file descriptor table entry is stored in 'checkedfd'
        let checkedfd = self.get_filedescriptor(fildes).unwrap();
        let mut unlocked_fd = checkedfd.write();
        if let Some(filedesc_enum) = &mut *unlocked_fd {
            //The current implementation supports only regular files
            match filedesc_enum {
                File(ref mut normalfile_filedesc_obj) => {
                    let inodeobj = FS_METADATA
                        .inodetable
                        .get(&normalfile_filedesc_obj.inode)
                        .unwrap();
                    //Confirm inode type is mappable
                    match &*inodeobj {
                        Inode::CharDev(_chardev_inode_obj) => {
                            syscall_error(Errno::EOPNOTSUPP, "mmap", "lind currently does not support mapping character files")
                        }
                        Inode::File(normalfile_inode_obj) => {
                            //For any kind of memory mapping, the file should be
                            //opened for reading, so if it was opened for write
                            //only, the mapping should be denied
                            if (normalfile_filedesc_obj.flags & O_WRONLY) != 0 {
                                return syscall_error(Errno::EACCES, "mmap", "file descriptor is not open for reading");
                            }
                            //If we want to write our changes back to the file the file needs to be open for reading and writing
                            if (flags & MAP_SHARED) != 0 && (prot & PROT_WRITE) != 0 && (normalfile_filedesc_obj.flags & O_RDWR) != O_RDWR {
                                return syscall_error(Errno::EACCES, "mmap", "file descriptor is not open RDWR, but MAP_SHARED and PROT_WRITE are set");
                            }
                            let filesize = normalfile_inode_obj.size;
                            //The offset cannot be negative, and we cannot read past the end of the file
                            if off < 0 || off > filesize as i64 {
                                return syscall_error(Errno::ENXIO, "mmap", "Addresses in the range [off,off+len) are invalid for the object specified by fildes.");
                            }
                            //Because of NaCl's internal workings we must allow mappings to extend past the end of the file
                            let fobj = FILEOBJECTTABLE.get(&normalfile_filedesc_obj.inode).unwrap();
                            //The actual memory mapping is not emulated inside Lind, so the call to the kernel
                            //is required. To perform this call, the file descriptor of the actual file
                            //stored on the host machine is needed. Since Lind's emulated filesystem
                            //does not match the underlying host's filesystem, the file descriptor
                            //provided to the `mmap_syscall()` cannot be used and must be converted
                            //to the actual file descriptor stored in the host's filesystem.
                            let fobjfdno = fobj.as_fd_handle_raw_int();
                            interface::libc_mmap(addr, len, prot, flags, fobjfdno, off)
                        }
                        _ => {syscall_error(Errno::EACCES, "mmap", "the fildes argument refers to a file whose type is not supported by mmap")}
                    }
                }
                _ => syscall_error(
                    Errno::EACCES,
                    "mmap",
                    "the fildes argument refers to a file whose type is not supported by mmap",
                ),
            }
        } else {
            syscall_error(Errno::EBADF, "mmap", "invalid file descriptor")
        }
    }

    /// ### Description
    ///
    /// The `munmap_syscall()` function shall remove any mappings
    /// containing any part of the address space of the process
    /// starting at `addr` and continuing for `len` bytes.
    /// Further references to these pages shall result in the
    /// generation of a `SIGSEGV` signal to the process. If there
    /// are no mappings in the specified address range, then
    /// `munmap_syscall()` has no effect.
    /// The current implementation of the syscall solely relies
    /// on the inner implementation of NaCl (except for a simple
    /// `len` argument check) by creating a new mapping in the
    /// specified memory region by using `MAP_FIXED` with
    /// `PROT_NONE` flag to deny any access to the unmapped
    /// memory region.
    ///
    /// ### Arguments
    ///
    /// The `munmap_syscall()` accepts two arguments:
    /// * `addr` - the address starting from which the mapping
    /// shall be removed
    /// * `len` - specifies the length of the mapping that
    /// shall be removed.
    ///
    /// ### Returns
    ///
    /// On success, `munmap_syscall()` returns 0.
    /// In case of a failure, an error is returned, and `errno`
    /// is set to `EINVAL`.
    ///
    /// ### Errors
    ///
    /// * `EINVAL` - the value of len is 0
    /// Other `EINVAL` errors are returned directly from the  call to
    /// `libc_mmap`
    ///
    /// ### Panics
    ///
    /// There are no cases where this function panics.
    ///
    /// To learn more about the syscall, flags, possible error values, etc., see
    /// [munmap(2)](https://linux.die.net/man/2/munmap)

    pub fn munmap_syscall(&self, addr: *mut u8, len: usize) -> i32 {
        if len == 0 {
            return syscall_error(Errno::EINVAL, "mmap", "the value of len is 0");
        }
        //NaCl's munmap implementation actually just writes
        //over the previously mapped data with PROT_NONE.
        //This frees all of the resources except page table
        //space, and is put inside safeposix for consistency.
        //`MAP_FIXED` is used to precisely unmap the specified
        //memory region, and `MAP_ANONYMOUS` is used to deny
        //any further access to the unmapped memory region
        //thereby emulating the unmapping process.
        interface::libc_mmap(
            addr,
            len,
            PROT_NONE,
            MAP_PRIVATE | MAP_ANONYMOUS | MAP_FIXED,
            -1,
            0,
        )
    }

    //------------------------------------FLOCK SYSCALL------------------------------------

    pub fn flock_syscall(&self, fd: i32, operation: i32) -> i32 {
        let checkedfd = self.get_filedescriptor(fd).unwrap();
        let unlocked_fd = checkedfd.read();
        if let Some(filedesc_enum) = &*unlocked_fd {
            let lock = match filedesc_enum {
                File(normalfile_filedesc_obj) => &normalfile_filedesc_obj.advlock,
                Socket(socket_filedesc_obj) => &socket_filedesc_obj.advlock,
                Stream(stream_filedesc_obj) => &stream_filedesc_obj.advlock,
                Pipe(pipe_filedesc_obj) => &pipe_filedesc_obj.advlock,
                Epoll(epoll_filedesc_obj) => &epoll_filedesc_obj.advlock,
            };
            match operation & (LOCK_SH | LOCK_EX | LOCK_UN) {
                LOCK_SH => {
                    if operation & LOCK_NB == LOCK_NB {
                        //EAGAIN and EWOULDBLOCK are the same
                        if !lock.try_lock_sh() {
                            return syscall_error(
                                Errno::EAGAIN,
                                "flock",
                                "shared lock would block",
                            );
                        };
                    } else {
                        lock.lock_sh();
                    }
                }
                LOCK_EX => {
                    if operation & LOCK_NB == LOCK_NB {
                        if !lock.try_lock_ex() {
                            return syscall_error(
                                Errno::EAGAIN,
                                "flock",
                                "exclusive lock would block",
                            );
                        };
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
                _ => {
                    return syscall_error(Errno::EINVAL, "flock", "unknown operation");
                }
            }
            0 //flock has  succeeded!
        } else {
            syscall_error(Errno::EBADF, "flock", "invalid file descriptor")
        }
    }

    /// ### Description
    ///
    /// The `remove_from_parent_dir()` is a helper function used by a couple
    /// of syscalls to remove a file from its parent directory's inode. It
    /// ensures that the parent directory has the appropriate permissions
    /// before removing the file entry and updating the parent directory's
    /// metadata.
    ///
    /// ### Arguments
    ///
    /// The `remove_from_parent_dir()` accepts two arguments:
    /// * `parent_inodenum` - an inode number of the parent directory from which
    /// the file is to be removed.
    /// * `truepath` - the absolute path of the file to be removed, used to
    ///   identify
    /// the filename within the parent directory.
    ///
    /// ### Returns
    ///
    /// Upon successful completion, zero is returned. In case of a failure, an
    /// error is returned, and `errno` is set depending on the error, e.g.,
    /// EPERM.
    ///
    /// ### Errors
    ///
    /// Currently, the following error is supported:
    /// * `EPERM` - the parent directory does not have write permission.
    ///
    /// ### Panics
    ///
    /// This function will panic if the `parent_inodenum` does not correspond to
    /// a directory inode.
    pub fn remove_from_parent_dir(
        parent_inodenum: usize,
        truepath: &interface::RustPathBuf,
    ) -> i32 {
        // Get the inode of the parent directory and ensure it is a directory
        if let Inode::Dir(ref mut parent_dir) =
            *(FS_METADATA.inodetable.get_mut(&parent_inodenum).unwrap())
        {
            // check if parent directory has write permissions
            if parent_dir.mode as u32 & (S_IWOTH | S_IWGRP | S_IWUSR) == 0 {
                return syscall_error(
                    Errno::EPERM,
                    "rmdir",
                    "Parent directory does not have write permission",
                );
            }

            // remove entry of corresponding filename from filename-inode dict
            parent_dir
                .filename_to_inode_dict
                .remove(&truepath.file_name().unwrap().to_str().unwrap().to_string())
                .unwrap();
            // Decrement the link count of the parent directory
            parent_dir.linkcount -= 1;
        } else {
            // Panic if the parent inode is not a directory
            panic!("Non directory file was parent!");
        }
        0
    }

    /// ### Description
    ///
    /// The `rmdir_syscall()` deletes a directory whose name is given by `path`.
    /// The directory shall be removed only if it is an empty directory.
    ///
    /// ### Arguments
    ///
    /// The `rmdir_syscall()` accepts one argument:
    /// * `path` - the path to the directory that shall be removed. It can be
    ///   either
    /// relative or absolute (symlinks are not supported).
    ///
    /// ### Returns
    ///
    /// Upon successful completion, 0 is returned.
    /// In case of a failure, an error is returned, and `errno` is set depending
    /// on the error, e.g. EACCES, ENOENT, etc.
    ///
    /// ### Errors
    ///
    /// * `ENOENT` - `path` is an empty string or names a nonexistent directory
    /// * `EBUSY` - `path` names a root directory that cannot be removed
    /// * `ENOEMPTY` - `path` names a non-empty directory,
    /// * `EPERM` - the directory to be removed or its parent directory
    /// does not allow write permission
    /// * `ENOTDIR` - `path` is not a directory
    /// Other errors, like `EACCES`, `EINVAL`, etc. are not supported.
    ///
    /// ### Panics
    /// A panic occurs when the directory to be removed does not have `S_IFDIR"`
    /// (directory file type flag) set or when parent inode is not a directory.
    ///
    /// To learn more about the syscall, error values, etc.,
    /// see [rmdir(3)](https://linux.die.net/man/3/rmdir)

    pub fn rmdir_syscall(&self, path: &str) -> i32 {
        if path.len() == 0 {
            return syscall_error(Errno::ENOENT, "rmdir", "Given path is an empty string");
        }
        //Convert the provided pathname into an absolute path without `.` or `..`
        //components.
        let truepath = normpath(convpath(path), self);

        //Perfrom a walk down the file tree starting from the root directory to
        //obtain an inode number of the file whose pathname was specified and
        //its parent directory's inode.
        match metawalkandparent(truepath.as_path()) {
            (None, ..) => syscall_error(Errno::ENOENT, "rmdir", "Path does not exist"),
            //The specified directory exists, but its parent does not,
            //which means it is a root directory that cannot be removed
            (Some(_), None) => syscall_error(Errno::EBUSY, "rmdir", "Cannot remove root directory"),
            (Some(inodenum), Some(parent_inodenum)) => {
                //If the parent directory of the directory that shall be removed
                //doesn't allow write permission, the removal cannot be performed
                if let Inode::Dir(ref mut parent_dir) =
                    *(FS_METADATA.inodetable.get_mut(&parent_inodenum).unwrap())
                {
                    // check if parent directory has write permissions
                    if parent_dir.mode as u32 & (S_IWOTH | S_IWGRP | S_IWUSR) == 0 {
                        return syscall_error(
                            Errno::EPERM,
                            "rmdir",
                            "Parent directory does not have write permission",
                        );
                    }
                }
                //Getting a mutable reference to an inode struct that corresponds to
                //the directory that shall be removed
                let mut inodeobj = FS_METADATA.inodetable.get_mut(&inodenum).unwrap();

                match &mut *inodeobj {
                    //A sanity check to make sure the inode matches a directory
                    Inode::Dir(ref mut dir_obj) => {
                        //When a new empty directory is created, its linkcount
                        //is set to 3. Thus, any empty directory should have a
                        //linkcount of 3. Otherwise, it is a non-empty directory
                        //that cannot be removed
                        if dir_obj.linkcount > 3 {
                            return syscall_error(
                                Errno::ENOTEMPTY,
                                "rmdir",
                                "Directory is not empty",
                            );
                        }
                        //A sanity check to make sure that the correct directory
                        //file type flag is set
                        if !is_dir(dir_obj.mode) {
                            panic!("This directory does not have its mode set to S_IFDIR");
                        }

                        //The directory to be removed should allow write permission
                        if dir_obj.mode as u32 & (S_IWOTH | S_IWGRP | S_IWUSR) == 0 {
                            return syscall_error(
                                Errno::EPERM,
                                "rmdir",
                                "Directory does not allow write permission",
                            );
                        }

                        //The directory cannot be removed if one or more processes
                        //have the directory open, which corresponds to a non-zero
                        //reference count
                        let remove_inode = dir_obj.refcount == 0;
                        //Any new empty directory has a linkcount of 3.
                        //If the refcount of the directory is 0, it means
                        //that there are no open file descriptors for that directory,
                        //and it can be safely removed both from the filesystem
                        //and the parent directory's inode table.
                        //However, if there exists an open file descriptor for that
                        //directory, it cannot be removed from the filesystem because
                        //otherwise, the process that has the directory open
                        //will end up calling `close_syscall()` on a nonexistent directory.
                        //At the same time, after `rmdir_syscall()` is called on the directory,
                        //no new files can be created inside it, even if the directory is open
                        //by some process. To prevent creating new files inside the directory,
                        //we delete its entry from the parent directory's inode table.
                        //This way, in case there is an open file descriptor for the directory
                        //to be removed, by deleting the directory's entry from its parent
                        //directory's inode table and keeping the directory's entry in the
                        //filesystem table, we both disallow the creation of any files inside
                        //that directory and prevent the process that has the directory
                        //open from closing a nonexistent directory.
                        //Setting the directory's linkcount as 2 works as a flag for
                        //the `close_syscall()` to mark the directory that needs to be
                        //removed from the filesystem when its last open file descriptor
                        //is closed because it could not be removed at the time of calling
                        //`rmdir_syscall()` because of some open file descriptor.
                        if remove_inode {
                            dir_obj.linkcount = 2;
                        }

                        //The mutable reference to the inode has to be dropped because
                        //remove_from_parent_dir() method will need to acquire an immutable
                        //reference to the parent directory's inode from the filesystem's
                        //inodetable. The inodetable represents a Rust DashMap that deadlocks
                        //when trying to get a reference to its entry while holding any sort
                        //of reference into it.
                        drop(inodeobj);

                        //`remove_from_parent_dir()` helper function returns 0 if an
                        //entry corresponding to the specified directory was
                        //successfully removed from the filename-inode dictionary
                        //of its parent.
                        //If the parent directory does not allow write permission,
                        //`EPERM` is returned.
                        //As a sanity check, if the parent inode specifies a
                        //non-directory type, the funciton panics
                        let removal_result =
                            Self::remove_from_parent_dir(parent_inodenum, &truepath);
                        if removal_result != 0 {
                            return removal_result;
                        }

                        //Remove entry of corresponding inodenum from the filesystem
                        //inodetable
                        if remove_inode {
                            FS_METADATA.inodetable.remove(&inodenum).unwrap();
                        }
                        //Log is used to store all the changes made to the filesystem. After
                        //the cage is closed, all the collected changes are serialized and
                        //the state of the underlying filesystem is persisted. This allows us
                        //to avoid serializing and persisting filesystem state after every
                        //`rmdir_syscall()`.
                        log_metadata(&FS_METADATA, parent_inodenum);
                        log_metadata(&FS_METADATA, inodenum);
                        0 // success
                    }
                    _ => syscall_error(Errno::ENOTDIR, "rmdir", "Path is not a directory"),
                }
            }
        }
    }

    //------------------RENAME SYSCALL------------------

    pub fn rename_syscall(&self, oldpath: &str, newpath: &str) -> i32 {
        if oldpath.len() == 0 {
            return syscall_error(Errno::ENOENT, "rename", "Old path is null");
        }
        if newpath.len() == 0 {
            return syscall_error(Errno::ENOENT, "rename", "New path is null");
        }

        let true_oldpath = normpath(convpath(oldpath), self);
        let true_newpath = normpath(convpath(newpath), self);

        // try to get inodenum of old path and its parent
        match metawalkandparent(true_oldpath.as_path()) {
            (None, ..) => syscall_error(Errno::EEXIST, "rename", "Old path does not exist"),
            (Some(_), None) => {
                syscall_error(Errno::EBUSY, "rename", "Cannot rename root directory")
            }
            (Some(inodenum), Some(parent_inodenum)) => {
                // make sure file is not moved to another dir
                // get inodenum for parent of new path
                let (_, new_par_inodenum) = metawalkandparent(true_newpath.as_path());
                // check if old and new paths share parent
                if new_par_inodenum != Some(parent_inodenum) {
                    return syscall_error(
                        Errno::EOPNOTSUPP,
                        "rename",
                        "Cannot move file to another directory",
                    );
                }

                let pardir_inodeobj = FS_METADATA.inodetable.get_mut(&parent_inodenum).unwrap();
                if let Inode::Dir(parent_dir) = &*pardir_inodeobj {
                    // add pair of new path and its inodenum to filename-inode dict
                    parent_dir.filename_to_inode_dict.insert(
                        true_newpath
                            .file_name()
                            .unwrap()
                            .to_str()
                            .unwrap()
                            .to_string(),
                        inodenum,
                    );

                    // remove entry of old path from filename-inode dict
                    parent_dir.filename_to_inode_dict.remove(
                        &true_oldpath
                            .file_name()
                            .unwrap()
                            .to_str()
                            .unwrap()
                            .to_string(),
                    );
                    drop(pardir_inodeobj);
                    log_metadata(&FS_METADATA, parent_inodenum);
                }
                NET_METADATA.domsock_paths.insert(true_newpath);
                NET_METADATA.domsock_paths.remove(&true_oldpath);
                0 // success
            }
        }
    }

    fn _truncate_helper(&self, inodenum: usize, length: isize, file_must_exist: bool) -> i32 {
        if length < 0 {
            return syscall_error(Errno::EINVAL, "truncate", "length specified as less than 0");
        }
        let mut inodeobj = FS_METADATA.inodetable.get_mut(&inodenum).unwrap();

        match *inodeobj {
            // only proceed when inode matches with a file
            Inode::File(ref mut normalfile_inode_obj) => {
                let ulength = length as usize;
                let filesize = normalfile_inode_obj.size as usize;

                // get file object table with write lock
                let mut maybe_fileobject = FILEOBJECTTABLE.entry(inodenum);
                let mut tempbind;
                let close_on_exit;

                //We check if the fileobject exists. If file_must_exist is true (i.e. we called
                // the helper from ftruncate) then we know that an fd must exist
                // and thus we panic if the fileobject does not
                // exist. If file_must_exist is false (i.e. we called the helper from truncate),
                // if the file does not exist,  we create a new fileobject to
                // use which we remove once we are done with it
                let fileobject = if let interface::RustHashEntry::Occupied(ref mut occ) =
                    maybe_fileobject
                {
                    close_on_exit = false;
                    occ.get_mut()
                } else if file_must_exist {
                    panic!("Somehow a normal file with an fd was truncated but there was no file object in rustposix?");
                } else {
                    let sysfilename = format!("{}{}", FILEDATAPREFIX, inodenum);
                    tempbind = interface::openfile(sysfilename, filesize).unwrap(); // open file with size given from inode
                    close_on_exit = true;
                    &mut tempbind
                };

                // if length is greater than original filesize,
                // file is extented with null bytes
                if filesize < ulength {
                    let blankbytecount = ulength - filesize;
                    if let Ok(byteswritten) = fileobject.zerofill_at(filesize, blankbytecount) {
                        if byteswritten != blankbytecount {
                            panic!("zerofill_at() has failed");
                        }
                    } else {
                        panic!("zerofill_at() has failed");
                    }
                } else {
                    // if length is smaller than original filesize,
                    // extra data are cut off
                    fileobject.shrink(ulength).unwrap();
                }

                if close_on_exit {
                    fileobject.close().unwrap();
                }

                drop(maybe_fileobject);

                normalfile_inode_obj.size = ulength;

                drop(inodeobj);
                log_metadata(&FS_METADATA, inodenum);
                0 // truncating has succeeded!
            }
            Inode::CharDev(_) => syscall_error(
                Errno::EINVAL,
                "truncate",
                "The named file is a character driver",
            ),
            Inode::Socket(_) => syscall_error(
                Errno::EINVAL,
                "truncate",
                "The named file is a domain socket",
            ),
            Inode::Dir(_) => {
                syscall_error(Errno::EISDIR, "truncate", "The named file is a directory")
            }
        }
    }

    //------------------------------------FSYNC SYSCALL------------------------------------

    pub fn fsync_syscall(&self, fd: i32) -> i32 {
        let checkedfd = self.get_filedescriptor(fd).unwrap();
        let mut unlocked_fd = checkedfd.write();
        if let Some(filedesc_enum) = &mut *unlocked_fd {
            match filedesc_enum {
                File(ref mut normalfile_filedesc_obj) => {
                    if is_rdonly(normalfile_filedesc_obj.flags) {
                        return syscall_error(
                            Errno::EBADF,
                            "fsync",
                            "specified file not open for sync",
                        );
                    }
                    let inodeobj = FS_METADATA
                        .inodetable
                        .get(&normalfile_filedesc_obj.inode)
                        .unwrap();
                    match &*inodeobj {
                        Inode::File(_) => {
                            let fileobject =
                                FILEOBJECTTABLE.get(&normalfile_filedesc_obj.inode).unwrap();

                            match fileobject.fsync() {
                                Ok(_) => 0,
                                _ => syscall_error(
                                    Errno::EIO,
                                    "fsync",
                                    "an error occurred during synchronization",
                                ),
                            }
                        }
                        _ => syscall_error(
                            Errno::EROFS,
                            "fsync",
                            "does not support special files for synchronization",
                        ),
                    }
                }
                _ => syscall_error(
                    Errno::EINVAL,
                    "fsync",
                    "fd is attached to an object which is unsuitable for synchronization",
                ),
            }
        } else {
            syscall_error(Errno::EBADF, "fsync", "invalid file descriptor")
        }
    }

    //------------------------------------FDATASYNC SYSCALL------------------------------------

    pub fn fdatasync_syscall(&self, fd: i32) -> i32 {
        let checkedfd = self.get_filedescriptor(fd).unwrap();
        let mut unlocked_fd = checkedfd.write();
        if let Some(filedesc_enum) = &mut *unlocked_fd {
            match filedesc_enum {
                File(ref mut normalfile_filedesc_obj) => {
                    if is_rdonly(normalfile_filedesc_obj.flags) {
                        return syscall_error(
                            Errno::EBADF,
                            "fdatasync",
                            "specified file not open for sync",
                        );
                    }
                    let inodeobj = FS_METADATA
                        .inodetable
                        .get(&normalfile_filedesc_obj.inode)
                        .unwrap();
                    match &*inodeobj {
                        Inode::File(_) => {
                            let fileobject =
                                FILEOBJECTTABLE.get(&normalfile_filedesc_obj.inode).unwrap();

                            match fileobject.fdatasync() {
                                Ok(_) => 0,
                                _ => syscall_error(
                                    Errno::EIO,
                                    "fdatasync",
                                    "an error occurred during synchronization",
                                ),
                            }
                        }
                        _ => syscall_error(
                            Errno::EROFS,
                            "fdatasync",
                            "does not support special files for synchronization",
                        ),
                    }
                }
                _ => syscall_error(
                    Errno::EINVAL,
                    "fdatasync",
                    "fd is attached to an object which is unsuitable for synchronization",
                ),
            }
        } else {
            syscall_error(Errno::EBADF, "fdatasync", "invalid file descriptor")
        }
    }

    //------------------------------------SYNC_FILE_RANGE SYSCALL------------------------------------

    pub fn sync_file_range_syscall(
        &self,
        fd: i32,
        offset: isize,
        nbytes: isize,
        flags: u32,
    ) -> i32 {
        let checkedfd = self.get_filedescriptor(fd).unwrap();
        let mut unlocked_fd = checkedfd.write();
        if let Some(filedesc_enum) = &mut *unlocked_fd {
            match filedesc_enum {
                File(ref mut normalfile_filedesc_obj) => {
                    let inodeobj = FS_METADATA
                        .inodetable
                        .get(&normalfile_filedesc_obj.inode)
                        .unwrap();
                    match &*inodeobj {
                        Inode::File(_) => {
                            // This code segment obtains the file object associated with the
                            // specified inode from FILEOBJECTTABLE.
                            // It calls 'sync_file_range' on this file object, where initially the
                            // flags are validated, returning -EINVAL for incorrect flags.
                            // If the flags are correct, libc::sync_file_range is invoked; if it
                            // fails (returns -1), 'from_discriminant' function handles the error
                            // code.

                            let fobj = FILEOBJECTTABLE.get(&normalfile_filedesc_obj.inode).unwrap();
                            let result = fobj.sync_file_range(offset, nbytes, flags);
                            if result == 0 || result == -(EINVAL as i32) {
                                return result;
                            }
                            match Errno::from_discriminant(interface::get_errno()) {
                                Ok(i) => {
                                    return syscall_error(
                                        i,
                                        "sync_file_range",
                                        "The libc call to sync_file_range failed!",
                                    );
                                }
                                Err(()) => panic!("Unknown errno value from setsockopt returned!"),
                            };
                        }
                        _ => syscall_error(
                            Errno::ESPIPE,
                            "sync_file_range",
                            "does not support special files for synchronization",
                        ),
                    }
                }
                _ => syscall_error(
                    Errno::EBADF,
                    "sync_file_range",
                    "fd is attached to an object which is unsuitable for synchronization",
                ),
            }
        } else {
            syscall_error(Errno::EBADF, "sync_file_range", "invalid file descriptor")
        }
    }

    //------------------FTRUNCATE SYSCALL------------------

    pub fn ftruncate_syscall(&self, fd: i32, length: isize) -> i32 {
        let checkedfd = self.get_filedescriptor(fd).unwrap();
        let unlocked_fd = checkedfd.read();
        if let Some(filedesc_enum) = &*unlocked_fd {
            match filedesc_enum {
                // only proceed when fd references a regular file
                File(normalfile_filedesc_obj) => {
                    if is_rdonly(normalfile_filedesc_obj.flags) {
                        return syscall_error(
                            Errno::EBADF,
                            "ftruncate",
                            "specified file not open for writing",
                        );
                    }
                    let inodenum = normalfile_filedesc_obj.inode;
                    self._truncate_helper(inodenum, length, true)
                }
                _ => syscall_error(
                    Errno::EINVAL,
                    "ftruncate",
                    "fd does not reference a regular file",
                ),
            }
        } else {
            syscall_error(
                Errno::EBADF,
                "ftruncate",
                "fd is not a valid file descriptor",
            )
        }
    }

    //------------------TRUNCATE SYSCALL------------------
    pub fn truncate_syscall(&self, path: &str, length: isize) -> i32 {
        let truepath = normpath(convpath(path), self);

        //Walk the file tree to get inode from path
        if let Some(inodenum) = metawalk(truepath.as_path()) {
            self._truncate_helper(inodenum, length, false)
        } else {
            syscall_error(
                Errno::ENOENT,
                "truncate",
                "path does not refer to an existing file",
            )
        }
    }

    /// ### Description
    ///
    /// The `pipe_syscall()` creates a pipe, a unidirectional data channel that
    /// can be used for interprocess communication.
    ///
    /// ### Arguments
    ///
    /// The `pipe_syscall()` accepts one argument:
    /// * `pipefd` - The array pipefd is used to return two file descriptors
    ///   referring to the ends of the pipe.
    ///
    /// ### Returns
    ///
    /// Upon successful completion, zero is returned.
    /// In case of a failure, an error is returned, and `errno` is set depending
    /// on the error, e.g. `ENFILE` etc.
    ///
    /// ### Errors
    ///
    /// Currently, only two errors are supposrted:
    /// * `ENFILE` - no available file descriptors
    ///
    /// ### Panics
    ///
    /// A panic can occur if there is no lock on the file descriptor index,
    /// which should not be possible, or if somehow the match statement
    /// finds an invalid flag
    ///
    /// To learn more about the syscall, flags, and error values, see
    /// [pipe(2)](https://man7.org/linux/man-pages/man2/pipe.2.html)
    pub fn pipe_syscall(&self, pipefd: &mut PipeArray) -> i32 {
        self.pipe2_syscall(pipefd, 0)
    }

    /// ### Description
    ///
    /// The `pipe2_syscall()` creates a pipe, a unidirectional data channel that
    /// can be used for interprocess communication. This syscall adds
    /// additional flags to the pipe syscall. We only implement CLOEXEC and
    /// NONBLOCK.
    ///
    /// ### Arguments
    ///
    /// The `pip2e_syscall()` accepts two arguments:
    /// * `pipefd` - The array pipefd is used to return two file descriptors
    ///   referring to the ends of the pipe.
    /// * `flags` - Flags that can be pre-set on the pipe file descriptors such
    ///   as CLOEXEC and NONBLOCK.
    ///
    /// ### Returns
    ///
    /// Upon successful completion, zero is returned.
    /// In case of a failure, an error is returned, and `errno` is set depending
    /// on the error, e.g. `ENFILE` etc.
    ///
    /// ### Errors
    ///
    /// Currently, the only error supported is:
    /// * `ENFILE` - no available file descriptors
    ///
    /// ### Panics
    ///
    /// A panic can occur if there is no lock on the file descriptor index,
    /// which should not be possible, or if somehow the match statement
    /// finds an invalid flag
    ///
    /// To learn more about the syscall, flags, and error values, see
    /// [pipe(2)](https://man7.org/linux/man-pages/man2/pipe.2.html)
    pub fn pipe2_syscall(&self, pipefd: &mut PipeArray, flags: i32) -> i32 {
        let flagsmask = O_CLOEXEC | O_NONBLOCK;
        let actualflags = flags & flagsmask;

        // lets make a standard pipe of 65,536 bytes
        let pipe =
            interface::RustRfc::new(interface::EmulatedPipe::new_with_capacity(PIPE_CAPACITY));

        // now lets get an fd for each end of the pipe and set flags to RD_ONLY and
        // WR_ONLY append each to pipefds list
        let accflags = [O_RDONLY, O_WRONLY];
        for accflag in accflags {
            let (fd, guardopt) = self.get_next_fd(None);
            if fd < 0 {
                return fd;
            }
            let fdoption = &mut *guardopt.unwrap();

            // insert this pipe descriptor into the fd slot
            let _insertval = fdoption.insert(Pipe(PipeDesc {
                pipe: pipe.clone(),
                // lets add the additional flags to read/write permission flag and add that to the
                // fd
                flags: accflag | actualflags,
                advlock: interface::RustRfc::new(interface::AdvisoryLock::new()),
            }));

            // now lets return the fd numbers in the pipefd array
            match accflag {
                O_RDONLY => {
                    pipefd.readfd = fd;
                }
                O_WRONLY => {
                    pipefd.writefd = fd;
                }
                _ => panic!("Corruption: Invalid flag"),
            }
        }

        0 // success
    }

    //------------------GETDENTS SYSCALL------------------
    /// ## `getdents_syscall`
    ///
    /// ### Description
    /// This function reads directory entries from a directory file descriptor
    /// and returns them in a buffer. Reading directory entries using multiple
    /// read calls can be less efficient because it involves reading the
    /// data in smaller chunks and then parsing it. getdents can often be
    /// faster by reading directory entries in a more optimized way.
    /// * The function first checks if the provided buffer size is sufficient to
    ///   store at least one `ClippedDirent` structure.
    /// * The function validates the provided file descriptor to ensure it
    ///   represents a valid file.
    /// * The function checks if the file descriptor refers to a directory.
    /// * The function iterates over the directory entries in the
    ///   `filename_to_inode_dict` of the directory inode.
    /// * For each entry, the function constructs a `ClippedDirent` structure,
    ///   which contains the inode number, offset, and record length.
    /// * It packs the constructed directory entries into the provided buffer
    ///   (`dirp`).
    /// * Updates the file position to the next directory entry to be read.
    ///
    /// ### Function Arguments
    /// * `fd`: A file descriptor representing the directory to read.
    /// * `dirp`: A pointer to a buffer where the directory entries will be
    ///   written.
    /// * `bufsize`: The size of the buffer in bytes.
    ///
    /// ### Returns
    /// * The number of bytes written to the buffer on success.
    ///
    /// ### Errors
    /// * `EINVAL(22)`: If the buffer size is too small or if the file
    ///   descriptor is invalid.
    /// * `ENOTDIR(20)`: If the file descriptor does not refer to a existing
    ///   directory.
    /// * `ESPIPE(29)`: If the file descriptor does not refer to a file.
    /// * `EBADF(9)` : If the file descriptor is invalid.
    /// ### Panics
    /// * There are no panics in this syscall.

    pub fn getdents_syscall(&self, fd: i32, dirp: *mut u8, bufsize: u32) -> i32 {
        let mut vec: Vec<(interface::ClippedDirent, Vec<u8>)> = Vec::new();

        // make sure bufsize is at least greater than size of a ClippedDirent struct
        // ClippedDirent is a simplified version of the traditional dirent structure
        // used in POSIX systems By using a simpler structure, SafePosix can
        // store and retrieve directory entries more efficiently, potentially
        // improving performance compared to using the full dirent structure.
        if bufsize <= interface::CLIPPED_DIRENT_SIZE {
            return syscall_error(Errno::EINVAL, "getdents", "Result buffer is too small.");
        }

        let checkedfd = match self.get_filedescriptor(fd) {
            Ok(fd) => fd,
            Err(_) => return syscall_error(Errno::EBADF, "getdents", "Invalid file descriptor."),
        };
        let mut unlocked_fd = checkedfd.write();
        if let Some(filedesc_enum) = &mut *unlocked_fd {
            match filedesc_enum {
                // only proceed when fd represents a file
                File(ref mut normalfile_filedesc_obj) => {
                    let inodeobj = FS_METADATA
                        .inodetable
                        .get(&normalfile_filedesc_obj.inode)
                        .unwrap();

                    match &*inodeobj {
                        // only proceed when inode is a dir
                        Inode::Dir(dir_inode_obj) => {
                            let position = normalfile_filedesc_obj.position;
                            let mut bufcount = 0;
                            let mut curr_size;
                            let mut count = 0;
                            let mut temp_len;

                            // iterate over filename-inode pairs in dict
                            for (filename, inode) in dir_inode_obj
                                .filename_to_inode_dict
                                .clone()
                                .into_iter()
                                .skip(position)
                            {
                                // convert filename to a filename vector of u8
                                let mut vec_filename: Vec<u8> = filename.as_bytes().to_vec();
                                vec_filename.push(b'\0'); // make filename null-terminated
                                                          // Push DT_UNKNOWN as d_type. This is a placeholder for now, as the
                                                          // actual file type is not yet determined.
                                vec_filename.push(DT_UNKNOWN); // push DT_UNKNOWN as d_type (for now)
                                temp_len =
                                    interface::CLIPPED_DIRENT_SIZE + vec_filename.len() as u32; // get length of current filename vector for padding calculation

                                // pad filename vector to the next highest 8 byte boundary
                                for _ in 0..(temp_len + 7) / 8 * 8 - temp_len {
                                    vec_filename.push(00);
                                }

                                // the fixed dirent size and length of filename vector add up to
                                // total size
                                curr_size =
                                    interface::CLIPPED_DIRENT_SIZE + vec_filename.len() as u32;

                                bufcount += curr_size; // increment bufcount

                                // stop iteration if current bufcount exceeds argument bufsize
                                if bufcount > bufsize {
                                    bufcount = bufcount - curr_size; // decrement bufcount since current element is not actually
                                                                     // written
                                    break;
                                }

                                // push properly constructed tuple to vector storing result
                                vec.push((
                                    interface::ClippedDirent {
                                        d_ino: inode as u64,
                                        d_off: bufcount as u64,
                                        d_reclen: curr_size as u16,
                                    },
                                    vec_filename,
                                ));
                                count += 1;
                            }
                            // update file position
                            // keeps track of the current position within the directory. It
                            // indicates which directory entry the
                            // function should read next.
                            normalfile_filedesc_obj.position = interface::rust_min(
                                position + count,
                                dir_inode_obj.filename_to_inode_dict.len(),
                            );

                            interface::pack_dirents(vec, dirp);
                            bufcount as i32 // return the number of bytes
                                            // written
                        }
                        _ => syscall_error(
                            Errno::ENOTDIR,
                            "getdents",
                            "File descriptor does not refer to a directory.",
                        ),
                    }
                }
                // raise error when fd represents a socket, pipe, or stream
                _ => syscall_error(
                    Errno::ESPIPE,
                    "getdents",
                    "Cannot getdents since fd does not refer to a file.",
                ),
            }
        } else {
            syscall_error(Errno::EBADF, "getdents", "Invalid file descriptor")
        }
    }

    /// ### Description
    ///
    /// The `getcwd_syscall()` function places an absolute pathname of the
    /// current working directory in the string pointed to by buf.
    ///
    /// ### Arguments
    ///
    /// The `getcwd_syscall()` accepts two arguments:
    /// * `buf` - a pointer to the string into which the current working
    /// directory is stored
    /// * `bufsize` - the length of the string `buf`
    ///
    /// ### Returns
    ///
    /// The standard requires returning the pointer to the string that
    /// stores the current working directory. In the current implementation,
    /// 0 is returned on success, while returning the pointer to the string is
    /// handled inside glibc.
    /// In case of a failure, an error is returned, and `errno` is set depending
    /// on the error, e.g. EINVAL, ERANGE, etc.
    ///
    /// ### Errors
    ///
    /// * `EINVAL` - the bufsize argument is zero and buf is not a NULL pointer.
    /// * `ERANGE` - the bufsize argument is less than the length of the
    /// absolute pathname of the working directory, including the
    /// terminating null byte.
    /// Other errors, like `EACCES`, `ENOMEM`, etc. are not supported.
    ///
    /// ### Panics
    ///
    /// There are no cases where this function panics.
    ///
    /// To learn more about the syscall and possible error values, see
    /// [getcwd(3)](https://man7.org/linux/man-pages/man3/getcwd.3.html)

    pub fn getcwd_syscall(&self, buf: *mut u8, bufsize: u32) -> i32 {
        //Here we only check if the size of the specified
        //string is 0. Null pointers are handled beforehand
        //by nacl and `types.rs`.
        if (bufsize as usize) == 0 {
            return syscall_error(Errno::EINVAL, "getcwd", "size of the specified buffer is 0");
        } else {
            //Cages store their current working directory as path buffers.
            //To use the obtained directory as a string, a null terminator needs
            //to be added to the path.
            let mut bytes: Vec<u8> = self.cwd.read().to_str().unwrap().as_bytes().to_vec();
            bytes.push(0u8); //Adding a null terminator to the end of the string
            let length = bytes.len();
            //The bufsize argument should be at least the length of the absolute
            //pathname of the working directory, including the terminating null byte.
            if (bufsize as usize) < length {
                return syscall_error(Errno::ERANGE, "getcwd", "the length (in bytes) of the absolute pathname of the current working directory exceeds the given size");
            }
            //It is expected that only the first `bufsize` bytes of the `buf` string
            //will be written into. The `fill()` function ensures this by taking
            //a mutable slice of length `bufsize` to the string pointed to by `buf`
            //and inserting the obtained current working directory into that slice,
            //thus prohibiting writing into the remaining bytes of the string.
            interface::fill(buf, length, &bytes);
            //returning 0 on success
            0
        }
    }

    //------------------SHMHELPERS----------------------

    pub fn rev_shm_find_index_by_addr(rev_shm: &Vec<(u32, i32)>, shmaddr: u32) -> Option<usize> {
        for (index, val) in rev_shm.iter().enumerate() {
            if val.0 == shmaddr as u32 {
                return Some(index);
            }
        }
        None
    }

    pub fn rev_shm_find_addrs_by_shmid(rev_shm: &Vec<(u32, i32)>, shmid: i32) -> Vec<u32> {
        let mut addrvec = Vec::new();
        for val in rev_shm.iter() {
            if val.1 == shmid as i32 {
                addrvec.push(val.0);
            }
        }

        return addrvec;
    }

    pub fn search_for_addr_in_region(
        rev_shm: &Vec<(u32, i32)>,
        search_addr: u32,
    ) -> Option<(u32, i32)> {
        let metadata = &SHM_METADATA;
        for val in rev_shm.iter() {
            let addr = val.0;
            let shmid = val.1;
            if let Some(segment) = metadata.shmtable.get_mut(&shmid) {
                let range = addr..(addr + segment.size as u32);
                if range.contains(&search_addr) {
                    return Some((addr, shmid));
                }
            }
        }
        None
    }

    //------------------SHMGET SYSCALL------------------

    pub fn shmget_syscall(&self, key: i32, size: usize, shmflg: i32) -> i32 {
        if key == IPC_PRIVATE {
            return syscall_error(Errno::ENOENT, "shmget", "IPC_PRIVATE not implemented");
        }
        let shmid: i32;
        let metadata = &SHM_METADATA;

        match metadata.shmkeyidtable.entry(key) {
            interface::RustHashEntry::Occupied(occupied) => {
                if (IPC_CREAT | IPC_EXCL) == (shmflg & (IPC_CREAT | IPC_EXCL)) {
                    return syscall_error(
                        Errno::EEXIST,
                        "shmget",
                        "key already exists and IPC_CREAT and IPC_EXCL were used",
                    );
                }
                shmid = *occupied.get();
            }
            interface::RustHashEntry::Vacant(vacant) => {
                if 0 == (shmflg & IPC_CREAT) {
                    return syscall_error(
                        Errno::ENOENT,
                        "shmget",
                        "tried to use a key that did not exist, and IPC_CREAT was not specified",
                    );
                }

                if (size as u32) < SHMMIN || (size as u32) > SHMMAX {
                    return syscall_error(
                        Errno::EINVAL,
                        "shmget",
                        "Size is less than SHMMIN or more than SHMMAX",
                    );
                }

                shmid = metadata.new_keyid();
                vacant.insert(shmid);
                let mode = (shmflg & 0x1FF) as u16; // mode is 9 least signficant bits of shmflag, even if we dont really do
                                                    // anything with them

                let segment = new_shm_segment(
                    key,
                    size,
                    self.cageid as u32,
                    DEFAULT_UID,
                    DEFAULT_GID,
                    mode,
                );
                metadata.shmtable.insert(shmid, segment);
            }
        };
        shmid // return the shmid
    }

    //------------------SHMAT SYSCALL------------------

    pub fn shmat_syscall(&self, shmid: i32, shmaddr: *mut u8, shmflg: i32) -> i32 {
        let metadata = &SHM_METADATA;
        let prot: i32;
        if let Some(mut segment) = metadata.shmtable.get_mut(&shmid) {
            if 0 != (shmflg & SHM_RDONLY) {
                prot = PROT_READ;
            } else {
                prot = PROT_READ | PROT_WRITE;
            }
            let mut rev_shm = self.rev_shm.lock();
            rev_shm.push((shmaddr as u32, shmid));
            drop(rev_shm);

            // update semaphores
            if !segment.semaphor_offsets.is_empty() {
                // lets just look at the first cage in the set, since we only need to grab the
                // ref from one
                if let Some(cageid) = segment
                    .attached_cages
                    .clone()
                    .into_read_only()
                    .keys()
                    .next()
                {
                    let cage2 = interface::cagetable_getref(*cageid);
                    let cage2_rev_shm = cage2.rev_shm.lock();
                    let addrs = Self::rev_shm_find_addrs_by_shmid(&cage2_rev_shm, shmid); // find all the addresses assoc. with shmid
                    for offset in segment.semaphor_offsets.iter() {
                        let sementry = cage2.sem_table.get(&(addrs[0] + *offset)).unwrap().clone(); //add  semaphors into semtable at addr + offsets
                        self.sem_table.insert(shmaddr as u32 + *offset, sementry);
                    }
                }
            }

            segment.map_shm(shmaddr, prot, self.cageid)
        } else {
            syscall_error(Errno::EINVAL, "shmat", "Invalid shmid value")
        }
    }

    //------------------SHMDT SYSCALL------------------

    pub fn shmdt_syscall(&self, shmaddr: *mut u8) -> i32 {
        let metadata = &SHM_METADATA;
        let mut rm = false;
        let mut rev_shm = self.rev_shm.lock();
        let rev_shm_index = Self::rev_shm_find_index_by_addr(&rev_shm, shmaddr as u32);

        if let Some(index) = rev_shm_index {
            let shmid = rev_shm[index].1;
            match metadata.shmtable.entry(shmid) {
                interface::RustHashEntry::Occupied(mut occupied) => {
                    let segment = occupied.get_mut();

                    // update semaphores
                    for offset in segment.semaphor_offsets.iter() {
                        self.sem_table.remove(&(shmaddr as u32 + *offset));
                    }

                    segment.unmap_shm(shmaddr, self.cageid);

                    if segment.rmid && segment.shminfo.shm_nattch == 0 {
                        rm = true;
                    }
                    rev_shm.swap_remove(index);

                    if rm {
                        let key = segment.key;
                        occupied.remove_entry();
                        metadata.shmkeyidtable.remove(&key);
                    }

                    return shmid; //NaCl relies on this non-posix behavior of
                                  // returning the shmid on success
                }
                interface::RustHashEntry::Vacant(_) => {
                    panic!("Inode not created for some reason");
                }
            };
        } else {
            return syscall_error(
                Errno::EINVAL,
                "shmdt",
                "No shared memory segment at shmaddr",
            );
        }
    }

    //------------------SHMCTL SYSCALL------------------

    pub fn shmctl_syscall(&self, shmid: i32, cmd: i32, buf: Option<&mut ShmidsStruct>) -> i32 {
        let metadata = &SHM_METADATA;

        if let Some(mut segment) = metadata.shmtable.get_mut(&shmid) {
            match cmd {
                IPC_STAT => {
                    *buf.unwrap() = segment.shminfo;
                }
                IPC_RMID => {
                    segment.rmid = true;
                    segment.shminfo.shm_perm.mode |= SHM_DEST as u16;
                    if segment.shminfo.shm_nattch == 0 {
                        let key = segment.key;
                        drop(segment);
                        metadata.shmtable.remove(&shmid);
                        metadata.shmkeyidtable.remove(&key);
                    }
                }
                _ => {
                    return syscall_error(
                        Errno::EINVAL,
                        "shmctl",
                        "Arguments provided do not match implemented parameters",
                    );
                }
            }
        } else {
            return syscall_error(Errno::EINVAL, "shmctl", "Invalid identifier");
        }

        0 //shmctl has succeeded!
    }

    //------------------MUTEX SYSCALLS------------------

    pub fn mutex_create_syscall(&self) -> i32 {
        let mut mutextable = self.mutex_table.write();
        let mut index_option = None;
        for i in 0..mutextable.len() {
            if mutextable[i].is_none() {
                index_option = Some(i);
                break;
            }
        }

        let index = if let Some(ind) = index_option {
            ind
        } else {
            mutextable.push(None);
            mutextable.len() - 1
        };

        let mutex_result = interface::RawMutex::create();
        match mutex_result {
            Ok(mutex) => {
                mutextable[index] = Some(interface::RustRfc::new(mutex));
                index as i32
            }
            Err(_) => match Errno::from_discriminant(interface::get_errno()) {
                Ok(i) => syscall_error(
                    i,
                    "mutex_create",
                    "The libc call to pthread_mutex_init failed!",
                ),
                Err(()) => panic!("Unknown errno value from pthread_mutex_init returned!"),
            },
        }
    }

    pub fn mutex_destroy_syscall(&self, mutex_handle: i32) -> i32 {
        let mut mutextable = self.mutex_table.write();
        if mutex_handle < mutextable.len() as i32
            && mutex_handle >= 0
            && mutextable[mutex_handle as usize].is_some()
        {
            mutextable[mutex_handle as usize] = None;
            0
        } else {
            //undefined behavior
            syscall_error(
                Errno::EBADF,
                "mutex_destroy",
                "Mutex handle does not refer to a valid mutex!",
            )
        }
        //the RawMutex is destroyed on Drop

        //this is currently assumed to always succeed, as the man page does not
        // list possible errors for pthread_mutex_destroy
    }

    pub fn mutex_lock_syscall(&self, mutex_handle: i32) -> i32 {
        let mutextable = self.mutex_table.read();
        if mutex_handle < mutextable.len() as i32
            && mutex_handle >= 0
            && mutextable[mutex_handle as usize].is_some()
        {
            let clonedmutex = mutextable[mutex_handle as usize].as_ref().unwrap().clone();
            drop(mutextable);
            let retval = clonedmutex.lock();

            if retval < 0 {
                match Errno::from_discriminant(interface::get_errno()) {
                    Ok(i) => {
                        return syscall_error(
                            i,
                            "mutex_lock",
                            "The libc call to pthread_mutex_lock failed!",
                        );
                    }
                    Err(()) => panic!("Unknown errno value from pthread_mutex_lock returned!"),
                };
            }

            retval
        } else {
            //undefined behavior
            syscall_error(
                Errno::EBADF,
                "mutex_lock",
                "Mutex handle does not refer to a valid mutex!",
            )
        }
    }

    pub fn mutex_trylock_syscall(&self, mutex_handle: i32) -> i32 {
        let mutextable = self.mutex_table.read();
        if mutex_handle < mutextable.len() as i32
            && mutex_handle >= 0
            && mutextable[mutex_handle as usize].is_some()
        {
            let clonedmutex = mutextable[mutex_handle as usize].as_ref().unwrap().clone();
            drop(mutextable);
            let retval = clonedmutex.trylock();

            if retval < 0 {
                match Errno::from_discriminant(interface::get_errno()) {
                    Ok(i) => {
                        return syscall_error(
                            i,
                            "mutex_trylock",
                            "The libc call to pthread_mutex_trylock failed!",
                        );
                    }
                    Err(()) => panic!("Unknown errno value from pthread_mutex_trylock returned!"),
                };
            }

            retval
        } else {
            //undefined behavior
            syscall_error(
                Errno::EBADF,
                "mutex_trylock",
                "Mutex handle does not refer to a valid mutex!",
            )
        }
    }

    pub fn mutex_unlock_syscall(&self, mutex_handle: i32) -> i32 {
        let mutextable = self.mutex_table.read();
        if mutex_handle < mutextable.len() as i32
            && mutex_handle >= 0
            && mutextable[mutex_handle as usize].is_some()
        {
            let clonedmutex = mutextable[mutex_handle as usize].as_ref().unwrap().clone();
            drop(mutextable);
            let retval = clonedmutex.unlock();

            if retval < 0 {
                match Errno::from_discriminant(interface::get_errno()) {
                    Ok(i) => {
                        return syscall_error(
                            i,
                            "mutex_unlock",
                            "The libc call to pthread_mutex_unlock failed!",
                        );
                    }
                    Err(()) => panic!("Unknown errno value from pthread_mutex_unlock returned!"),
                };
            }

            retval
        } else {
            //undefined behavior
            syscall_error(
                Errno::EBADF,
                "mutex_unlock",
                "Mutex handle does not refer to a valid mutex!",
            )
        }
    }

    //------------------CONDVAR SYSCALLS------------------

    pub fn cond_create_syscall(&self) -> i32 {
        let mut cvtable = self.cv_table.write();
        let mut index_option = None;
        for i in 0..cvtable.len() {
            if cvtable[i].is_none() {
                index_option = Some(i);
                break;
            }
        }

        let index = if let Some(ind) = index_option {
            ind
        } else {
            cvtable.push(None);
            cvtable.len() - 1
        };

        let cv_result = interface::RawCondvar::create();
        match cv_result {
            Ok(cv) => {
                cvtable[index] = Some(interface::RustRfc::new(cv));
                index as i32
            }
            Err(_) => match Errno::from_discriminant(interface::get_errno()) {
                Ok(i) => syscall_error(
                    i,
                    "cond_create",
                    "The libc call to pthread_cond_init failed!",
                ),
                Err(()) => panic!("Unknown errno value from pthread_cond_init returned!"),
            },
        }
    }

    pub fn cond_destroy_syscall(&self, cv_handle: i32) -> i32 {
        let mut cvtable = self.cv_table.write();
        if cv_handle < cvtable.len() as i32
            && cv_handle >= 0
            && cvtable[cv_handle as usize].is_some()
        {
            cvtable[cv_handle as usize] = None;
            0
        } else {
            //undefined behavior
            syscall_error(
                Errno::EBADF,
                "cond_destroy",
                "Condvar handle does not refer to a valid condvar!",
            )
        }
        //the RawCondvar is destroyed on Drop

        //this is currently assumed to always succeed, as the man page does not
        // list possible errors for pthread_cv_destroy
    }

    pub fn cond_signal_syscall(&self, cv_handle: i32) -> i32 {
        let cvtable = self.cv_table.read();
        if cv_handle < cvtable.len() as i32
            && cv_handle >= 0
            && cvtable[cv_handle as usize].is_some()
        {
            let clonedcv = cvtable[cv_handle as usize].as_ref().unwrap().clone();
            drop(cvtable);
            let retval = clonedcv.signal();

            if retval < 0 {
                match Errno::from_discriminant(interface::get_errno()) {
                    Ok(i) => {
                        return syscall_error(
                            i,
                            "cond_signal",
                            "The libc call to pthread_cond_signal failed!",
                        );
                    }
                    Err(()) => panic!("Unknown errno value from pthread_cond_signal returned!"),
                };
            }

            retval
        } else {
            //undefined behavior
            syscall_error(
                Errno::EBADF,
                "cond_signal",
                "Condvar handle does not refer to a valid condvar!",
            )
        }
    }

    pub fn cond_broadcast_syscall(&self, cv_handle: i32) -> i32 {
        let cvtable = self.cv_table.read();
        if cv_handle < cvtable.len() as i32
            && cv_handle >= 0
            && cvtable[cv_handle as usize].is_some()
        {
            let clonedcv = cvtable[cv_handle as usize].as_ref().unwrap().clone();
            drop(cvtable);
            let retval = clonedcv.broadcast();

            if retval < 0 {
                match Errno::from_discriminant(interface::get_errno()) {
                    Ok(i) => {
                        return syscall_error(
                            i,
                            "cond_broadcast",
                            "The libc call to pthread_cond_broadcast failed!",
                        );
                    }
                    Err(()) => panic!("Unknown errno value from pthread_cond_broadcast returned!"),
                };
            }

            retval
        } else {
            //undefined behavior
            syscall_error(
                Errno::EBADF,
                "cond_broadcast",
                "Condvar handle does not refer to a valid condvar!",
            )
        }
    }

    pub fn cond_wait_syscall(&self, cv_handle: i32, mutex_handle: i32) -> i32 {
        let cvtable = self.cv_table.read();
        if cv_handle < cvtable.len() as i32
            && cv_handle >= 0
            && cvtable[cv_handle as usize].is_some()
        {
            let clonedcv = cvtable[cv_handle as usize].as_ref().unwrap().clone();
            drop(cvtable);

            let mutextable = self.mutex_table.read();
            if mutex_handle < mutextable.len() as i32
                && mutex_handle >= 0
                && mutextable[mutex_handle as usize].is_some()
            {
                let clonedmutex = mutextable[mutex_handle as usize].as_ref().unwrap().clone();
                drop(mutextable);
                let retval = clonedcv.wait(&*clonedmutex);

                // if the cancel status is set in the cage, we trap around a cancel point
                // until the individual thread is signaled to cancel itself
                if self
                    .cancelstatus
                    .load(interface::RustAtomicOrdering::Relaxed)
                {
                    loop {
                        interface::cancelpoint(self.cageid);
                    } // we check cancellation status here without letting the
                      // function return
                }

                if retval < 0 {
                    match Errno::from_discriminant(interface::get_errno()) {
                        Ok(i) => {
                            return syscall_error(
                                i,
                                "cond_wait",
                                "The libc call to pthread_cond_wait failed!",
                            );
                        }
                        Err(()) => panic!("Unknown errno value from pthread_cond_wait returned!"),
                    };
                }

                retval
            } else {
                //undefined behavior
                syscall_error(
                    Errno::EBADF,
                    "cond_wait",
                    "Mutex handle does not refer to a valid mutex!",
                )
            }
        } else {
            //undefined behavior
            syscall_error(
                Errno::EBADF,
                "cond_wait",
                "Condvar handle does not refer to a valid condvar!",
            )
        }
    }

    pub fn cond_timedwait_syscall(
        &self,
        cv_handle: i32,
        mutex_handle: i32,
        time: interface::RustDuration,
    ) -> i32 {
        let cvtable = self.cv_table.read();
        if cv_handle < cvtable.len() as i32
            && cv_handle >= 0
            && cvtable[cv_handle as usize].is_some()
        {
            let clonedcv = cvtable[cv_handle as usize].as_ref().unwrap().clone();
            drop(cvtable);

            let mutextable = self.mutex_table.read();
            if mutex_handle < mutextable.len() as i32
                && mutex_handle >= 0
                && mutextable[mutex_handle as usize].is_some()
            {
                let clonedmutex = mutextable[mutex_handle as usize].as_ref().unwrap().clone();
                drop(mutextable);
                let retval = clonedcv.timedwait(&*clonedmutex, time);
                if retval < 0 {
                    match Errno::from_discriminant(interface::get_errno()) {
                        Ok(i) => {
                            return syscall_error(
                                i,
                                "cond_wait",
                                "The libc call to pthread_cond_wait failed!",
                            );
                        }
                        Err(()) => panic!("Unknown errno value from pthread_cond_wait returned!"),
                    };
                }

                retval
            } else {
                //undefined behavior
                syscall_error(
                    Errno::EBADF,
                    "cond_wait",
                    "Mutex handle does not refer to a valid mutex!",
                )
            }
        } else {
            //undefined behavior
            syscall_error(
                Errno::EBADF,
                "cond_wait",
                "Condvar handle does not refer to a valid condvar!",
            )
        }
    }

    //##------------------SEMAPHORE SYSCALLS------------------
    /*
     *  Initialize semaphore object SEM to value
     *  pshared used to indicate whether the semaphore is shared in threads (when
     * equals to 0)  or shared between processes (when nonzero)
     */
    /// ## `sem_init_syscall`
    ///
    /// ### Description
    /// This function initializes a semaphore object, setting its initial value
    /// and specifying whether it's shared between threads or processes.
    /// 1. Boundary Check: The function first checks if the initial value is
    /// within the allowed range.
    /// 2. Check for Existing Semaphore: The function then checks if a semaphore
    /// with the given handle already exists.
    /// 3. Initialize New Semaphore: If the semaphore does not exist, the
    ///    function
    /// creates a new semaphore object and inserts it into the semaphore table.
    /// 4. Add to Shared Memory Attachments (if shared): If the semaphore is
    ///    shared
    /// between processes,
    /// the function adds it to the shared memory attachments of other processes
    /// that have already attached to the shared memory segment.
    /// 5. The function ensures thread safety by using a unique semaphore handle
    ///    and
    /// checking for existing entries in the semaphore table before attempting
    /// to create a new one. The code also avoids inserting a semaphore into
    /// the same cage twice during the shared memory attachment process by
    /// excluding the initial cage from the iteration loop. [sem_init](https://man7.org/linux/man-pages/man3/sem_init.3.html)
    /// ### Function Arguments
    /// * `sem_handle`: A unique identifier for the semaphore.
    /// * `pshared`:  Indicates whether the semaphore is shared between
    /// threads (0) or processes (non-zero).
    /// * `value`: The initial value of the semaphore.
    ///
    /// ### Returns
    /// * 0 on success.
    /// ### Errors
    /// * `EBADF (9)`: If the semaphore handle is invalid or the semaphore is
    ///   already initialized.
    /// * `EINVAL (22)`: If the initial value exceeds the maximum allowable
    ///   value
    /// * 'ENOSYS(38)' : Currently not supported
    /// for a semaphore (SEM_VALUE_MAX).

    pub fn sem_init_syscall(&self, sem_handle: u32, pshared: i32, value: u32) -> i32 {
        // Boundary check
        if value > SEM_VALUE_MAX {
            return syscall_error(Errno::EINVAL, "sem_init", "value exceeds SEM_VALUE_MAX");
        }

        let metadata = &SHM_METADATA;
        let is_shared = pshared != 0;

        // Check if a semaphore with the given handle already exists in the semaphore
        // table. If it exists, the semaphore is already initialized, so an
        // error is returned. This ensures that only new semaphores are
        // initialized.
        let semtable = &self.sem_table;

        if !semtable.contains_key(&sem_handle) {
            // Create a new semaphore object.
            let new_semaphore =
                interface::RustRfc::new(interface::RustSemaphore::new(value, is_shared));
            // Insert the new semaphore into the semaphore table.
            semtable.insert(sem_handle, new_semaphore.clone());

            // If the semaphore is shared, add it to the shared memory attachments of other
            // processes.
            if is_shared {
                let rev_shm = self.rev_shm.lock();
                // if its shared and exists in an existing mapping we need to add it to other
                // cages
                if let Some((mapaddr, shmid)) =
                    Self::search_for_addr_in_region(&rev_shm, sem_handle)
                {
                    let offset = mapaddr - sem_handle;
                    // iterate through all cages with shared memory segment attached and add
                    // semaphor in segments at attached addr + offset
                    // offset represents the relative position of the semaphore within the shared
                    // memory region.

                    if let Some(segment) = metadata.shmtable.get_mut(&shmid) {
                        for cageid in segment.attached_cages.clone().into_read_only().keys() {
                            let cage = interface::cagetable_getref(*cageid);
                            // Find all addresses in the shared memory region that belong to the
                            // current segment.
                            let addrs = Self::rev_shm_find_addrs_by_shmid(&rev_shm, shmid);
                            // Iterate through all addresses and add the semaphore to the cage's
                            // semaphore table.
                            for addr in addrs.iter() {
                                cage.sem_table.insert(addr + offset, new_semaphore.clone());
                            }
                        }
                        // Add the offset to the semaphore offsets list.
                        segment.semaphor_offsets.insert(offset);
                    }
                }
            }
            return 0;
        }
        // Return an error indicating that the semaphore is already initialized.
        return syscall_error(Errno::EBADF, "sem_init", "semaphore already initialized");
    }

    /// ## `sem_wait_syscall`
    ///
    /// ### Description
    /// 1. Check for Semaphore Existence:The function first checks if the
    ///    provided
    /// semaphore handle exists in the semaphore table.
    /// 2. Acquire Semaphore: If the semaphore exists, the function attempts to
    ///    acquire it using `lock`.
    /// This operation will block the calling process until the semaphore
    /// becomes available.
    /// 3. Error Handling:If the semaphore handle is invalid, the function
    ///    returns an error
    /// 4. This function allows a process to wait for a semaphore to become
    ///    available.
    /// If the semaphore is currently available (its value is greater than 0),
    /// the function will acquire the semaphore and return 0.
    /// 5. If the semaphore is unavailable (its value is 0), the function will
    ///    block the
    /// calling process until the semaphore becomes available(its value becomes
    /// 1). [sem_wait(2)](https://man7.org/linux/man-pages/man3/sem_wait.3.html)
    /// ### Function Arguments
    /// * `sem_handle`: A unique identifier for the semaphore.
    ///
    /// ### Returns
    /// * 0 on success.
    /// ### Errors
    /// * `EINVAL(22)`: If the semaphore handle is invalid.
    /// * 'EAGAIN(11)' & 'EINTR(4)' currently are not supported
    pub fn sem_wait_syscall(&self, sem_handle: u32) -> i32 {
        let semtable = &self.sem_table;
        // Check whether the semaphore exists in the semaphore table. If found, obtain a
        // mutable borrow to the semaphore entry.
        if let Some(sementry) = semtable.get_mut(&sem_handle) {
            // Clone the semaphore entry to create an independent copy that we can modify
            // without affecting other threads.
            let semaphore = sementry.clone();
            // Release the mutable borrow on the original semaphore entry to allow other
            // threads to access the semaphore table concurrently. Cloning and
            // dropping the original reference lets us modify the value without deadlocking
            // the dashmap.
            drop(sementry);
            // Acquire the semaphore. This operation will block the calling process until
            // the semaphore becomes available. The`lock` method internally
            // decrements the semaphore value.
            // The lock fun is located in misc.rs
            semaphore.lock();
        } else {
            return syscall_error(Errno::EINVAL, "sem_wait", "sem is not a valid semaphore");
        }
        // If the semaphore was successfully acquired, return 0.
        return 0;
    }

    /// ## `sem_post_syscall`
    ///
    /// ### Description
    /// This function increments the value of a semaphore.
    ///  1. Check for Semaphore Existence:The function first checks if the
    ///     provided
    /// semaphore handle exists in the semaphore table.
    ///  2. Increment Semaphore Value: If the semaphore exists, the function
    /// increments its value using `unlock`.
    ///  3. Error Handling: If the semaphore handle is invalid or incrementing
    ///     the semaphore
    ///  would exceed the maximum value, the function returns an appropriate
    /// error code. [sem_post](https://man7.org/linux/man-pages/man3/sem_post.3.html)
    ///
    /// ### Function Arguments
    /// * `sem_handle`: A unique identifier for the semaphore.
    ///
    /// ### Returns
    /// * 0 on success.
    ///
    /// ### Errors
    /// * `EINVAL(22)`: If the semaphore handle is invalid.
    /// * `EOVERFLOW(75)`: If incrementing the semaphore would exceed the
    ///   maximum allowable value.
    pub fn sem_post_syscall(&self, sem_handle: u32) -> i32 {
        let semtable = &self.sem_table;
        // Check whether semaphore exists
        if let Some(sementry) = semtable.get_mut(&sem_handle) {
            // Clone the semaphore entry to create an independent copy that we can modify
            // without affecting other threads
            let semaphore = sementry.clone();
            // Release the mutable borrow on the original semaphore entry to allow other
            // threads to access the semaphore table concurrently. Cloning and
            // dropping the original reference lets us modify the value without deadlocking
            // the dashmap.
            drop(sementry);
            // Increment the semaphore value.
            //If the semaphore's value becomes greater than zero, one or more blocked
            // threads will be woken up and proceed to acquire the semaphore,
            // decreasing its value. The unlock fun is located in misc.rs
            if !semaphore.unlock() {
                // Return an error indicating that the maximum allowable value for a semaphore
                // would be exceeded.
                return syscall_error(
                    Errno::EOVERFLOW,
                    "sem_post",
                    "The maximum allowable value for a semaphore would be exceeded",
                );
            }
        } else {
            return syscall_error(Errno::EINVAL, "sem_wait", "sem is not a valid semaphore");
        }
        return 0;
    }

    /// ## `sem_destroy_syscall`
    ///
    /// ### Description
    /// This function destroys a semaphore, freeing its associated resources.
    ///   1. Check for Semaphore Existence: The function first checks if the
    ///      provided
    /// semaphore handle exists in the semaphore table.
    ///   2. Remove from Semaphore Table: If the semaphore exists, the function
    ///      removes
    /// it from the semaphore table.
    ///   3. Remove from Shared Memory Attachments (if shared): If the semaphore
    ///      is shared, the
    /// function also removes it from the shared memory attachments of other
    /// processes.
    ///   4. Error Handling: If the semaphore handle is invalid, the function
    ///      returns an error.
    ///[sem_destroy](https://man7.org/linux/man-pages/man3/sem_destroy.3.html)
    /// ### Function Arguments
    /// * `sem_handle`: A unique identifier for the semaphore.
    ///
    /// ### Returns
    /// * 0 on success.
    ///
    /// ### Errors
    /// * `EINVAL(22)`: If the semaphore handle is invalid.
    pub fn sem_destroy_syscall(&self, sem_handle: u32) -> i32 {
        let metadata = &SHM_METADATA;

        let semtable = &self.sem_table;
        // remove entry from semaphore table
        if let Some(sementry) = semtable.remove(&sem_handle) {
            // If the semaphore is shared, remove it from other process attachments.
            if sementry
                .1
                .is_shared
                .load(interface::RustAtomicOrdering::Relaxed)
            {
                // if its shared we'll need to remove it from other attachments
                let rev_shm = self.rev_shm.lock();
                // Search for the semaphore address in the shared memory region.
                if let Some((mapaddr, shmid)) =
                    Self::search_for_addr_in_region(&rev_shm, sem_handle)
                {
                    // find all segments that contain semaphore
                    let offset = mapaddr - sem_handle;
                    // Iterate through all segments containing the semaphore.
                    if let Some(segment) = metadata.shmtable.get_mut(&shmid) {
                        // Iterate through all cages containing the segment.
                        for cageid in segment.attached_cages.clone().into_read_only().keys() {
                            // Get a reference to the cagetable for the current cage.
                            let cage = interface::cagetable_getref(*cageid);
                            // Find all addresses in the shared memory region that belong to the
                            // current segment.
                            let addrs = Self::rev_shm_find_addrs_by_shmid(&rev_shm, shmid);
                            // Iterate through all addresses and remove the semaphore from the
                            // cage's semaphore table.
                            for addr in addrs.iter() {
                                cage.sem_table.remove(&(addr + offset)); //remove semapoores at attached addresses + the offset
                                                                         //offset represents the relative position of the semaphore within the shared memory region.
                            }
                        }
                    }
                }
            }
            // Return 0 to indicate successful semaphore destruction.
            return 0;
        } else {
            return syscall_error(Errno::EINVAL, "sem_destroy", "sem is not a valid semaphore");
        }
    }

    /*
     * Take only sem_t *sem as argument, and return int *sval
     */

    /// ## `sem_getvalue_syscall`
    ///
    /// ### Description
    /// This function implements the `sem_getvalue` system call, which retrieves
    /// the current value of a semaphore.
    ///   1. Check for Semaphore Existence: The function first checks if the
    ///      provided
    /// semaphore handle exists in the semaphore table.
    ///   2. Retrieve Semaphore Value: If the semaphore exists, the function
    ///      retrieves
    ///  its current value and returns it.
    ///   3. Error Handling: If the semaphore handle is invalid, the function
    ///      returns an error.
    ///[sem_getvalue(2)](https://man7.org/linux/man-pages/man3/sem_getvalue.3.html#:~:text=sem_getvalue()%20places%20the%20current,sem_wait(3)%2C%20POSIX.)
    ///
    /// ### Function Arguments
    /// * `sem_handle`: A unique identifier for the semaphore.
    ///
    /// ### Returns
    /// * The current value of the semaphore on success.
    ///
    /// ### Errors
    /// * `EINVAL(22)`: If the semaphore handle is invalid.
    pub fn sem_getvalue_syscall(&self, sem_handle: u32) -> i32 {
        let semtable = &self.sem_table;
        // Check whether the semaphore exists in the semaphore table.
        if let Some(sementry) = semtable.get_mut(&sem_handle) {
            // Clone the semaphore entry to avoid modifying the original entry in the table.
            let semaphore = sementry.clone();
            // Release the mutable borrow on the original semaphore entry to allow other
            // threads to access the semaphore table concurrently. Cloning and
            // dropping the original reference lets us modify the value without deadlocking
            // the dashmap.
            drop(sementry);
            return semaphore.get_value();
        }
        return syscall_error(
            Errno::EINVAL,
            "sem_getvalue",
            "sem is not a valid semaphore",
        );
    }

    /// ## `sem_trywait_syscall`
    ///
    /// ### Description
    /// This function implements the `sem_trywait` system call, which attempts
    /// to acquire a semaphore without blocking.
    ///   1. Check for Semaphore Existence: The function first checks if the
    /// provided semaphore handle is valid.
    ///   2. Attempt to Acquire: If the semaphore exists, the function attempts
    ///  to acquire it using `trylock`.
    ///   3. Error Handling: If the semaphore is unavailable or the handle is
    ///      invalid,
    ///  the function returns an appropriate error code.
    /// [sem_trywait(2)](https://man7.org/linux/man-pages/man3/sem_trywait.3p.html)
    /// ### Function Arguments
    /// * `sem_handle`: A unique identifier for the semaphore.
    ///
    /// ### Returns
    /// * 0 on success (semaphore acquired).
    ///
    /// ### Errors
    /// * `EINVAL(22)`: If the semaphore handle is invalid.
    /// * `EAGAIN(11)`: If the semaphore is unavailable (its value is 0).

    pub fn sem_trywait_syscall(&self, sem_handle: u32) -> i32 {
        let semtable = &self.sem_table;
        // Check whether semaphore exists
        if let Some(sementry) = semtable.get_mut(&sem_handle) {
            // Clone the semaphore entry to avoid modifying the original entry in the table.
            let semaphore = sementry.clone();
            // Release the mutable borrow on the original semaphore entry to allow other
            // threads to access the semaphore table concurrently. Cloning and
            // dropping the original reference lets us modify the value without deadlocking
            // the dashmap.
            drop(sementry);
            // Attempt to acquire the semaphore without blocking.
            // If the semaphore is currently unavailable (value is 0), this operation will
            // fail.
            if !semaphore.trylock() {
                // Return an error indicating that the operation could not be performed without
                // blocking.
                return syscall_error(
                    Errno::EAGAIN,
                    "sem_trywait",
                    "The operation could not be performed without blocking",
                );
            }
        } else {
            return syscall_error(Errno::EINVAL, "sem_trywait", "sem is not a valid semaphore");
        }
        // If the semaphore was successfully acquired, return 0.
        return 0;
    }

    /// ## `sem_timedwait_syscall`
    ///
    /// ### Description
    /// This function implements the `sem_timedwait` system call, which attempts
    /// to acquire a semaphore with a timeout.
    ///   1. Convert Timeout to Timespec: The function first converts the
    ///      provided
    /// timeout duration into a `timespec` structure, which is used by the
    /// underlying `timedlock` function.
    ///   2. Check for Semaphore Existence: The function then checks if the
    ///      provided
    /// semaphore handle exists in the semaphore table.
    ///   3. Attempt to Acquire with Timeout: If the semaphore exists, the
    ///      function attempts
    /// to acquire it using `timedlock`, which will block for the specified
    /// duration.
    ///   4. Error Handling: If the semaphore is unavailable, the timeout
    ///      expires,
    /// or the handle is invalid, the function returns an appropriate error
    /// code. [sem_timedwait(2)](https://man7.org/linux/man-pages/man3/sem_timedwait.3p.html)
    /// ### Function Arguments
    /// * `sem_handle`: A unique identifier for the semaphore.
    /// * `time`: The maximum time to wait for the semaphore to become
    ///   available,
    ///  expressed as a `RustDuration`.
    ///
    /// ### Returns
    /// * 0 on success (semaphore acquired).
    ///
    /// ### Errors
    /// * `ETIMEDOUT(110)`: If the timeout expires before the semaphore becomes
    ///   available.
    /// * `EINVAL(22)`: If the semaphore handle is invalid or the timeout value
    ///   is invalid.
    pub fn sem_timedwait_syscall(&self, sem_handle: u32, time: interface::RustDuration) -> i32 {
        let abstime = libc::timespec {
            tv_sec: time.as_secs() as i64,
            tv_nsec: (time.as_nanos() % 1000000000) as i64,
        };
        if abstime.tv_nsec < 0 {
            return syscall_error(Errno::EINVAL, "sem_timedwait", "Invalid timedout");
        }
        let semtable = &self.sem_table;
        // Check whether semaphore exists
        if let Some(sementry) = semtable.get_mut(&sem_handle) {
            // Clone the semaphore entry to create an independent copy that we can modify
            // without affecting other threads
            let semaphore = sementry.clone();
            // Release the mutable borrow on the original semaphore entry to allow other
            // threads to access the semaphore table concurrently. Cloning and
            // dropping the original reference lets us modify the value without deadlocking
            // the dashmap.
            drop(sementry);
            // Attempt to acquire the semaphore with a timeout.
            if !semaphore.timedlock(time) {
                // Return an error indicating that the call timed out before the semaphore could
                // be locked.
                return syscall_error(
                    Errno::ETIMEDOUT,
                    "sem_timedwait",
                    "The call timed out before the semaphore could be locked",
                );
            }
        } else {
            return syscall_error(
                Errno::EINVAL,
                "sem_timedwait",
                "sem is not a valid semaphore",
            );
        }
        // If the semaphore was successfully acquired, return 0.
        return 0;
    }
}
