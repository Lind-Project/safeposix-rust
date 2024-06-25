#![allow(dead_code)] //suppress warning for these functions not being used in targets other than the tests

#[allow(unused_parens)]
#[cfg(test)] 
pub mod test_sys {
    use super::super::*;
    use crate::interface;
    use crate::safeposix::syscalls::sys_calls::*;

    pub fn test_sys() {
        ut_lind_getpid(); 
        ut_lind_getppid(); 
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

    pub fn ut_lind_getuid() {
        lindrustinit(0);
        // The first call to geteuid always returns -1
        assert_eq!(cage.getuid_syscall(),-1);
        // Subsequent calls return the default value
        assert_eq!(cage.getuid_syscall(),DEFAULT_UID);
        lindrustfinalize()
    }

    pub fn ut_lind_geteuid() {
        lindrustinit(0);
        let cage = interface::cagetable_getref(1);
        // The first call to geteuid always returns -1
        assert_eq!(cage.geteuid_syscall(),-1);
        // Subsequent calls return the default value
        assert_eq!(cage.geteuid_syscall(),DEFAULT_UID);
        lindrustfinalize()
    }

    pub fn ut_lind_getgid() {
        lindrustinit(0);
        let cage = interface::cagetable_getref(1);
        // The first call to geteuid always returns -1
        assert_eq!(cage.getgid_syscall(),-1);
        // Subsequent calls return the default value
        assert_eq!(cage.getgid_syscall(),DEFAULT_GID);
        lindrustfinalize()
    } 

    pub fn ut_lind_getegid() {
        lindrustinit(0);
        let cage = interface::cagetable_getref(1);
        // The first call to geteuid always returns -1
        assert_eq!(cage.getegid_syscall(),-1);
        // Subsequent calls return the default value
        assert_eq!(cage.getegid_syscall(),DEFAULT_GID);
        lindrustfinalize()
    } 

} 

