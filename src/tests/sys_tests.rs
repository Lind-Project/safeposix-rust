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
        ut_lind_getegid();
        ut_lind_getuid();
        ut_lind_geteuid();
        ut_lind_getgid();
        ut_lind_fork();
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

    pub fn ut_lind_fork() {
        // Since the fork syscall is heavily tested in relation to other syscalls
        // we only perform simple checks for testing the sanity of the fork syscall
        lindrustinit(0); 
        let cage = interface::cagetable_getref(1); 
        // Spawn a new child object using the fork syscall 
        cage.fork_syscall(2); 
        // Search for the new cage object with cage_id = 2
        let child_cage = interface::cagetable_getref(2); 
        // Assert the parent value is the the id of the first cage object
        assert_eq!(child_cage.getpid_syscall(),1);
        // Assert that the cage id of the child is the value passed in the original fork syscall
        assert_eq!(child_cage.getuid(),2);
        // Assert that the cwd is the same as the parent cage object
        assert_eq!(child_cage.cwd.read(),cage.cwd.read())
    }

} 

