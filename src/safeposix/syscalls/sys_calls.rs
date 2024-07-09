//! This module contains all system calls that are being emulated/faked in Lind.
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
//! ## System Calls
//!
//! This module contains all system calls that are being emulated/faked in Lind.
//!
//! - [fork_syscall](crate::safeposix::cage::Cage::fork_syscall)
//! - [exec_syscall](crate::safeposix::cage::Cage::exec_syscall)
//! - [exit_syscall](crate::safeposix::cage::Cage::exit_syscall)
//! - [getpid_syscall](crate::safeposix::cage::Cage::getpid_syscall)
//! - [getppid_syscall](crate::safeposix::cage::Cage::getppid_syscall)
//! - [getgid_syscall](crate::safeposix::cage::Cage::getgid_syscall)
//! - [getegid_syscall](crate::safeposix::cage::Cage::getegid_syscall)
//! - [getuid_syscall](crate::safeposix::cage::Cage::getuid_syscall)
//! - [geteuid_syscall](crate::safeposix::cage::Cage::geteuid_syscall)
//! - [sigaction_syscall](crate::safeposix::cage::Cage::sigaction_syscall)
//! - [kill_syscall](crate::safeposix::cage::Cage::kill_syscall)
//! - [sigprocmask_syscall](crate::safeposix::cage::Cage::sigprocmask_syscall)
//! - [setitimer_syscall](crate::safeposix::cage::Cage::setitimer_syscall)
//! - [getrlimit](crate::safeposix::cage::Cage::getrlimit)
//! - [setrlimit](crate::safeposix::cage::Cage::setrlimit)

#![allow(dead_code)]

// System related system calls
use super::fs_constants::*;
use super::net_constants::*;
use super::sys_constants::*;
use crate::interface;
use crate::safeposix::cage::{FileDescriptor::*, *};
use crate::safeposix::filesystem::{decref_dir, metawalk, Inode, FS_METADATA};
use crate::safeposix::net::NET_METADATA;
use crate::safeposix::shm::SHM_METADATA;

impl Cage {
    fn unmap_shm_mappings(&self) {
        //unmap shm mappings on exit or exec
        for rev_mapping in self.rev_shm.lock().iter() {
            let shmid = rev_mapping.1;
            let metadata = &SHM_METADATA;
            match metadata.shmtable.entry(shmid) {
                interface::RustHashEntry::Occupied(mut occupied) => {
                    let segment = occupied.get_mut();
                    segment.shminfo.shm_nattch -= 1;
                    segment.shminfo.shm_dtime = interface::timestamp() as isize;
                    segment.attached_cages.remove(&self.cageid);

                    if segment.rmid && segment.shminfo.shm_nattch == 0 {
                        let key = segment.key;
                        occupied.remove_entry();
                        metadata.shmkeyidtable.remove(&key);
                    }
                }
                interface::RustHashEntry::Vacant(_) => {
                    panic!("Shm entry not created for some reason");
                }
            };
        }
    }

    /// ### Description
    ///
    ///'fork_syscall` creates a new process (cage object)
    /// The newly created child process is an exact copy of the
    /// parent process (the process that calls fork)
    /// apart from it's cage_id and the parent_id
    /// In this function we clone the mutex table, condition variables table,
    /// semaphore table and the file descriptors and create
    /// a new Cage object with these cloned tables.
    /// We also update the shared memory mappings - and create mappings
    /// from the new Cage object the the
    /// parent Cage object's memory mappings.
    ///
    /// ### Arguments
    ///
    /// It accepts one parameter:
    ///
    /// * `child_cageid` : an integer representing the pid of the child process
    ///
    /// ### Errors
    ///    
    /// There are 2 scenarios where the call to `fork_syscall` might return an
    /// error
    ///
    /// * When the RawMutex::create() call fails to create a new Mutex object
    /// * When the RawCondvar::create() call fails to create a new Condition
    ///   Variable object
    ///
    /// ### Returns
    ///
    /// On success it returns a value of 0, and the new child Cage object is
    /// added to Cagetable
    ///
    /// ### Panics
    ///
    /// This system call has no scenarios where it panics
    ///
    /// To learn more about the syscall and possible error values, see
    /// [fork(2)](https://man7.org/linux/man-pages/man2/fork.2.html)

