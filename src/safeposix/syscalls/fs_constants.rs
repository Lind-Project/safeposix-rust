// File system related constants
#![allow(dead_code)]

use crate::interface;


// Define constants using static or const
// Imported into fs_calls file

pub const STARTINGFD: usize = 0;
pub const MAXFD: usize = 1024;

pub const ROOTDIRECTORYINODE: usize = 1;
pub const STREAMINODE: usize = 2;

pub const O_RDONLY: usize = 0o0;
pub const O_WRONLY: usize = 0o1;
pub const O_RDWR: usize = 0o2;

pub const DEFAULTTIME: u64 = 1323630836;

pub const DEFAULT_UID:usize = 1000;
pub const DEFAULT_GID:usize = 1000;

//Standard flag combinations
pub const S_IRWXA: usize = 0o777;
pub const S_IRWXU: usize = 0o700;
pub const S_IRUSR: usize = 0o400;
pub const S_IWUSR: usize = 0o200;
pub const S_IXUSR: usize = 0o100;
pub const S_IRWXG: usize = 0o070;
pub const S_IRGRP: usize = 0o040;
pub const S_IWGRP: usize = 0o020;
pub const S_IXGRP: usize = 0o010;
pub const S_IRWXO: usize = 0o007;
pub const S_IROTH: usize = 0o004;
pub const S_IWOTH: usize = 0o002;
pub const S_IXOTH: usize = 0o001;

//File types for open/stat etc.
pub const S_IFBLK: usize = 24576;
pub const S_IFCHR: usize = 8192;
pub const S_IFDIR: usize = 16384;
pub const S_IFIFO: usize = 4096;
pub const S_IFLNK: usize = 40960;
pub const S_IFREG: usize = 32768;
pub const S_IFSOCK: usize = 49152;

//device info for char files
#[derive(PartialEq)]
pub struct DevNo {
  pub major: u64,
  pub minor: u64
}
pub const NULLDEVNO: DevNo = DevNo {major: 1, minor: 3};
pub const ZERODEVNO: DevNo = DevNo {major: 1, minor: 5};
pub const RANDOMDEVNO: DevNo = DevNo {major: 1, minor: 8};
pub const URANDOMDEVNO: DevNo = DevNo {major: 1, minor: 9};
