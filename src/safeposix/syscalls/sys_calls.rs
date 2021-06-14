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
}