    pub fn fork_syscall(&self, child_cageid: u64) -> i32 {
        //Create a new mutex table that replicates the mutex table of the parent
        // (calling) Cage object Since the child process inherits all the locks
        // that the parent process holds,
        let mutextable = self.mutex_table.read();
        // Initialize the child object's mutex table
        let mut new_mutex_table = vec![];
        //Loop through each element in the mutex table
        //Each entry in the mutex table represents a `lock` which the parent process
        // holds Copying them into the child's Cage exhibits the inheritance of
        // the lock
        for elem in mutextable.iter() {
            if elem.is_some() {
                //If the mutex is `Some` - we create a new mutex and store it in the child's
                // mutex table The create method returns a new struct obejct
                // that represents a Mutex
                let new_mutex_result = interface::RawMutex::create();
                match new_mutex_result {
                    // If the mutex creation is successful we push it on the child's table
                    Ok(new_mutex) => new_mutex_table.push(Some(interface::RustRfc::new(new_mutex))),
                    // If the mutex creation returns an error, we abort the system call and return
                    // the appropriate error
                    Err(_) => {
                        match Errno::from_discriminant(interface::get_errno()) {
                            Ok(i) => {
                                return syscall_error(
                                    i,
                                    "fork",
                                    "The libc call to pthread_mutex_init failed!",
                                );
                            }
                            Err(()) => {
                                panic!("Unknown errno value from pthread_mutex_init returned!")
                            }
                        };
                    }
                }
            } else {
                // If the mutex is `None` - we mimic the same behavior in the child's mutex
                // table
                new_mutex_table.push(None);
            }
        }
        drop(mutextable);

        //Construct a replica of the condition variables table in the child cage object
        //This table stores condition variables - which are special variables that the
        // process uses to determine whether certain conditions have been met or
        // not. Threads use condition variables to stop or resume their
        // operation depending on the value of these variables. Read the CondVar
        // table of the calling process
        let cvtable = self.cv_table.read();
        // Initialize the table for the child process
        let mut new_cv_table = vec![];
        // Loop through all the variables in the parent's table
        for elem in cvtable.iter() {
            if elem.is_some() {
                //Create a condvar to store in the child's Cage object
                //Returns the condition variable struct object which implements theb signal,
                // wait, broadcast and timed_wait methods
                let new_cv_result = interface::RawCondvar::create();
                match new_cv_result {
                    // If the result of the creation of the RawCondVar is successful - push it onto
                    // the child's mutex table
                    Ok(new_cv) => new_cv_table.push(Some(interface::RustRfc::new(new_cv))),
                    // If the creation was unsucessful - return an Error
                    Err(_) => {
                        match Errno::from_discriminant(interface::get_errno()) {
                            Ok(i) => {
                                return syscall_error(
                                    i,
                                    "fork",
                                    "The libc call to pthread_cond_init failed!",
                                );
                            }
                            Err(()) => {
                                panic!("Unknown errno value from pthread_cond_init returned!")
                            }
                        };
                    }
                }
            } else {
                // If the value is None - mimic the behavior in the child's condition variable
                // table
                new_cv_table.push(None);
            }
        }
        drop(cvtable);

        //Clone the file descriptor table in the child's Cage object
        //Each entry in the file descriptor table points to an open file description
        // which in turn references the actual inodes of the files on disk
        let newfdtable = init_fdtable();
        //Loop from 0 to maximum value of file descriptor index
        for fd in 0..MAXFD {
            let checkedfd = self.get_filedescriptor(fd).unwrap();
            //Get the lock for the file descriptor
            let unlocked_fd = checkedfd.read();
            if let Some(filedesc_enum) = &*unlocked_fd {
                // Check the type of the file descriptor
                match filedesc_enum {
                    // If the fd is linked to a file
                    File(_normalfile_filedesc_obj) => {
                        let inodenum_option = if let File(f) = filedesc_enum {
                            Some(f.inode)
                        } else {
                            None
                        };

                        if let Some(inodenum) = inodenum_option {
                            let mut inode = FS_METADATA.inodetable.get_mut(&inodenum).unwrap();
                            //Since the child Cage also inherits the parent's fd table
                            //We increment the reference count on the actual inodes of the files on
                            // disk Since the child Cage is a new
                            // process that also references those files
                            match *inode {
                                Inode::File(ref mut f) => {
                                    f.refcount += 1;
                                }
                                Inode::CharDev(ref mut f) => {
                                    f.refcount += 1;
                                }
                                Inode::Socket(ref mut f) => {
                                    f.refcount += 1;
                                }
                                Inode::Dir(ref mut f) => {
                                    f.refcount += 1;
                                }
                            }
                        }
                    }
                    // If the fd is linked to a pipe increment the ref count of the pipe
                    Pipe(pipe_filedesc_obj) => {
                        pipe_filedesc_obj.pipe.incr_ref(pipe_filedesc_obj.flags)
                    }
                    // If the fd is linked to a socket increment the ref count of the socket
                    Socket(socket_filedesc_obj) => {
                        // Check if it is a domain socket
                        let sock_tmp = socket_filedesc_obj.handle.clone();
                        let mut sockhandle = sock_tmp.write();
                        let socket_type = sockhandle.domain;
                        //Here we only increment the reference for AF_UNIX socket type
                        //Since these are the only sockets that have an inode associated with them
                        if socket_type == AF_UNIX {
                            //Increment the appropriate reference counter of the correct socket
                            //Each socket has two pipes associated with them - a read and write
                            // pipe Here we grab these two pipes and
                            // increment their references individually
                            // And also increment the reference count of the socket as a whole
                            if let Some(sockinfo) = &sockhandle.unix_info {
                                if let Some(sendpipe) = sockinfo.sendpipe.as_ref() {
                                    sendpipe.incr_ref(O_WRONLY);
                                }
                                if let Some(receivepipe) = sockinfo.receivepipe.as_ref() {
                                    receivepipe.incr_ref(O_RDONLY);
                                }
                                if let Inode::Socket(ref mut sock) =
                                    *(FS_METADATA.inodetable.get_mut(&sockinfo.inode).unwrap())
                                {
                                    sock.refcount += 1;
                                }
                            }
                        }
                        drop(sockhandle);
                    }
                    _ => {}
                }

                let newfdobj = filedesc_enum.clone();
                // Insert the file descriptor object into the new file descriptor table
                let _insertval = newfdtable[fd as usize].write().insert(newfdobj);
            }
        }

        //We read the current working directory of the parent Cage object
        let cwd_container = self.cwd.read();
        //We try to resolve the inode of the current working directory - if the
        //resolution is successful we update the reference count of the current working
        // directory since the newly created Child cage object also references
        // the same directory If the resolution is not successful - the code
        // panics since the cwd's inode cannot be resolved correctly
        if let Some(cwdinodenum) = metawalk(&cwd_container) {
            if let Inode::Dir(ref mut cwddir) =
                *(FS_METADATA.inodetable.get_mut(&cwdinodenum).unwrap())
            {
                cwddir.refcount += 1;
            } else {
                panic!("We changed from a directory that was not a directory in chdir!");
            }
        } else {
            panic!("We changed from a directory that was not a directory in chdir!");
        }

        // We clone the parent cage's main threads and store them and index 0
        // This is done since there isn't a thread established for the child Cage object
        // yet - And there is no threadId to store it at.
        // The child Cage object can then initialize and store the sigset appropriately
        // when it establishes its own main thread id.
        let newsigset = interface::RustHashMap::new();
        // Here we check if Lind is being run under the test suite or not
        if !interface::RUSTPOSIX_TESTSUITE.load(interface::RustAtomicOrdering::Relaxed) {
            // When rustposix runs independently (not as Lind paired with NaCL runtime) we
            // do not handle signals The test suite runs rustposix independently
            // and hence we do not handle signals for the test suite
            let mainsigsetatomic = self
                .sigset
                .get(
                    &self
                        .main_threadid
                        .load(interface::RustAtomicOrdering::Relaxed),
                )
                .unwrap();
            let mainsigset = interface::RustAtomicU64::new(
                mainsigsetatomic.load(interface::RustAtomicOrdering::Relaxed),
            );
            // Insert the parent cage object's main threads sigset and store them at index 0
            newsigset.insert(0, mainsigset);
        }

        // Construct a new semaphore table in child cage which equals to the one in the
        // parent cage
        let semtable = &self.sem_table;
        let new_semtable: interface::RustHashMap<
            u32,
            interface::RustRfc<interface::RustSemaphore>,
        > = interface::RustHashMap::new();
        // Loop all pairs of semaphores and insert their copies into the new semaphore
        // table Each pair consists of a key which is 32 bit unsigned integer
        // And a Semaphore Object implemented as RustSemaphore
        for pair in semtable.iter() {
            new_semtable.insert((*pair.key()).clone(), pair.value().clone());
        }

        // Create a new cage object using the cloned tables and the child id passed as a
        // parameter
        let cageobj = Cage {
            cageid: child_cageid,
            cwd: interface::RustLock::new(self.cwd.read().clone()),
            // Setting the parent to be the current Cage object
            parent: self.cageid,
            // Setting the fd table with our cloned fd table
            filedescriptortable: newfdtable,
            cancelstatus: interface::RustAtomicBool::new(false),
            // Intitialize IDs with the default value
            // This happens because self.getgid tries to copy atomic value which does not implement
            // "Copy" trait; self.getgid.load returns i32.
            getgid: interface::RustAtomicI32::new(
                self.getgid.load(interface::RustAtomicOrdering::Relaxed),
            ),
            getuid: interface::RustAtomicI32::new(
                self.getuid.load(interface::RustAtomicOrdering::Relaxed),
            ),
            getegid: interface::RustAtomicI32::new(
                self.getegid.load(interface::RustAtomicOrdering::Relaxed),
            ),
            geteuid: interface::RustAtomicI32::new(
                self.geteuid.load(interface::RustAtomicOrdering::Relaxed),
            ),
            // Clone the reverse shm mappings
            rev_shm: interface::Mutex::new((*self.rev_shm.lock()).clone()),
            // Setting the mutex tables with our copy of the mutex table
            mutex_table: interface::RustLock::new(new_mutex_table),
            // Setting the condition variables table with our copy
            cv_table: interface::RustLock::new(new_cv_table),
            // Setting the semaphores table with our copy
            sem_table: new_semtable,
            // Creating a new empty table for storing threads of the child Cage object
            thread_table: interface::RustHashMap::new(),
            // Cloning the signal handler of the parent Cage object
            signalhandler: self.signalhandler.clone(),
            // Setting the signal set with the cloned and altered sigset
            sigset: newsigset,
            // Creating a new copy for the pending signal set
            pendingsigset: interface::RustHashMap::new(),
            // Setting the main thread id to 0 - since it is uninitialized
            main_threadid: interface::RustAtomicU64::new(0),
            // Creating a new timer for the process with id = child_cageid
            interval_timer: interface::IntervalTimer::new(child_cageid),
        };

        let shmtable = &SHM_METADATA.shmtable;
        // Updating the shared mappings in the child cage object
        // Loop through all the reverse mappings in the new cage object
        for rev_mapping in cageobj.rev_shm.lock().iter() {
            let mut shment = shmtable.get_mut(&rev_mapping.1).unwrap();
            shment.shminfo.shm_nattch += 1;
            // Get the references of the curret cage id
            let refs = shment.attached_cages.get(&self.cageid).unwrap();
            // Copy the references
            let childrefs = refs.clone();
            drop(refs);
            // Create references from the new Cage object to the copied references
            shment.attached_cages.insert(child_cageid, childrefs);
        }
        // Inserting the child Cage object at the appropriate index in the Cage table
        interface::cagetable_insert(child_cageid, cageobj);

        0
    }

