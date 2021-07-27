#![allow(dead_code)]
#![allow(unused_variables)]
// retreive cage table
const ACCESS_SYSCALL: i32 = 2;
const UNLINK_SYSCALL: i32 = 4;
const LINK_SYSCALL: i32 = 5;
const CHDIR_SYSCALL: i32 = 6;
const MKDIR_SYSCALL: i32 = 7;
const RMDIR_SYSCALL: i32 = 8;
const XSTAT_SYSCALL: i32 = 9;
const OPEN_SYSCALL: i32 = 10;
const CLOSE_SYSCALL: i32 = 11;
const READ_SYSCALL: i32 = 12;
const WRITE_SYSCALL: i32 = 13;
const LSEEK_SYSCALL: i32 = 14;
const IOCTL_SYSCALL: i32 = 15;
const FXSTAT_SYSCALL: i32 = 17;
const FSTATFS_SYSCALL: i32 = 19;
const MMAP_SYSCALL: i32 = 21;
const MUNMAP_SYSCALL: i32 = 22;
const GETDENTS_SYSCALL: i32 = 23;
const DUP_SYSCALL: i32 = 24;
const DUP2_SYSCALL: i32 = 25;
const STATFS_SYSCALL: i32 = 26;
const FCNTL_SYSCALL: i32 = 28;
const GETPPID_SYSCALL: i32 = 29;
const EXIT_SYSCALL: i32 = 30;
const GETPID_SYSCALL: i32 = 31;
const SOCKET_SYSCALL: i32 = 32;
const BIND_SYSCALL: i32 = 33;
const SEND_SYSCALL: i32 = 34;
const SENDTO_SYSCALL: i32 = 35;
const RECV_SYSCALL: i32 = 36;
const RECVFROM_SYSCALL: i32 = 37;
const CONNECT_SYSCALL: i32 = 38;
const LISTEN_SYSCALL: i32 = 39;
const ACCEPT_SYSCALL: i32 = 40;
const GETPEERNAME_SYSCALL: i32 = 41;
const GETSOCKNAME_SYSCALL: i32 = 42;
const GETSOCKOPT_SYSCALL: i32 = 43;
const SETSOCKOPT_SYSCALL: i32 = 44;
const SHUTDOWN_SYSCALL: i32 = 45;
const SELECT_SYSCALL: i32 = 46;
const GETIFADDRS_SYSCALL: i32 = 47;
const POLL_SYSCALL: i32 = 48;
const SOCKETPAIR_SYSCALL: i32 = 49;
const GETUID_SYSCALL: i32 = 50;
const GETEUID_SYSCALL: i32 = 51;
const GETGID_SYSCALL: i32 = 52;
const GETEGID_SYSCALL: i32 = 53;
const FLOCK_SYSCALL: i32 = 54;
const RENAME_SYSCALL: i32 = 55;
const EPOLL_CREATE_SYSCALL: i32 = 56;
const EPOLL_CTL_SYSCALL: i32 = 57;
const EPOLL_WAIT_SYSCALL: i32 = 58;

const PIPE_SYSCALL: i32 = 66;
const PIPE2_SYSCALL: i32 = 67;
const FORK_SYSCALL: i32 = 68;
const EXEC_SYSCALL: i32 = 69;

const GETHOSTNAME_SYSCALL: i32 = 125;
const PREAD_SYSCALL: i32 = 126;
const PWRITE_SYSCALL: i32 = 127;


use crate::interface;
use super::cage::{CAGE_TABLE, Cage};
use super::syscalls::{sys_constants::*, fs_constants::*};
use super::filesystem::{FS_METADATA, load_fs, incref_root};


#[repr(C)]
pub union Arg {
  pub dispatch_int: i32,
  pub dispatch_uint: u32,
  pub dispatch_ulong: u64,
  pub dispatch_long: i64,
  pub dispatch_usize: usize, //For types not specified to be a given length, but often set to word size (i.e. size_t)
  pub dispatch_isize: isize, //For types not specified to be a given length, but often set to word size (i.e. off_t)
  pub dispatch_cbuf: *const u8, //Typically corresponds to an immutable void* pointer as in write
  pub dispatch_mutcbuf: *mut u8, //Typically corresponds to a mutable void* pointer as in read
  pub dispatch_cstr: *const i8, //Typically corresponds to a passed in string of type char*, as in open
  pub dispatch_cstrarr: *const *const i8, //Typically corresponds to a passed in string array of type char* const[] as in execve
  pub dispatch_rlimitstruct: *mut Rlimit,
  pub dispatch_statdatastruct: *mut StatData,
  pub dispatch_fsdatastruct: *mut FSData
}

