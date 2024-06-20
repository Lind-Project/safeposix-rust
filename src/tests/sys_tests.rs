#![allow(dead_code)] //suppress warning for these functions not being used in targets other than the tests

#[allow(unused_parens)]
#[cfg(test)] 
pub mod sys_tests {
    use super::super::*;
    use crate::interface;
    use crate::safeposix::syscalls::sys_calls::*;
    use crate::safeposix::{cage::*, dispatcher::*, filesystem};
    use libc::c_void;
    use std::fs::OpenOptions;
    use std::os::unix::fs::PermissionsExt; 

    pub fn test_fs() {
        ut_lind_getpid(); 
        ut_lind_getppid(); 
        ut_lind_getpid(); 
    } 

    pub fn ut_lind_getpid() {
        lindrustinit(0); 
        let cage =  interface::cagetable_getref(1); 
        assert_eq!(cage.getpid_syscall(),1); 
        lindrustfinalize();
    } 

    pub fn ut_lind_getppid() {
        lindrustinit(0); 
        let cage = interface::cagetable_getref(1); 
        cage.fork_syscall(2);
        let cage2 = interface::cagetable_getref(2);
        assert_eq!(cage2.getppid_syscall(),1); 
        lindrustfinalize(); 
        
    } 

} 

