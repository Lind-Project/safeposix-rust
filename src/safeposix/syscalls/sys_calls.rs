#![allow(dead_code)]

// System related system calls
use crate::interface;
use crate::safeposix::cage::{*, FileDescriptor::*};
use crate::safeposix::filesystem::{FS_METADATA, Inode, metawalk, decref_dir};
use crate::safeposix::net::{NET_METADATA};
use crate::safeposix::shm::{SHM_METADATA};
use super::sys_constants::*;
use super::net_constants::*;
use super::fs_constants::*;

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
            
                    if segment.rmid && segment.shminfo.shm_nattch == 0 {
                        let key = segment.key;
                        occupied.remove_entry();
                        metadata.shmkeyidtable.remove(&key);
                    }
                }
                interface::RustHashEntry::Vacant(_) => {panic!("Shm entry not created for some reason");}
            };   
        }
    }

    pub fn fork_syscall(&self, child_cageid: u64) -> i32 {
        //construct a new mutex in the child cage where each initialized mutex is in the parent cage
        let mutextable = self.mutex_table.read();
        let mut new_mutex_table = vec!();
        for elem in mutextable.iter() {
            if elem.is_some() {
                let new_mutex_result = interface::RawMutex::create();
                match new_mutex_result {
                    Ok(new_mutex) => {new_mutex_table.push(Some(interface::RustRfc::new(new_mutex)))}
                        Err(_) => {
                        match Errno::from_discriminant(interface::get_errno()) {
                            Ok(i) => {return syscall_error(i, "fork", "The libc call to pthread_mutex_init failed!");},
                            Err(()) => panic!("Unknown errno value from pthread_mutex_init returned!"),
                        };
                    }
                }
            } else {
                new_mutex_table.push(None);
            }
        }
        drop(mutextable);

        //construct a new condvar in the child cage where each initialized condvar is in the parent cage
        let cvtable = self.cv_table.read();
        let mut new_cv_table = vec!();
        for elem in cvtable.iter() {
            if elem.is_some() {
                let new_cv_result = interface::RawCondvar::create();
                match new_cv_result {
                    Ok(new_cv) => {new_cv_table.push(Some(interface::RustRfc::new(new_cv)))}
                    Err(_) => {
                        match Errno::from_discriminant(interface::get_errno()) {
                            Ok(i) => {return syscall_error(i, "fork", "The libc call to pthread_cond_init failed!");},
                            Err(()) => panic!("Unknown errno value from pthread_cond_init returned!"),
                        };
                    }
                }
            } else {
                new_cv_table.push(None);
            }
        }
        drop(cvtable);

        //construct new cage struct with a cloned fdtable
        let newfdtable = init_fdtable();
        for fd in 0..MAXFD {
            let unlocked_fd = self.filedescriptortable[fd as usize].read();
            if let Some(filedesc_enum) = &*unlocked_fd {
                match filedesc_enum {
                    File(_normalfile_filedesc_obj) => {
                        let inodenum_option = if let File(f) = filedesc_enum {Some(f.inode)} else {None};

                        if let Some(inodenum) = inodenum_option {
                            //increment the reference count on the inode
                            let mut inode = FS_METADATA.inodetable.get_mut(&inodenum).unwrap();
                            match *inode {
                                Inode::File(ref mut f) => {f.refcount += 1;}
                                Inode::CharDev(ref mut f) => {f.refcount += 1;}
                                Inode::Socket(ref mut f) => {f.refcount += 1;}
                                Inode::Dir(ref mut f) => {f.refcount += 1;}
                            }
                        }
                    }
                    Pipe(pipe_filedesc_obj) => {
                        pipe_filedesc_obj.pipe.incr_ref(pipe_filedesc_obj.flags)
                    }
                    Socket(socket_filedesc_obj) => {
                        let sock_tmp = socket_filedesc_obj.handle.clone();
                        let mut sockhandle = sock_tmp.write();
                        if let Some(uinfo) = &mut sockhandle.unix_info {
                            if let Inode::Socket(ref mut sock) = *(FS_METADATA.inodetable.get_mut(&uinfo.inode).unwrap()) { 
                                sock.refcount += 1;
                            }
                        }
                    }
                    _ => {}
                }
                
                let newfdobj = filedesc_enum.clone();

                let _insertval = newfdtable[fd as usize].write().insert(newfdobj); //add deep copied fd to fd table
            }

        }
        let cwd_container = self.cwd.read();
        if let Some(cwdinodenum) = metawalk(&cwd_container) {
            if let Inode::Dir(ref mut cwddir) = *(FS_METADATA.inodetable.get_mut(&cwdinodenum).unwrap()) {
                cwddir.refcount += 1;
            } else {panic!("We changed from a directory that was not a directory in chdir!");}
        } else {panic!("We changed from a directory that was not a directory in chdir!");}

        let cageobj = Cage {
            cageid: child_cageid, cwd: interface::RustLock::new(self.cwd.read().clone()), parent: self.cageid,
            filedescriptortable: newfdtable,
            cancelstatus: interface::RustAtomicBool::new(false),
            // This happens because self.getgid tries to copy atomic value which does not implement "Copy" trait; self.getgid.load returns i32.
            getgid: interface::RustAtomicI32::new(self.getgid.load(interface::RustAtomicOrdering::Relaxed)), 
            getuid: interface::RustAtomicI32::new(self.getuid.load(interface::RustAtomicOrdering::Relaxed)), 
            getegid: interface::RustAtomicI32::new(self.getegid.load(interface::RustAtomicOrdering::Relaxed)), 
            geteuid: interface::RustAtomicI32::new(self.geteuid.load(interface::RustAtomicOrdering::Relaxed)),
            rev_shm: interface::Mutex::new((*self.rev_shm.lock()).clone()),
            mutex_table: interface::RustLock::new(new_mutex_table),
            cv_table: interface::RustLock::new(new_cv_table),
            thread_table: interface::RustHashMap::new(),
            signalhandler: self.signalhandler.clone(),
            sigset: interface::RustAtomicU64::new(self.sigset.load(interface::RustAtomicOrdering::Relaxed)),
            main_threadid: interface::get_pthreadid(),
            interval_timer: interface::IntervalTimer::new(child_cageid),
        };

        let shmtable = &SHM_METADATA.shmtable;
        //update fields for shared mappings in cage
        for rev_mapping in cageobj.rev_shm.lock().iter() {
            let mut shment = shmtable.get_mut(&rev_mapping.1).unwrap();
            shment.shminfo.shm_nattch += 1;
        }
        drop(shmtable);
        interface::cagetable_insert(child_cageid, cageobj);

        0
    }

    pub fn exec_syscall(&self, child_cageid: u64) -> i32 {
        interface::cagetable_remove(self.cageid);

        self.unmap_shm_mappings();

        let mut cloexecvec = vec!();
        for fd in 0..MAXFD {
            let unlocked_fd = self.filedescriptortable[fd as usize].read();
            if let Some(filedesc_enum) = &*unlocked_fd {
                if match filedesc_enum {
                    File(f) => f.flags & O_CLOEXEC,
                    Stream(s) => s.flags & O_CLOEXEC,
                    Socket(s) => s.flags & O_CLOEXEC,
                    Pipe(p) => p.flags & O_CLOEXEC,
                    Epoll(p) => p.flags & O_CLOEXEC,
                } != 0 { cloexecvec.push(fd); }
            }
        };
        for fdnum in cloexecvec {
            self.close_syscall(fdnum);
        }

        let newcage = Cage {cageid: child_cageid, cwd: interface::RustLock::new(self.cwd.read().clone()), 
            parent: self.parent, 
            filedescriptortable: self.filedescriptortable.clone(),
            cancelstatus: interface::RustAtomicBool::new(false),
            getgid: interface::RustAtomicI32::new(-1), 
            getuid: interface::RustAtomicI32::new(-1), 
            getegid: interface::RustAtomicI32::new(-1), 
            geteuid: interface::RustAtomicI32::new(-1),
            rev_shm: interface::Mutex::new(vec!()),
            mutex_table: interface::RustLock::new(vec!()),
            cv_table: interface::RustLock::new(vec!()),
            thread_table: interface::RustHashMap::new(),
            signalhandler: interface::RustHashMap::new(),
            sigset: interface::RustAtomicU64::new(self.sigset.load(interface::RustAtomicOrdering::Relaxed)),
            main_threadid: interface::get_pthreadid(),
            interval_timer: self.interval_timer.clone(),
        };
        //wasteful clone of fdtable, but mutability constraints exist

        interface::cagetable_insert(child_cageid, newcage);
        0
    }

    pub fn exit_syscall(&self, status: i32) -> i32 {

        //flush anything left in stdout
        interface::flush_stdout();

        self.unmap_shm_mappings();

        // close fds
        for fd in 0..MAXFD {
            self._close_helper(fd);
        }

        //get file descriptor table into a vector
        let cwd_container = self.cwd.read();
        decref_dir(&*cwd_container);

        //may not be removable in case of lindrustfinalize, we don't unwrap the remove result
        interface::cagetable_remove(self.cageid);
        
        // Trigger SIGCHLD
        if self.cageid != self.parent {
            interface::lind_kill(self.parent, SIGCHLD);
        }

        //fdtable will be dropped at end of dispatcher scope because of Arc
        status
    }

    pub fn getpid_syscall(&self) -> i32 {
        self.cageid as i32 //not sure if this is quite what we want but it's easy enough to change later
    }
    pub fn getppid_syscall(&self) -> i32 {
        self.parent as i32 // mimicing the call above -- easy to change later if necessary
    }

    /*if its negative 1
    return -1, but also set the values in the cage struct to the DEFAULTs for future calls*/
    pub fn getgid_syscall(&self) -> i32 {
        if self.getgid.load(interface::RustAtomicOrdering::Relaxed) == -1 {
            self.getgid.store(DEFAULT_GID as i32, interface::RustAtomicOrdering::Relaxed);
            return -1
        }   
        DEFAULT_GID as i32 //Lind is only run in one group so a default value is returned
    }
    pub fn getegid_syscall(&self) -> i32 {
        if self.getegid.load(interface::RustAtomicOrdering::Relaxed) == -1 {
            self.getegid.store(DEFAULT_GID as i32, interface::RustAtomicOrdering::Relaxed);
            return -1
        } 
        DEFAULT_GID as i32 //Lind is only run in one group so a default value is returned
    }

    pub fn getuid_syscall(&self) -> i32 {
        if self.getuid.load(interface::RustAtomicOrdering::Relaxed) == -1 {
            self.getuid.store(DEFAULT_UID as i32, interface::RustAtomicOrdering::Relaxed);
            return -1
        } 
        DEFAULT_UID as i32 //Lind is only run as one user so a default value is returned
    }
    pub fn geteuid_syscall(&self) -> i32 {
        if self.geteuid.load(interface::RustAtomicOrdering::Relaxed) == -1 {
            self.geteuid.store(DEFAULT_UID as i32, interface::RustAtomicOrdering::Relaxed);
            return -1
        } 
        DEFAULT_UID as i32 //Lind is only run as one user so a default value is returned
    }

    pub fn sigaction_syscall(&self, sig: i32, act: Option<&interface::SigactionStruct>, oact: Option<&mut interface::SigactionStruct>) -> i32 {
        if let Some(some_oact) = oact {
            let old_sigactionstruct = self.signalhandler.get(&sig);

            if let Some(entry) = old_sigactionstruct {
                some_oact.clone_from(entry.value());
            } else {
                return syscall_error(Errno::EINVAL, "sigaction", "Signal action unmodified in lind")
            }
        }

        if let Some(some_act) = act {
            if sig == 9 || sig == 19 { // Disallow changing the action for SIGKILL and SIGSTOP
                return syscall_error(Errno::EINVAL, "sigaction", "Cannot modify the action of SIGKILL or SIGSTOP");
            }

            self.signalhandler.insert(
                sig,
                some_act.clone()
            );
        }

        0
    }

    pub fn kill_syscall(&self, cage_id: i32, sig: i32) -> i32 {
        interface::lind_kill(cage_id as u64, sig)
    }

    pub fn sigprocmask_syscall(&self, how: i32, set: Option<& interface::SigsetType>, oldset: Option<&mut interface::SigsetType>) -> i32 {
        let mut res = 0;

        if let Some(some_oldset) = oldset {
            *some_oldset = self.sigset.load(interface::RustAtomicOrdering::Relaxed);
        }

        if let Some(some_set) = set {
            let curr_sigset = self.sigset.load(interface::RustAtomicOrdering::Relaxed);
            res = match how {
                0 => { // Block signals in set
                    self.sigset.store(curr_sigset | *some_set, interface::RustAtomicOrdering::Relaxed);
                    0
                },
                1 => { // Unblock signals in set
                    self.sigset.store(curr_sigset & !*some_set, interface::RustAtomicOrdering::Relaxed);
                    0
                },
                2 => { // Set sigset to set
                    self.sigset.store(*some_set, interface::RustAtomicOrdering::Relaxed);
                    0
                },
                _ => syscall_error(Errno::EINVAL, "sigprocmask", "Invalid value for how"),
            }
        }
        res
    }

    pub fn setitimer_syscall(&self, which: i32, new_value: Option<& interface::ITimerVal>, old_value: Option<&mut interface::ITimerVal>) -> i32 {
        match which {
            0 => { // ITMER_REAL
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
            },
            RLIMIT_STACK => {
                rlimit.rlim_cur = STACK_CUR;
                rlimit.rlim_max = STACK_MAX;
            },
            _ => return -1,
        }
        0
    }

    pub fn setrlimit(&self, res_type: u64, _limit_value: u64) -> i32 {
        match res_type{
            RLIMIT_NOFILE => {
                if NOFILE_CUR > NOFILE_MAX {-1} else {0}
                //FIXME: not implemented yet to update value in program
            },
            _ => -1,
        }
    }

}
