#![feature(once_cell)]
#![feature(rustc_private)] //for private crate imports for tests
#![feature(vec_into_raw_parts)]
#![allow(unused)]

use std::env;
use std::fs::File;
use std::io::Read;

mod interface;
mod safeposix;
use safeposix::{cage::*, filesystem};
//assume deserialization

fn update_dir_into_lind(cage: &Cage, hostfilepath: interface::RustPathBuf, lindfilepath: String) {
    if !hostfilepath.exists() {
        return; //perhaps error message later
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
    if hostfilepath.exists() || hostfilepath.is_file() {
        return; //error message
    }
    let fmetadata = hostfilepath.metadata().unwrap();
    //let host_mtime = fmetadata.modified();
    let host_size = fmetadata.len();
    let mut lindstat_res: StatData;
    let stat_us = cage.stat_syscall(format!("{:?}", hostfilepath).as_str(), &mut lindstat_res);
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
        cp_into_lind(hostfilepath, lindfilepath);
    } else {
        println!("Same files on host and lind--{} and {:?}, skipping", lindfilepath, hostfilepath);
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let utilcage = Cage{cageid: 0,
                        cwd: interface::RustLock::new(interface::RustRfc::new(interface::RustPathBuf::from("/"))),
                        parent: 0, 
                        filedescriptortable: interface::RustLock::new(interface::RustHashMap::new())};

    if args.len() == 1 {
        return; //print usage
    }
    let command = args[1].as_str();

    match command {
        "help" | "usage" => {
        }
        "cp" => {
        }
        "update" => {
        }
        "find" => {
        }
        "format" => {
        }
        "deltree" => {
        }
        "rm" => {
        }
        "mkdir" => {
        }
        "rmdir" => {
        }
        _ => {
            return; //error message later
        }
    }
}
