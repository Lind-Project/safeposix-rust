// System related system calls

use crate::interface;
use crate::safeposix::cage::{CAGE_TABLE, Cage, FileDescriptor::*};

use super::sys_constants::*;

impl Cage {
  pub fn fork_syscall(&self, child_cageid: u64) -> i32 {
    CAGE_TABLE.write().unwrap().insert(child_cageid, interface::RustRfc::new(Cage {cageid: child_cageid, cwd: self.cwd.clone(), parent: self.cageid, filedescriptortable: interface::RustLock::new(self.filedescriptortable.read().unwrap().clone())}));
    0
  }
  pub fn exec_syscall(&self, child_cageid: u64) -> i32 {
    { CAGE_TABLE.write().unwrap().remove(&self.cageid).unwrap(); }
    self.filedescriptortable.write().unwrap().retain(|&_, v| !match &**v.read().unwrap() {
      File(_f) => true,//f.flags & CLOEXEC,
      Stream(_s) => true,//s.flags & CLOEXEC,
      Socket(_s) => true,//s.flags & CLOEXEC,
      Pipe(_p) => true,//p.flags & CLOEXEC
    });
    let newcage = Cage {cageid: child_cageid, cwd: self.cwd.clone(), parent: self.parent, filedescriptortable: interface::RustLock::new(self.filedescriptortable.read().unwrap().clone())};
    //wasteful clone of fdtable, but mutability constraints exist

    {CAGE_TABLE.write().unwrap().insert(child_cageid, interface::RustRfc::new(newcage))};
    0
  }
  pub fn exit_syscall(&self) -> i32 {
    CAGE_TABLE.write().unwrap().remove(&self.cageid);
    //fdtable will be dropped at end of dispatcher scope because of Arc
    0
  }
  pub fn getpid_syscall(&self) -> i32 {
    self.cageid as i32 //not sure if this is quite what we want but it's easy enough to change later
  }

  pub fn getppid_syscall(&self) -> i32 {
    self.parent as i32 // mimicing the call above -- easy to change later if necessary
  }
  
  pub fn getgid_syscall(&self) -> i32 {
    DEFAULT_GID as i32 //Lind is only run in one group so a default value is returned
  }
  
  pub fn getegid_syscall(&self) -> i32 {
    DEFAULT_GID as i32 //Lind is only run in one group so a default value is returned
  }
  
  pub fn getuid_syscall(&self) -> i32 {
    DEFAULT_UID as i32 //Lind is only run in one group so a default value is returned
  }
  
  pub fn geteuid_syscall(&self) -> i32 {
    DEFAULT_UID as i32 //Lind is only run in one group so a default value is returned
  }
  
  pub fn getrlimit(&self, res_type: void, rlimit: Rlimit) -> i32 {
    match res_type{
      RLIMIT_NOFILE => {
          rlimit.rlim_cur = NOFILE_CUR,
          rlimit.rlim_max = NOFILE_MAX,
        }
        0
      },
      RLIMIT_STACK => {
        rlimit.rlim_cur = STACK_CUR,
        rlimit.rlim_max = STACK_MAX,
        0
      },
      _ => -1,
      }
    }
  }
  
  pub fn setrlimit(&self, res_type: void, limitValue: u64) -> i32 {
    match res_type{
      RLIMIT_NOFILE => {
          if (&NOFILE_CUR > NOFILE_MAX) -1
          else  0 
          //FIXME: not implemented yet to update value in program
        },
      _ => -1,
      }
    }
  }

}
