// File system related constants
#![allow(dead_code)]

use crate::interface;


// Define constants using static or const
// Imported into fs_calls file

pub const STARTINGFD: i32 = 0;
pub const MAXFD: i32 = 1024;

pub const ROOTDIRECTORYINODE: usize = 1;
pub const STREAMINODE: usize = 2;

pub const F_OK: u32 = 0;
pub const X_OK: u32 = 1;
pub const W_OK: u32 = 2;
pub const R_OK: u32 = 4;

pub const O_RDONLY: i32 = 0o0;
pub const O_WRONLY: i32 = 0o1;
pub const O_RDWR: i32 = 0o2;
pub const O_RDWRFLAGS: i32 = 0o3;

pub const O_CREAT: i32 = 0o100;
pub const O_EXCL: i32 = 0o200;
pub const O_NOCTTY: i32 = 0o400;
pub const O_TRUNC: i32 = 0o1000;
pub const O_APPEND: i32 = 0o2000;
pub const O_NONBLOCK: i32 = 0o4000;
// O_NDELAY=O_NONBLOCK
pub const O_SYNC: i32 = 0o10000;
// O_FSYNC=O_SYNC
pub const O_ASYNC: i32 = 0o20000;
pub const O_CLOEXEC: i32 = 0o2000000;

pub const DEFAULTTIME: u64 = 1323630836;

pub const DEFAULT_UID: usize = 1000;
pub const DEFAULT_GID: usize = 1000;

//Standard flag combinations
pub const S_IRWXA: u32 = 0o777;
pub const S_IRWXU: u32 = 0o700;
pub const S_IRUSR: u32 = 0o400;
pub const S_IWUSR: u32 = 0o200;
pub const S_IXUSR: u32 = 0o100;
pub const S_IRWXG: u32 = 0o070;
pub const S_IRGRP: u32 = 0o040;
pub const S_IWGRP: u32 = 0o020;
pub const S_IXGRP: u32 = 0o010;
pub const S_IRWXO: u32 = 0o007;
pub const S_IROTH: u32 = 0o004;
pub const S_IWOTH: u32 = 0o002;
pub const S_IXOTH: u32 = 0o001;

//File types for open/stat etc.
pub const S_IFBLK: i32 = 0o60000;
pub const S_IFCHR: i32 = 0o20000;
pub const S_IFDIR: i32 = 0o40000;
pub const S_IFIFO: i32 = 0o10000;
pub const S_IFLNK: i32 = 0o120000;
pub const S_IFREG: i32 = 0o100000;
pub const S_IFSOCK: i32 = 0o140000;
pub const S_FILETYPEFLAGS: i32 = 0o170000;

//device info for char files
#[derive(PartialEq,Debug)]
pub struct DevNo {
  pub major: u64,
  pub minor: u64
}
pub const NULLDEVNO: DevNo = DevNo {major: 1, minor: 3};
pub const ZERODEVNO: DevNo = DevNo {major: 1, minor: 5};
pub const RANDOMDEVNO: DevNo = DevNo {major: 1, minor: 8};
pub const URANDOMDEVNO: DevNo = DevNo {major: 1, minor: 9};

pub const FILEDATAPREFIX: &str = "linddata.";

pub struct StatData {
  // pub dev_id: usize,
  // pub inode: usize,  //Not sure if this is necessary, and if it is, then it needs to have a use line at the top of this file
  pub mode: u32,
  pub linkcount: usize,
  pub refcount: usize,
  pub uid: usize,
  pub gid: usize,
  // pub dev: usize, //should this be diff type?
  pub size: usize,
  // pub blksize: usize,
  // pub blocks: usize,
  pub atime: u64,
  // pub atimens: usize,
  pub mtime: u64,
  // pub mtimens: usize,
  pub ctime: u64,
  // pub ctimens: usize
}

pub fn is_reg(mode: u32) -> bool {
  (mode as i32 & S_FILETYPEFLAGS) == S_IFREG
}

pub fn is_chr(mode: u32) -> bool {
  (mode as i32 & S_FILETYPEFLAGS) == S_IFCHR
}