pub extern "C" fn dispatcher(cageid: u64, callnum: i32, arg1: Arg, arg2: Arg, arg3: Arg, arg4: Arg, arg5: Arg, arg6: Arg) -> i32 {

    // need to match based on if cage exists
    let cage = { CAGE_TABLE.read().unwrap().get(&cageid).unwrap().clone() };

    //implement syscall method calling using matching
    //COMMENT: I know that the plan is to return the error integer, but, that seems like it could introduce vulnerabilities

    match callnum {
        ACCESS_SYSCALL => {
            match (interface::get_cstr(arg1), interface::get_uint(arg2)) {
                (Ok(cstr1), Ok(uint2)) => {
                    return cage.access_syscall(cstr1, uint2);
                }
                (Err(returned_error_code), ..) |
                (.., Err(returned_error_code)) => {
                    return returned_error_code;
                }
            }
        }
        UNLINK_SYSCALL => {
            match interface::get_cstr(arg1) {
                Ok(cstr1) => {
                    return cage.unlink_syscall(cstr1);
                }
                Err(returned_error_code) => {
                    return returned_error_code;
                }
            }
        }
        LINK_SYSCALL => {
            match (interface::get_cstr(arg1), interface::get_cstr(arg2)) {
                (Ok(cstr1), Ok(cstr2)) => {
                    cage.link_syscall(cstr1, cstr2)
                }
                (Err(returned_error_code), ..) |
                (.., Err(returned_error_code)) => {
                    return returned_error_code;
                }
            }

            
        }
        CHDIR_SYSCALL => {
            match interface::get_cstr(arg1) {
                Ok(cstr1) => {
                    return cage.chdir_syscall(cstr1);
                }
                Err(returned_error_code) => {
                    return returned_error_code;
                }
            }
        }
        XSTAT_SYSCALL => {
            match (interface::get_cstr(arg1), interface::get_statdatastruct(arg2)) {
                (Ok(cstr1), Ok(statdata2)) => {
                    return cage.stat_syscall(cstr1, statdata2);
                }
                (Err(returned_error_code), ..) |
                (.., Err(returned_error_code)) => {
                    return returned_error_code;
                }
            }
        }
        OPEN_SYSCALL => {
            match (interface::get_cstr(arg1), interface::get_int(arg2), interface::get_uint(arg3)) {
                (Ok(cstr1), Ok(int2), Ok(uint3)) => {
                    return cage.open_syscall(cstr1, int2, uint3);
                }
                (Err(returned_error_code), ..) |
                (.., Err(returned_error_code)) |
                (.., Err(returned_error_code), _) => {
                    return returned_error_code;
                }
            }
        }
        READ_SYSCALL => {
            match (interface::get_int(arg1), interface::get_mutcbuf(arg2), interface::get_usize(arg3)) {
                (Ok(int1), Ok(mutcbuf2), Ok(usize3)) => {
                    return cage.read_syscall(int1, mutcbuf2, usize3);
                }
                (Err(returned_error_code), ..) |
                (.., Err(returned_error_code)) |
                (.., Err(returned_error_code), _) => {
                    return returned_error_code;
                }
            }
        }
        WRITE_SYSCALL => {
            match (interface::get_int(arg1), interface::get_cbuf(arg2), interface::get_usize(arg3)) {
                (Ok(int1), Ok(cbuf2), Ok(usize3)) => {
                    return cage.write_syscall(int1, cbuf2, usize3);
                }
                (Err(returned_error_code), ..) |
                (.., Err(returned_error_code)) |
                (.., Err(returned_error_code), _) => {
                    return returned_error_code;
                }
            }
        }
        CLOSE_SYSCALL => {
            match interface::get_int(arg1) {
                Ok(int1) => {
                    return cage.close_syscall(int1);
                }
                Err(returned_error_code) => {
                    return returned_error_code;
                }
            }
        }
        CLOSE_SYSCALL => {
            cage.close_syscall(unsafe{arg1.dispatch_int})
        }
        LSEEK_SYSCALL => {
            match (interface::get_int(arg1), interface::get_isize(arg2), interface::get_int(arg3)) {
                (Ok(int1), Ok(isize2), Ok(int3)) => {
                    return cage.lseek_syscall(int1, isize2, int3);
                }
                (Err(returned_error_code), ..) |
                (.., Err(returned_error_code)) |
                (.., Err(returned_error_code), _) => {
                    return returned_error_code;
                }
            }
        }
        FXSTAT_SYSCALL => {
            match (interface::get_int(arg1), interface::get_statdatastruct(arg2)) {
                (Ok(int1), Ok(statdata2)) => {
                    return cage.fstat_syscall(int1, statdata2);                    
                }
                (Err(returned_error_code), ..) |
                (.., Err(returned_error_code)) => {
                    return returned_error_code;
                }
            }
        }
        FSTATFS_SYSCALL => {
            match (interface::get_int(arg1), interface::get_fsdatastruct(arg2)) {
                (Ok(int1), Ok(fstatdata2)) => {
                    return cage.fstatfs_syscall(int1, fstatdata2);                    
                }
                (Err(returned_error_code), ..) |
                (.., Err(returned_error_code)) => {
                    return returned_error_code;
                }
            }
        }
        FSTATFS_SYSCALL => {
            cage.fstatfs_syscall(unsafe{arg1.dispatch_int}, unsafe{&mut *arg2.dispatch_fsdatastruct})
        }
        MMAP_SYSCALL => {
            match (interface::get_mutcbuf(arg1), interface::get_usize(arg2), interface::get_int(arg3), interface::get_int(arg4), interface::get_int(arg5), interface::get_long(arg6)) {
                (Ok(mutcbuf1), Ok(usize2), Ok(int3), Ok(int4), Ok(int5), Ok(long6)) => {
                    return cage.mmap_syscall(mutcbuf1, usize2, int3, int4, int5, long6);
                } 
                (Err(returned_error_code), ..) | 
                (.., Err(returned_error_code), _) | 
                (.., Err(returned_error_code), _, _) | 
                (.., Err(returned_error_code), _, _, _) | 
                (.., Err(returned_error_code), _, _, _, _) | 
                (.., Err(returned_error_code)) => {
                    return returned_error_code;
                }
            }
        }
        MUNMAP_SYSCALL => {
            match (interface::get_mutcbuf(arg1), interface::get_usize(arg2)) {
                (Ok(mutcbuf1), Ok(usize2)) => {
                    return cage.munmap_syscall(mutcbuf1, usize2);
                }
                (Err(returned_error_code), ..) |
                (.., Err(returned_error_code)) => {
                    return returned_error_code;
                }
            }
        }
        DUP_SYSCALL => {
            match interface::get_int(arg1) {
                Ok(int1) => {
                    return cage.dup_syscall(int1, None);
                }
                Err(returned_error_code) => {
                    return returned_error_code;
                }
            }
        }
        DUP2_SYSCALL => {
            match (interface::get_int(arg1), interface::get_int(arg2)) {
                (Ok(int1), Ok(int2)) => {
                    return cage.dup2_syscall(int1, int2);
                }
                (Err(returned_error_code), ..) |
                (.., Err(returned_error_code)) => {
                    return returned_error_code;
                }
            }
        }
        STATFS_SYSCALL => {
            match (interface::get_cstr(arg1), interface::get_fsdatastruct(arg2)) {
                (Ok(cstr1), Ok(fsdata2)) => {
                    return cage.statfs_syscall(cstr1, fsdata2);                    
                }
                (Err(returned_error_code), ..) |
                (.., Err(returned_error_code)) => {
                    return returned_error_code;
                }
            }
        }
        FCNTL_SYSCALL => {
            match (interface::get_int(arg1), interface::get_int(arg2), interface::get_int(arg3)) {
                (Ok(int1), Ok(int2), Ok(int3)) => {
                    return cage.fcntl_syscall(int1, int2, int3);
                }
                (Err(returned_error_code), ..) |
                (.., Err(returned_error_code)) |
                (.., Err(returned_error_code), _) => {
                    return returned_error_code;
                }
            }
        }
        GETPPID_SYSCALL => {
            cage.getppid_syscall()
        }
        GETPID_SYSCALL => {
            cage.getpid_syscall()
        }
        EXIT_SYSCALL => {
            cage.exit_syscall()
        }
        FLOCK_SYSCALL => {
            match (interface::get_int(arg1), interface::get_int(arg2)) {
                (Ok(int1), Ok(int2)) => {
                    return cage.flock_syscall(int1, int2);
                }
                (Err(returned_error_code), ..) |
                (.., Err(returned_error_code)) => {
                    return returned_error_code;
                }
            }
        }
        FORK_SYSCALL => {
            match interface::get_ulong(arg1) {
                Ok(ulong1) => {
                    return cage.fork_syscall(ulong1);
                }
                Err(returned_error_code) => {
                    return returned_error_code;
                }
            }
        }
        EXEC_SYSCALL => {
            match interface::get_ulong(arg1) {
                Ok(ulong1) => {
                    return cage.exec_syscall(ulong1);
                }
                Err(returned_error_code) => {
                    return returned_error_code;
                }
            }
        }
        EXEC_SYSCALL => {
            cage.exec_syscall(unsafe{arg1.dispatch_ulong})
        }
        GETUID_SYSCALL => {
            cage.getuid_syscall()
        }
        GETEUID_SYSCALL => {
            cage.geteuid_syscall()
        }
        GETGID_SYSCALL => {
            cage.getgid_syscall()
        }
        GETEGID_SYSCALL => {
            cage.getegid_syscall()
        }
        PREAD_SYSCALL => {
            match (interface::get_int(arg1), interface::get_mutcbuf(arg2), interface::get_usize(arg3), interface::get_isize(arg4)) {
                (Ok(int1), Ok(mutcbuf2), Ok(usize3), Ok(isize4)) => {
                    return cage.pread_syscall(int1, mutcbuf2, usize3, isize4);
                }
                (Err(returned_error_code), ..) |
                (.., Err(returned_error_code)) |
                (.., Err(returned_error_code), _) |
                (.., Err(returned_error_code), _, _) => {
                    return returned_error_code;
                }
            }
        }
        PWRITE_SYSCALL => {
            match (interface::get_int(arg1), interface::get_mutcbuf(arg2), interface::get_usize(arg3), interface::get_isize(arg4)) {
                (Ok(int1), Ok(mutcbuf2), Ok(usize3), Ok(isize4)) => {
                    return cage.pwrite_syscall(int1, mutcbuf2, usize3, isize4);
                }
                (Err(returned_error_code), ..) |
                (.., Err(returned_error_code)) |
                (.., Err(returned_error_code), _) |
                (.., Err(returned_error_code), _, _) => {
                    return returned_error_code;
                }
            }
        }
        // CHMOD_SYSCALL => {
        //     match (interface::get_cstr(arg1), interface::get_uint(arg2)) {
        //         (Ok(cstr1), Ok(uint2)) => {
        //             return cage.chmod_syscall(cstr1, uint2);
        //         }
        //         (Err(returned_error_code), _) => {
        //             return returned_error_code;
        //         }
        //         (_, Err(returned_error_code)) => {
        //             return returned_error_code;
        //         }
        //     }
        // }
        RMDIR_SYSCALL => {
            match interface::get_cstr(arg1) {
                Ok(cstr1) => {
                    return cage.rmdir_syscall(cstr1);
                }
                Err(returned_error_code) => {
                    return returned_error_code;
                }
            }
        }
        RENAME_SYSCALL => {
            match (interface::get_cstr(arg1), interface::get_cstr(arg2)) {
                (Ok(cstr1), Ok(cstr2)) => {
                    return cage.rename_syscall(cstr1, cstr2);
                }
                (Err(returned_error_code), ..) |
                (.., Err(returned_error_code)) => {
                    return returned_error_code;
                }
            }
        }
        _ => {//unknown syscall
            -1
        }
    }
}