    pub fn exec_syscall(&self, child_cageid: u64) -> i32 {
        interface::cagetable_remove(self.cageid);

        self.unmap_shm_mappings();

        let mut cloexecvec = vec![];
        for fd in 0..MAXFD {
            let checkedfd = self.get_filedescriptor(fd).unwrap();
            let unlocked_fd = checkedfd.read();
            if let Some(filedesc_enum) = &*unlocked_fd {
                if match filedesc_enum {
                    File(f) => f.flags & O_CLOEXEC,
                    Stream(s) => s.flags & O_CLOEXEC,
                    Socket(s) => s.flags & O_CLOEXEC,
                    Pipe(p) => p.flags & O_CLOEXEC,
                    Epoll(p) => p.flags & O_CLOEXEC,
                } != 0
                {
                    cloexecvec.push(fd);
                }
            }
        }

        for fdnum in cloexecvec {
            self.close_syscall(fdnum);
        }

        // we grab the parent cages main threads sigset and store it at 0
        // this way the child can initialize the sigset properly when it establishes its
        // own mainthreadid
        let newsigset = interface::RustHashMap::new();
        if !interface::RUSTPOSIX_TESTSUITE.load(interface::RustAtomicOrdering::Relaxed) {
            // we don't add these for the test suite
            let mainsigsetatomic = self
                .sigset
                .get(
                    &self
                        .main_threadid
                        .load(interface::RustAtomicOrdering::Relaxed),
                )
                .unwrap();
            let mainsigset = interface::RustAtomicU64::new(
                mainsigsetatomic.load(interface::RustAtomicOrdering::Relaxed),
            );
            newsigset.insert(0, mainsigset);
        }

        let newcage = Cage {
            cageid: child_cageid,
            cwd: interface::RustLock::new(self.cwd.read().clone()),
            parent: self.parent,
            filedescriptortable: self.filedescriptortable.clone(),
            cancelstatus: interface::RustAtomicBool::new(false),
            getgid: interface::RustAtomicI32::new(-1),
            getuid: interface::RustAtomicI32::new(-1),
            getegid: interface::RustAtomicI32::new(-1),
            geteuid: interface::RustAtomicI32::new(-1),
            rev_shm: interface::Mutex::new(vec![]),
            mutex_table: interface::RustLock::new(vec![]),
            cv_table: interface::RustLock::new(vec![]),
            sem_table: interface::RustHashMap::new(),
            thread_table: interface::RustHashMap::new(),
            signalhandler: interface::RustHashMap::new(),
            sigset: newsigset,
            pendingsigset: interface::RustHashMap::new(),
            main_threadid: interface::RustAtomicU64::new(0),
            interval_timer: self.interval_timer.clone_with_new_cageid(child_cageid),
        };
        //wasteful clone of fdtable, but mutability constraints exist

        interface::cagetable_insert(child_cageid, newcage);
        0
    }

