#![feature(once_cell)]
#![feature(rustc_private)] //for private crate imports for tests
#![feature(vec_into_raw_parts)]
#![allow(unused)]

/// Author: Jonathan Singer
///
/// This file provides a command line interface for interacting in certain ways with the lind file
/// system from the host, such as copying files from the host into lind, removing files and
/// directories, and listing files in the lind fs, and more
///
/// This interface should be sufficient for anything we'd need to do between lind and the host

use std::env;
use std::fs::File;
use std::io::{Read, prelude};
use std::iter::repeat;

mod interface;
mod safeposix;
use safeposix::{cage::*, filesystem::*, dispatcher::{lindrustfinalize, lindrustinit}};
//assume deserialization

fn update_dir_into_lind(cage: &Cage, hostfilepath: &interface::RustPath, lindfilepath: &str) {
    if hostfilepath.exists() {
        if let Ok(_) = hostfilepath.read_link() {
            println!("Ignore broken symlink at {:?} on host fs", hostfilepath);
            return;
        } //if read_link succeeds it's a symlink
    } else {
        eprintln!("Cannot locate file on host fs: {:?}", hostfilepath);
        return;
    }

    //update directly if not a directory on the host, otherwise recursively handle children
    if hostfilepath.is_file() {
        update_into_lind(cage, hostfilepath, lindfilepath);
    } else {
        let children = hostfilepath.read_dir().unwrap();
        for wrappedchild in children {
            let child = wrappedchild.unwrap();
            update_dir_into_lind(cage, child.path().as_path(), format!("{}/{}", lindfilepath, child.file_name().to_str().unwrap()).as_str());
        }
    }
}

fn update_into_lind(cage: &Cage, hostfilepath: &interface::RustPath, lindfilepath: &str) {
    if !hostfilepath.exists() || !hostfilepath.is_file() {
        println!("{:?} does not exist or is not a regular file, skipping", hostfilepath);
        return;
    }
    let fmetadata = hostfilepath.metadata().unwrap();

    let host_size = fmetadata.len();
    let mut lindstat_res: StatData = StatData::default();
    let stat_us = cage.stat_syscall(lindfilepath, &mut lindstat_res);

    let lind_exists;
    let lind_isfile;
    let lind_size;
    if stat_us < 0 {
        lind_exists = false;
        lind_isfile = false;
        lind_size = 0;
    } else {
        lind_exists = true;
        lind_isfile = is_reg(lindstat_res.st_mode);
        lind_size = lindstat_res.st_size;
    }

    if lind_exists && !lind_isfile {
        println!("{:?} on lind file system is not a regular file, skipping", hostfilepath);
        return;
    }

    //compare files to tell whether they are identical
    let samefile = if host_size as usize == lind_size {
        let mut hostslice = vec![0u8; lind_size];
        let mut lindslice = vec![0u8; lind_size];
        let mut hostfile = File::open(hostfilepath).unwrap();
        hostfile.read(hostslice.as_mut_slice());
        let lindfd = cage.open_syscall(lindfilepath, O_RDONLY, 0);
        cage.read_syscall(lindfd, lindslice.as_mut_ptr(), lind_size);
        cage.close_syscall(lindfd);
        hostslice == lindslice
    } else {
        false
    };

    //if they are not the same file, remove the lind file and replace it with the host file
    if !samefile {
        if lind_exists {
            cage.unlink_syscall(lindfilepath);
            println!("removing {} on lind file system", lindfilepath);
        }
        cp_into_lind(cage, hostfilepath, lindfilepath, true);
    } else {
        println!("Same files on host and lind--{} and {:?}, skipping", lindfilepath, hostfilepath);
    }
}

fn cp_dir_into_lind(cage: &Cage, hostfilepath: &interface::RustPath, lindfilepath: &str, create_missing_dirs: bool) {
    if hostfilepath.exists() {
        if let Ok(_) = hostfilepath.read_link() {
            println!("Ignore broken symlink at {:?} on host fs", hostfilepath);
            return;
        } //if read_link succeeds it's a symlink
    } else {
        eprintln!("Cannot locate file on host fs: {:?}", hostfilepath);
        return
    }

    //update directly if not a directory on the host, otherwise recursively handle children
    if hostfilepath.is_file() {
        cp_into_lind(cage, hostfilepath, lindfilepath, create_missing_dirs);
    } else if hostfilepath.is_dir() {
        let children = hostfilepath.read_dir().unwrap();
        for wrappedchild in children {
            let child = wrappedchild.unwrap();
            cp_dir_into_lind(cage, child.path().as_path(), format!("{}/{}", lindfilepath, child.file_name().to_str().unwrap()).as_str(), create_missing_dirs);
        }
    }
}

