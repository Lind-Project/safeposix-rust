#![allow(dead_code)]

// System related system calls
use crate::interface;
use crate::safeposix::cage::{Arg, CAGE_TABLE, PIPE_TABLE, Cage, Errno, FileDescriptor::*, FSData, Rlimit, StatData};
use crate::safeposix::filesystem::{FS_METADATA, Inode, metawalk, decref_dir};
use crate::safeposix::net::{NET_METADATA};

use super::sys_constants::*;
use super::fs_constants::*;

impl Cage {
    pub fn fork_syscall(&self, child_cageid: u64) -> i32 {
        let mutcagetable = &CAGE_TABLE;

        //construct new cage struct with a cloned fdtable
        let newfdtable = interface::RustHashMap::new();
        {
            //dashmap doesn't allow you to get key, value pairs directly, it only allows you to get a
            //RefMulti struct which can be decomposed into the key and value
            for refmulti in self.filedescriptortable.iter() {
                let (key, value) = refmulti.pair();
                let fd = value.read();

                //only file inodes have real inode objects currently
                match &*fd {
                    File(_normalfile_filedesc_obj) => {
                        let inodenum_option = if let File(f) = &*fd {Some(f.inode)} else {None};

                        if let Some(inodenum) = inodenum_option {
                            //increment the reference count on the inode
                            let mut inode = FS_METADATA.inodetable.get_mut(&inodenum).unwrap();
                            match *inode {
                                Inode::File(ref mut f) => {f.refcount += 1;}
                                Inode::CharDev(ref mut f) => {f.refcount += 1;}
                                Inode::Dir(ref mut f) => {f.refcount += 1;}
                            }
                        }
                    }
                    Pipe(pipe_filedesc_obj) => {
                        let pipe = PIPE_TABLE.get(&pipe_filedesc_obj.pipe).unwrap().clone();
                        pipe.incr_ref(pipe_filedesc_obj.flags)
                    }
                    Socket(socket_filedesc_obj) => {
                        if let Some(socknum) = socket_filedesc_obj.socketobjectid {
                            NET_METADATA.socket_object_table.get_mut(&socknum).unwrap().write().refcnt += 1;
                        }
                    }
                    _ => {}
                }
                
                let newfdobj = (&*fd).clone();
                let wrappedfd = interface::RustRfc::new(interface::RustLock::new(newfdobj));

                newfdtable.insert(*key, wrappedfd); //add deep copied fd to hashmap

            }
            let cwd_container = self.cwd.read();
            if let Some(cwdinodenum) = metawalk(&cwd_container) {
                if let Inode::Dir(ref mut cwddir) = *(FS_METADATA.inodetable.get_mut(&cwdinodenum).unwrap()) {
                    cwddir.refcount += 1;
                } else {panic!("We changed from a directory that was not a directory in chdir!");}
            } else {panic!("We changed from a directory that was not a directory in chdir!");}
        }
        let cageobj = Cage {
            cageid: child_cageid, cwd: interface::RustLock::new(self.cwd.read().clone()), parent: self.cageid,
            filedescriptortable: newfdtable,
            getgid: interface::RustAtomicI32::new(self.getgid.load(interface::RustAtomicOrdering::Relaxed)), 
            getuid: interface::RustAtomicI32::new(self.getuid.load(interface::RustAtomicOrdering::Relaxed)), 
            getegid: interface::RustAtomicI32::new(self.getegid.load(interface::RustAtomicOrdering::Relaxed)), 
            geteuid: interface::RustAtomicI32::new(self.geteuid.load(interface::RustAtomicOrdering::Relaxed))
            // This happens because self.getgid tries to copy atomic value which does not implement "Copy" trait; self.getgid.load returns i32.
        };
        mutcagetable.insert(child_cageid, interface::RustRfc::new(cageobj));
        0
    }

    pub fn exec_syscall(&self, child_cageid: u64) -> i32 {
        {CAGE_TABLE.remove(&self.cageid).unwrap();}
        
        let mut cloexecvec = vec!();
        let iterator = self.filedescriptortable.iter();
        for pair in iterator {
            let (&fdnum, inode) = pair.pair();
            if match &*inode.read() {
               File(f) => f.flags & O_CLOEXEC,
               Stream(s) => s.flags & O_CLOEXEC,
               Socket(s) => s.flags & O_CLOEXEC,
               Pipe(p) => p.flags & O_CLOEXEC,
               Epoll(p) => p.flags & O_CLOEXEC,
            } != 0 {
                cloexecvec.push(fdnum);
            }
        };
        for fdnum in cloexecvec {
            self.close_syscall(fdnum);
        }

        let newcage = Cage {cageid: child_cageid, cwd: interface::RustLock::new(self.cwd.read().clone()), 
            parent: self.parent, filedescriptortable: self.filedescriptortable.clone(),
            getgid: interface::RustAtomicI32::new(-1), 
            getuid: interface::RustAtomicI32::new(-1), 
            getegid: interface::RustAtomicI32::new(-1), 
            geteuid: interface::RustAtomicI32::new(-1)
        };
        //wasteful clone of fdtable, but mutability constraints exist

        {CAGE_TABLE.insert(child_cageid, interface::RustRfc::new(newcage))};
        0
    }

    pub fn exit_syscall(&self, status: i32) -> i32 {

        //flush anything left in stdout
        interface::flush_stdout();

        //close all remaining files in the fdtable
        {
            let fds_to_close = self.filedescriptortable.iter_mut().map(|x| *x.key()).collect::<Vec<i32>>();
            for fd in  fds_to_close {
                self._close_helper(fd);
            }
        }

        //get file descriptor table into a vector
        let cwd_container = self.cwd.read();
        decref_dir(&*cwd_container);

        //may not be removable in case of lindrustfinalize, we don't unwrap the remove result
        CAGE_TABLE.remove(&self.cageid);

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