    /// ### Description
    /// 
    /// The exit function causes normal process(Cage) termination
    /// The termination entails unmapping all memory references
    /// Removing the cage object from the cage table, closing all open files
    /// And decrement all references to files and directories
    /// For more information please refer [https://man7.org/linux/man-pages/man3/exit.3.html]
    /// 
    /// ### Arguments 
    /// 
    /// The exit function takes only one argument which is `status`
    /// `status` : This is a 32 bit integer value that the function returns back 
    /// upon sucessfully terminating the process
    /// 
    /// ### Returns
    /// 
    /// This function returns a 32 bit integer value - which represents succesful 
    /// termination of the calling Cage object
    /// 
    /// ### Panics
    /// 
    /// While this syscall does not panic directly - it can panic if the 
    /// `decref_dir` function panics - which occurs when the working directory
    /// passed to it is not a valid directory or the directory did not exist at all. 
    /// or if the cage_id passed to the remove function is not a valid cage id. 
    pub fn exit_syscall(&self, status: i32) -> i32 {
        //Clear all values in stdout stream
        interface::flush_stdout();
        //Unmap all memory mappings for the current cage object
        self.unmap_shm_mappings();

        //For all file descriptors that the cage holds
        for fd in 0..MAXFD {
            // Close the file pointed to by the file descriptor
            self._close_helper(fd);
        }

        //Read the current working directory (acquire a lock)
        let cwd_container = self.cwd.read();
        //For all inodes to which the current cage object points to
        //Decrement their reference count
        decref_dir(&*cwd_container);

        //may not be removable in case of lindrustfinalize, we don't unwrap the remove
        //Remove the current cage object from the cage table
        interface::cagetable_remove(self.cageid);

        // Check if Lind is being run as a test suite or not
        if !interface::RUSTPOSIX_TESTSUITE.load(interface::RustAtomicOrdering::Relaxed) {
            // Trigger SIGCHILD if LIND is not run as a test suite
            if self.cageid != self.parent {
                interface::lind_kill_from_id(self.parent, SIGCHLD);
            }
        }

        // Return the status integer back to the calling function
        status
    }