fn cp_into_lind(cage: &Cage, hostfilepath: &interface::RustPath, lindfilepath: &str, create_missing_dirs: bool) {
    if !hostfilepath.exists() {
        eprintln!("Cannot locate file on host fs: {:?}", hostfilepath);
        return
    }
    if !hostfilepath.is_file() {
        eprintln!("File is not a regular file on host fs: {:?}", hostfilepath);
        return
    }

    let lindtruepath = normpath(convpath(lindfilepath), cage);

    //if a directory in the lindfilepath does not exist in the lind file system, create it!
    let mut ancestor = interface::RustPathBuf::from("/");
    for component in lindtruepath.parent().unwrap().components() {
        ancestor.push(component);
        let mut lindstat_res: StatData = StatData::default();

        //check whether file exists
        let stat_us = cage.stat_syscall(ancestor.to_str().unwrap(), &mut lindstat_res);
        if stat_us == 0 {
            if !is_dir(lindstat_res.st_mode) {
                eprintln!("Fatal error in trying to create child of non-directory file");
                return;
            }
            continue;
        }
        if stat_us != -(Errno::ENOENT as i32) {
            eprintln!("Fatal error in trying to get lind file path");
            return;
        }

        //check whether we are supposed to create missing directories, and whether we'd be
        //clobbering anything to do so (if so error out)
        if create_missing_dirs {
            if cage.mkdir_syscall(ancestor.to_str().unwrap(), S_IRWXA) != 0 { //let's not mirror stat data
                eprintln!("Lind fs path does not exist but should not be created (is rooted at non-directory) {:?}", ancestor);
                return;
            }
        } else {
            eprintln!("Lind fs path does not exist but should not be created {:?}", ancestor);
            return;
        }
    }

    //copy file contents into lind file system
    let mut host_fileobj = File::open(hostfilepath).unwrap();
    let mut filecontents: Vec<u8> = Vec::new();
    host_fileobj.read_to_end(&mut filecontents);

    let lindfd = cage.open_syscall(lindtruepath.to_str().unwrap(), O_CREAT | O_TRUNC | O_WRONLY, S_IRWXA);
    assert!(lindfd >= 0);
    let veclen = filecontents.len();
    let fileslice = filecontents.as_slice();
    let writtenlen = cage.write_syscall(lindfd, fileslice.as_ptr(), veclen);

    //confirm that write succeeded
    assert_eq!(veclen as i32, writtenlen);

    //get diagnostic data to print
    let mut lindstat_res: StatData = StatData::default();
    let stat_us = cage.fstat_syscall(lindfd, &mut lindstat_res);
    let inode = lindstat_res.st_ino;

    cage.close_syscall(lindfd);

    println!("Copied {:?} as {} ({})", hostfilepath, lindfilepath, inode);
}

fn visit_children(cage: &Cage, path: String, arg: Option<usize>, visitor: fn(&Cage, String, bool, Option<usize>)) {
    let direntvec = cage.getdents_syscall(path, 1000000);//arbitrarily large numbeer used here because we don't need to pack
    for (_, name) in direntvec {
        let childpath = [path, name].concat();
        let mut lindstat_res: StatData = StatData::default();
        let stat_us = cage.stat_syscall(childpath.as_str(), &mut lindstat_res);
        visitor(cage, childpath, (stat_us & S_IFDIR) == S_IFDIR, arg);
    }
}

fn lind_deltree(cage: &Cage, path: String) {
    let mut lindstat_res: StatData = StatData::default();
    let stat_us = cage.stat_syscall(path.as_str(), &mut lindstat_res);
    if !is_dir(lindstat_res.st_mode) {
        eprintln!("Deltree must be run on a directory!");
        return;
    }

    visit_children(cage, path, None, |childcage, childpath, isdir, _| {
        if isdir {
            lind_deltree(childcage, childpath);
        } else {
            childcage.unlink_syscall(childpath.as_str());
        }
    });
    cage.rmdir_syscall(path.as_str());
}

fn lind_tree(cage: &Cage, path: String, indentlevel: usize) {
    let mut lindstat_res: StatData = StatData::default();
    let stat_us = cage.stat_syscall(path.as_str(), &mut lindstat_res);
    if !is_dir(lindstat_res.st_mode) {
        eprintln!("Tree must be run on a directory!");
        return;
    }

    visit_children(cage, path, Some(indentlevel), |childcage, childpath, isdir, childindentlevelopt| {
        let childindentlevel = childindentlevelopt.unwrap();
        print!("{}", "|   ".repeat(childindentlevel));
        if childindentlevel > 0 {
            print!("{}", "|---");
        }
        println!("{}", childpath);
        if isdir {
            lind_tree(childcage, childpath, childindentlevel + 1);
        }
    });
}

