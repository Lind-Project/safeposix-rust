#![feature(once_cell)]
#![feature(rustc_private)] //for private crate imports for tests
#![feature(vec_into_raw_parts)]
#![allow(unused)]

use std::env;
use std::fs::File;
use std::io::{Read, prelude};

mod interface;
mod safeposix;
use safeposix::{cage::*, filesystem::*};
//assume deserialization

fn update_dir_into_lind(cage: &Cage, hostfilepath: interface::RustPathBuf, lindfilepath: String) {
    if !hostfilepath.exists() {
        if let Ok(_) = hostfilepath.read_link() {
            return; //print error todo, ignore broken symlink on host fs
        } //if read_link succeeds it's a symlink
    } else {
        panic!("Cannot locate file on host fs: {:?}", hostfilepath);
    }

    if hostfilepath.is_file() {
        update_into_lind(cage, hostfilepath, lindfilepath);
    } else {
        let children = hostfilepath.read_dir().unwrap();
        for wrappedchild in children {
            let child = wrappedchild.unwrap();
            update_dir_into_lind(cage, child.path(), format!("{}/{:?}", lindfilepath, child.file_name()));
        }
    }
}

fn update_into_lind(cage: &Cage, hostfilepath: interface::RustPathBuf, lindfilepath: String) {
    if !hostfilepath.exists() || !hostfilepath.is_file() {
        return; //error message
    }
    let fmetadata = hostfilepath.metadata().unwrap();
    //let host_mtime = fmetadata.modified();
    let host_size = fmetadata.len();
    let mut lindstat_res: StatData;
    let stat_us = cage.stat_syscall(lindfilepath.as_str(), &mut lindstat_res);
    let lind_exists;
    let lind_isfile;
    let lind_size;
    if stat_us >= 0 {
        lind_exists = false;
        lind_isfile = false;
        lind_size = 0;
    } else {
        lind_exists = true;
        lind_isfile = is_reg(lindstat_res.st_mode);
        lind_size = lindstat_res.st_size;
    }

    if lind_exists && !lind_isfile {
        return; //error message later
    }

    let samefile = if host_size as usize == lind_size {
        let mut hostslice = vec![0u8; lind_size];
        let mut lindslice = vec![0u8; lind_size];
        let hostfile = File::open(hostfilepath).unwrap();
        hostfile.read(hostslice.as_mut_slice());
        let lindfd = cage.open_syscall(lindfilepath.as_str(), O_RDONLY, 0);
        cage.read_syscall(lindfd, lindslice.as_mut_ptr(), lind_size);
        cage.close_syscall(lindfd);
        hostslice == lindslice
    } else {
        false
    };

    if samefile {
        if lind_exists {
            cage.unlink_syscall(lindfilepath.as_str());
        }
        cp_into_lind(cage, hostfilepath, lindfilepath, true);
    } else {
        println!("Same files on host and lind--{} and {:?}, skipping", lindfilepath, hostfilepath);
    }
}

fn cp_dir_into_lind(cage: &Cage, hostfilepath: interface::RustPathBuf, lindfilepath: String, create_missing_dirs: bool) {
    if !hostfilepath.exists() {
        if let Ok(_) = hostfilepath.read_link() {
            return; //print error todo, ignore broken symlink on host fs
        } //if read_link succeeds it's a symlink
    } else {
        panic!("Cannot locate file on host fs: {:?}", hostfilepath);
    }

    if hostfilepath.is_file() {
        cp_into_lind(cage, hostfilepath, lindfilepath, create_missing_dirs);
    } else if hostfilepath.is_dir() {
        let children = hostfilepath.read_dir().unwrap();
        for wrappedchild in children {
            let child = wrappedchild.unwrap();
            cp_dir_into_lind(cage, child.path(), format!("{}/{:?}", lindfilepath, child.file_name()), create_missing_dirs);
        }
    }
}

fn cp_into_lind(cage: &Cage, hostfilepath: interface::RustPathBuf, lindfilepath: String, create_missing_dirs: bool) {
    if !hostfilepath.exists() {
        panic!("Cannot locate file on host fs: {:?}", hostfilepath);
    }
    if !hostfilepath.is_file() {
        panic!("Cannot locate file on host fs: {:?}", hostfilepath);
    }

    let lindtruepath = normpath(convpath(lindfilepath.as_str()), cage);
    let mutmetadata = FS_METADATA.write().unwrap();
    let mut ancestor = interface::RustPathBuf::from("/");
    for component in lindtruepath.parent().unwrap().components() {
        ancestor.push(component);
        let mut lindstat_res: StatData;
        let stat_us = cage.stat_syscall(format!("{:?}", ancestor).as_str(), &mut lindstat_res);
        if stat_us == 0 {continue;}
        if stat_us != -(Errno::ENOENT as i32) {
            panic!("Fatal error in trying to get lind file path");
        }
        if create_missing_dirs && is_dir(lindstat_res.st_mode) {
            cage.mkdir_syscall(format!("{:?}", ancestor).as_str(), S_IRWXA); //let's not mirror stat data
        } else {
            panic!("Lind fs path does not exist but should not be created {:?}", ancestor);
        }
    }

    let host_fileobj = File::open(hostfilepath).unwrap();
    let mut filecontents = Vec::new();
    host_fileobj.read_to_end(&mut filecontents);

    let lindfd = cage.open_syscall(format!("{:?}", lindtruepath).as_str(), O_CREAT | O_TRUNC | O_WRONLY, S_IRWXA);
    let veclen = filecontents.len();
    let fileslice = filecontents.as_slice();
    let writtenlen = cage.write_syscall(lindfd, fileslice.as_ptr(), veclen);
    assert_eq!(veclen as i32, writtenlen);

    let mut lindstat_res: StatData;
    let stat_us = cage.fstat_syscall(lindfd, &mut lindstat_res);
    let inode = lindstat_res.st_ino;

    cage.close_syscall(lindfd);

    println!("Copied {:?} as {} ({})", hostfilepath, lindfilepath, inode);
}

fn main() {
    let args = env::args();
    let utilcage = Cage{cageid: 0,
                        cwd: interface::RustLock::new(interface::RustRfc::new(interface::RustPathBuf::from("/"))),
                        parent: 0, 
                        filedescriptortable: interface::RustLock::new(interface::RustHashMap::new())};

    args.next();//first arg is executable, we don't care
    let command = if let Some(cmd) = args.next() {
        cmd.as_str()
    } else {
        return; //print usage
    };

    match command {
        "help" | "usage" => {
        }
        "cp" => {
            let _source = args.next().unwrap();
            let _dest = args.next().unwrap();
        }
        "update" => {
            let _source = args.next().unwrap();
            let _dest = args.next().unwrap();
        }
        "ls" => {
            let _file = args.next().unwrap();
        }
        "tree" => {
            let _rootdir = if let Some(dirstr) = args.next() {
                dirstr.as_str()
            } else {"/"};
        }
        "format" => {
            FilesystemMetadata::blank_fs_init();
        }
        "deltree" => {
            let _rootdir = args.next().unwrap();
        }
        "rm" => {
            for _file in args {
            }
        }
        "mkdir" => {
            for _dir in args {
            }
        }
        "rmdir" => {
            for _dir in args {
            }
        }
        _ => {
            return; //error message later
        }
    }
}