    /// ### Description
    ///  
    /// The `getpid_syscall()` system call returns the id of the calling
    /// process. The uid is guaranteed to be unique and can be used for
    /// naming temporary files. The call is always successful.
    ///
    /// ### Arguments
    ///
    /// This system call does not take any arguments.
    ///
    /// ### Returns
    ///
    /// Returns the 32 bit integer uid of the calling Cage object
    pub fn getpid_syscall(&self) -> i32 {
        self.cageid as i32
    }

    /// ### Description
    ///
    /// The `getppid_syscall()` returns the id of the parent process of the
    /// calling process. The uid is guaranteed to be unique, and like the
    /// getpid call, this call is also always successful. This call is
    /// always successfull
    ///
    /// ### Arguments
    /// The getppid syscall does not take any arguments
    ///
    /// ### Returns
    /// Returns a 32 bit integer value that represents the unique id of the
    /// parent process.
    pub fn getppid_syscall(&self) -> i32 {
        self.parent as i32 // mimicing the call above -- easy to change later if
                           // necessary
    }

    /// ### Description
    ///
    /// This function returns the real group id of the calling process. The real
    /// group id is specified at login time. The group id is the group of
    /// the user who invoked the program. Lind is only run in one group -
    /// and hence a default value is expected from this function.
    /// Initially we check if the call takes place during the loading stage, and
    /// return -1 if yes and set the gid to be the default value.
    ///
    /// ### Arguments
    ///
    /// The `getgid_syscall` does not take any argument.
    ///
    /// ### Returns
    ///
    /// Depending on whether the gid has been initialized or not this function
    /// returns either -1 or the default gid as a 32 bit integer.
    pub fn getgid_syscall(&self) -> i32 {
        // We return -1 for the first call for compatibility with the dynamic loader.
        // For subsequent calls we return our default value.
        if self.getgid.load(interface::RustAtomicOrdering::Relaxed) == -1 {
            self.getgid
                .store(DEFAULT_GID as i32, interface::RustAtomicOrdering::Relaxed);
            return -1;
        }
        DEFAULT_GID as i32 //Lind is only run in one group so a default value
                           // is returned
    }

