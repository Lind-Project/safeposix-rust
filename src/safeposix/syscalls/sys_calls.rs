// System related system calls

use crate::interface;
use crate::safeposix::cage::{CAGE_TABLE, Cage, FileDescriptor::*};

use super::sys_constants::*;

impl Cage {
  pub fn fork_syscall(&self, child_cageid: u64) {
    CAGE_TABLE.write().unwrap().insert(child_cageid, interface::RustRfc::new(Cage {cageid: child_cageid, cwd: self.cwd.clone(), parent: self.cageid, filedescriptortable: interface::RustLock::new(self.filedescriptortable.read().unwrap().clone())}));
  }
  pub fn exec_syscall(&self, child_cageid: u64) {
    { CAGE_TABLE.write().unwrap().remove(&self.cageid).unwrap(); }
    self.filedescriptortable.write().unwrap().retain(|&_, v| !match &**v.read().unwrap() {
      File(f) => true,//f.flags & CLOEXEC,
      Stream(s) => true,//s.flags & CLOEXEC,
      Socket(s) => true,//s.flags & CLOEXEC,
      Pipe(p) => true,//p.flags & CLOEXEC
    });
    let newcage = Cage {cageid: child_cageid, cwd: self.cwd.clone(), parent: self.parent, filedescriptortable: interface::RustLock::new(self.filedescriptortable.read().unwrap().clone())};
    //wasteful clone of fdtable, but mutability constraints exist

    {CAGE_TABLE.write().unwrap().insert(child_cageid, interface::RustRfc::new(newcage))};
  }
}