fn lind_ls(cage: &Cage, path: String) {
    let mut lindstat_res: StatData = StatData::default();
    let stat_us = cage.stat_syscall(path.as_str(), &mut lindstat_res);

    if is_dir(lindstat_res.st_mode) {
        visit_children(cage, path, None, |_childcage, childpath, isdir, _| {
            if isdir {print!("{}/ ", childpath);}
            else {print!("{} ", childpath);}
        });
    } else {
        print!("{} ", path);
    }
    println!();
}

fn print_usage() {
    println!("
Usage: lind_fs_utils [commandname] [arguments...]

Where commandname is one of the following:

cp [hostsource] [linddest]      : Copies files from the host file system into the lind filesystem.
                                  For example, cp bar/etc/passwd /etc/passwd will copy the
                                  former file in the host file system to the latter in lind's fs.
                                  Directories are handled recursively, cp bar/etc /etc/ will make a
                                  directory at /etc in the lind fs, and then populate it with all
                                  of the files in the root fs.
deltree [linddir]               : Delete a directory on the lind file system and all it contains
format                          : Make a new blank fs, removing the current one
help                            : Print this message
ls [lindpath]                   : List the contents of a lind file system directory
mkdir [linddir1...]             : Create a lind file system directory (for each arg)
rm [lindfile1...]               : Delete a file on the lind file system
rmdir [linddir1...]             : Delete a directory on the lind file system
tree [startlindpath]            : Print the lindfs file tree starting at the specified directory
                                  Assumes root directory if no starting path is specified.
update [hostsource] [linddest]  : Copies files from the host file system into the lind filesystem.
                                  Will not copy files if the host and lind files are identical.
                                  For example, update bar/etc/passwd /etc/passwd will copy the
                                  former file in the host file system to the latter in lind's fs if
                                  the latter does not exist or is not identical to the former.
                                  Directories are handled recursively, cp bar/etc /etc/ will make a
                                  directory at /etc in the lind fs, and then populate it with all
                                  of the files in the root fs, with identical files being skipped.
");
}

fn main() {
    lindrustinit();
    let mut args = env::args();
    let utilcage = Cage{cageid: 0,
                        cwd: interface::RustLock::new(interface::RustRfc::new(interface::RustPathBuf::from("/"))),
                        parent: 0, 
                        filedescriptortable: interface::RustLock::new(interface::RustHashMap::new())};

    args.next();//first arg is executable, we don't care
    let command = if let Some(cmd) = args.next() {
        cmd
    } else {
        print_usage();
        return; //print usage
    };

    match command.as_str() {
        "help" | "usage" => {
            print_usage();
        }

        "cp" => {
            let source = args.next().expect("cp needs 2 arguments");
            let dest = args.next().expect("cp needs 2 arguments");
            args.next().and_then::<String, fn(String) -> Option<String>>(|_| panic!("cp cannot take more than 2 arguments"));
            cp_dir_into_lind(&utilcage, interface::RustPath::new(&source), dest.as_str(), true);
        }

        "update" => {
            let source = args.next().expect("update needs 2 arguments");
            let dest = args.next().expect("update needs 2 arguments");
            args.next().and_then::<String, fn(String) -> Option<String>>(|_| panic!("update cannot take more than 2 arguments"));
            update_dir_into_lind(&utilcage, interface::RustPath::new(&source), dest.as_str());
        }

        "ls" => {
            let file = args.next().expect("ls needs 1 argument");
            args.next().and_then::<String, fn(String) -> Option<String>>(|_| panic!("ls cannot take more than 1 argument"));
            lind_ls(&utilcage, file);
        }

        "tree" => {
            let rootdir = if let Some(dirstr) = args.next() {
                dirstr
            } else {"/".to_owned()};
            println!("{}", rootdir);
            lind_tree(&utilcage, rootdir, 0);
        }

        "format" => {
            let mut metadata = FS_METADATA.write().unwrap();
            *metadata = FilesystemMetadata::blank_fs_init();
            drop(metadata);
            load_fs_special_files(&utilcage);

            let metadata2 = FS_METADATA.read().unwrap();
            persist_metadata(&*metadata2);
            return;
        }

        "deltree" => {
            let rootdir = args.next().expect("deltree needs 1 argument");
            args.next().and_then::<String, fn(String) -> Option<String>>(|_| panic!("deltree cannot take more than 1 argument"));
            lind_deltree(&utilcage, rootdir);
        }

        "rm" => {
            for file in args {
                utilcage.unlink_syscall(file.as_str());
            }
        }

        "mkdir" => {
            for dir in args {
                utilcage.mkdir_syscall(dir.as_str(), S_IRWXA);
            }
        }

        "rmdir" => {
            for dir in args {
                utilcage.rmdir_syscall(dir.as_str());
            }
        }

        _ => {
            eprintln!("Error, command unknown");
            return;
        }
    }
    lindrustfinalize();
}