    /// ### Description
    ///
    /// The `getegid_syscall` returns the effective group id of the user who
    /// invoked the process. Since Lind is only run in one group a default
    /// value (or -1) is returned. Initially we check if the call takes
    /// place during the loading stage, and return -1 if yes and set the
    /// egid to be the default value.
    ///
    /// ### Arguments
    ///
    /// This syscall does not take any arguments
    ///
    /// ### Returns
    ///
    /// Returns a 32 bit integer value (or -1) which represents the effective
    /// group
    pub fn getegid_syscall(&self) -> i32 {
        // We return -1 for the first call for compatibility with the dynamic loader.
        // For subsequent calls we return our default value.
        if self.getegid.load(interface::RustAtomicOrdering::Relaxed) == -1 {
            self.getegid
                .store(DEFAULT_GID as i32, interface::RustAtomicOrdering::Relaxed);
            return -1;
        }
        DEFAULT_GID as i32 //Lind is only run in one group so a default value
                           // is returned
    }

    /// ### Description
    ///
    /// The `getuid_syscall` returns the real user id of the calling process.
    /// The real user id is the user who invoked the calling process.
    /// As Lind only allows one user, a default value is returned.
    /// Initially we check if the call takes place during the loading stage, and
    /// return -1 if yes and set the uid to be the default value.
    ///
    /// ### Arguments
    ///  
    /// The `getuid_syscall` does not take any arguments
    ///
    /// ### Returns
    ///
    /// Returns a 32 bit default integer (or -1) representing the user
    pub fn getuid_syscall(&self) -> i32 {
        // We return -1 for the first call for compatibility with the dynamic loader.
        // For subsequent calls we return our default value.
        if self.getuid.load(interface::RustAtomicOrdering::Relaxed) == -1 {
            self.getuid
                .store(DEFAULT_UID as i32, interface::RustAtomicOrdering::Relaxed);
            return -1;
        }
        DEFAULT_UID as i32 //Lind is only run as one user so a default value is
                           // returned
    }