pub extern "C" fn lindrustinit() {
    load_fs();
    incref_root();
    let mut mutcagetable = CAGE_TABLE.write().unwrap();


    //init cage is its own parent
    let mut initcage = Cage{
        cageid: 1, cwd: interface::RustLock::new(interface::RustRfc::new(interface::RustPathBuf::from("/"))), 
        parent: 1, filedescriptortable: interface::RustLock::new(interface::RustHashMap::new())};
    initcage.load_lower_handle_stubs();
    mutcagetable.insert(1, interface::RustRfc::new(initcage));

}

pub extern "C" fn lindrustfinalize() {
    //wipe all keys from hashmap, i.e. free all cages
    let mut cagetable = CAGE_TABLE.write().unwrap();
    let drainedcages: Vec<(u64, interface::RustRfc<Cage>)> = cagetable.drain().collect();
    drop(cagetable);
    for (_cageid, cage) in drainedcages {
        cage.exit_syscall();
    }
}

#[cfg(test)]
pub mod dispatch_tests {
    use super::*;
    pub fn cagetest() {
        lindrustinit();

        {interface::RustRfc::get_mut(CAGE_TABLE.write().unwrap().get_mut(&1).unwrap()).unwrap().load_lower_handle_stubs();}
        {println!("{:?}", CAGE_TABLE.read().unwrap());};
        {println!("{:?}", FS_METADATA.read().unwrap().inodetable);};
        dispatcher(1, FORK_SYSCALL, Arg {dispatch_ulong: 2_u64}, Arg {dispatch_int: 34132}, Arg {dispatch_int: 109384}, Arg {dispatch_int: -12341}, Arg {dispatch_int: -12341}, Arg {dispatch_int: 0});
        {println!("{:?}", CAGE_TABLE.read().unwrap());};
        dispatcher(2, EXEC_SYSCALL, Arg {dispatch_ulong: 7_u64}, Arg {dispatch_int: 34132}, Arg {dispatch_int: 109384}, Arg {dispatch_int: -12341}, Arg {dispatch_int: -12341}, Arg {dispatch_int: 0});
        {println!("{:?}", CAGE_TABLE.read().unwrap());};
        dispatcher(7, EXIT_SYSCALL, Arg {dispatch_ulong: 61_u64}, Arg {dispatch_int: 33987}, Arg {dispatch_int: 123452}, Arg {dispatch_int: -98493}, Arg {dispatch_int: -1}, Arg {dispatch_int: 0});
        {println!("{:?}", CAGE_TABLE.read().unwrap());};

        lindrustfinalize();
    }
}