    /// ### Description
    ///
    /// The `geteuid_syscall` returns the effective user id of the calling
    /// process. As Lind only allows one user, a default value (or -1) is
    /// returned. Initially we check if the call takes place during the
    /// loading stage, and return -1 if yes and set the euid to be the
    /// default value.
    ///
    /// ### Function Arguments
    /// The geteuid syscall does not take any arguments
    ///
    /// ### Returns
    ///
    /// Returns a 32 bit default integer value (or -1) representing the
    /// effective user
    pub fn geteuid_syscall(&self) -> i32 {
        // We return -1 for the first call for compatibility with the dynamic loader.
        // For subsequent calls we return our default value.
        if self.geteuid.load(interface::RustAtomicOrdering::Relaxed) == -1 {
            self.geteuid
                .store(DEFAULT_UID as i32, interface::RustAtomicOrdering::Relaxed);
            return -1;
        }
        DEFAULT_UID as i32 //Lind is only run as one user so a default value is
                           // returned
    }

    pub fn sigaction_syscall(
        &self,
        sig: i32,
        act: Option<&interface::SigactionStruct>,
        oact: Option<&mut interface::SigactionStruct>,
    ) -> i32 {
        if let Some(some_oact) = oact {
            let old_sigactionstruct = self.signalhandler.get(&sig);

            if let Some(entry) = old_sigactionstruct {
                some_oact.clone_from(entry.value());
            } else {
                some_oact.clone_from(&interface::SigactionStruct::default()); // leave handler field as NULL
            }
        }

        if let Some(some_act) = act {
            if sig == 9 || sig == 19 {
                // Disallow changing the action for SIGKILL and SIGSTOP
                return syscall_error(
                    Errno::EINVAL,
                    "sigaction",
                    "Cannot modify the action of SIGKILL or SIGSTOP",
                );
            }

            self.signalhandler.insert(sig, some_act.clone());
        }

        0
    }

    pub fn kill_syscall(&self, cage_id: i32, sig: i32) -> i32 {
        if (cage_id < 0) || (cage_id >= interface::MAXCAGEID) {
            return syscall_error(Errno::EINVAL, "sigkill", "Invalid cage id.");
        }

        if let Some(cage) = interface::cagetable_getref_opt(cage_id as u64) {
            interface::lind_threadkill(
                cage.main_threadid
                    .load(interface::RustAtomicOrdering::Relaxed),
                sig,
            );
            return 0;
        } else {
            return syscall_error(Errno::ESRCH, "kill", "Target cage does not exist");
        }
    }

    pub fn sigprocmask_syscall(
        &self,
        how: i32,
        set: Option<&interface::SigsetType>,
        oldset: Option<&mut interface::SigsetType>,
    ) -> i32 {
        let mut res = 0;
        let pthreadid = interface::get_pthreadid();

        let sigset = self.sigset.get(&pthreadid).unwrap();

        if let Some(some_oldset) = oldset {
            *some_oldset = sigset.load(interface::RustAtomicOrdering::Relaxed);
        }

        if let Some(some_set) = set {
            let curr_sigset = sigset.load(interface::RustAtomicOrdering::Relaxed);
            res = match how {
                SIG_BLOCK => {
                    // Block signals in set
                    sigset.store(
                        curr_sigset | *some_set,
                        interface::RustAtomicOrdering::Relaxed,
                    );
                    0
                }
                SIG_UNBLOCK => {
                    // Unblock signals in set
                    let newset = curr_sigset & !*some_set;
                    let pendingsignals = curr_sigset & some_set;
                    sigset.store(newset, interface::RustAtomicOrdering::Relaxed);
                    self.send_pending_signals(pendingsignals, pthreadid);
                    0
                }
                SIG_SETMASK => {
                    // Set sigset to set
                    sigset.store(*some_set, interface::RustAtomicOrdering::Relaxed);
                    0
                }
                _ => syscall_error(Errno::EINVAL, "sigprocmask", "Invalid value for how"),
            }
        }
        res
    }

    pub fn setitimer_syscall(
        &self,
        which: i32,
        new_value: Option<&interface::ITimerVal>,
        old_value: Option<&mut interface::ITimerVal>,
    ) -> i32 {
        match which {
            ITIMER_REAL => {
                if let Some(some_old_value) = old_value {
                    let (curr_duration, next_duration) = self.interval_timer.get_itimer();
                    some_old_value.it_value.tv_sec = curr_duration.as_secs() as i64;
                    some_old_value.it_value.tv_usec = curr_duration.subsec_millis() as i64;
                    some_old_value.it_interval.tv_sec = next_duration.as_secs() as i64;
                    some_old_value.it_interval.tv_usec = next_duration.subsec_millis() as i64;
                }

                if let Some(some_new_value) = new_value {
                    let curr_duration = interface::RustDuration::new(
                        some_new_value.it_value.tv_sec as u64,
                        some_new_value.it_value.tv_usec as u32,
                    );
                    let next_duration = interface::RustDuration::new(
                        some_new_value.it_interval.tv_sec as u64,
                        some_new_value.it_interval.tv_usec as u32,
                    );

                    self.interval_timer.set_itimer(curr_duration, next_duration);
                }
            }

            _ => { /* ITIMER_VIRTUAL and ITIMER_PROF is not implemented*/ }
        }
        0
    }

    pub fn getrlimit(&self, res_type: u64, rlimit: &mut Rlimit) -> i32 {
        match res_type {
            RLIMIT_NOFILE => {
                rlimit.rlim_cur = NOFILE_CUR;
                rlimit.rlim_max = NOFILE_MAX;
            }
            RLIMIT_STACK => {
                rlimit.rlim_cur = STACK_CUR;
                rlimit.rlim_max = STACK_MAX;
            }
            _ => return -1,
        }
        0
    }

    pub fn setrlimit(&self, res_type: u64, _limit_value: u64) -> i32 {
        match res_type {
            RLIMIT_NOFILE => {
                if NOFILE_CUR > NOFILE_MAX {
                    -1
                } else {
                    0
                }
                //FIXME: not implemented yet to update value in program
            }
            _ => -1,
        }
    }
}
