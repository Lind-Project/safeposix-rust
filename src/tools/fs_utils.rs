#![feature(once_cell)]
#![feature(rustc_private)] //for private crate imports for tests
#![feature(vec_into_raw_parts)]
#![allow(unused)]

use std::env;
use std::fs::File;
use std::io::{Read, prelude};
use std::iter::repeat;

mod interface;
mod safeposix;
use safeposix::{cage::*, filesystem::*, dispatcher::{lindrustfinalize, lindrustinit}};
//assume deserialization

fn update_dir_into_lind(cage: &Cage, hostfilepath: interface::RustPathBuf, lindfilepath: String) {
    if !hostfilepath.exists() {
        if let Ok(_) = hostfilepath.read_link() {
            return; //print error todo, ignore broken symlink on host fs
        } //if read_link succeeds it's a symlink
    } else {
        panic!("Cannot locate file on host fs: {:?}", hostfilepath);
    }

    //update directly if not a directory on the host, otherwise recursively handle children
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

    //compare files to tell whether they are identical
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

    //if they are not the same file, remove the lind file and replace it with the host file
    if !samefile {
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

    //update directly if not a directory on the host, otherwise recursively handle children
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

    //if a directory in the lindfilepath does not exist in the lind file system, create it!
    let mut ancestor = interface::RustPathBuf::from("/");
    for component in lindtruepath.parent().unwrap().components() {
        ancestor.push(component);
        let mut lindstat_res: StatData;

        //check whether file exists
        let stat_us = cage.stat_syscall(format!("{:?}", ancestor).as_str(), &mut lindstat_res);
        if stat_us == 0 {continue;}
        if stat_us != -(Errno::ENOENT as i32) {
            panic!("Fatal error in trying to get lind file path");
        }

        //check whether we are supposed to create missing directories, and whether we'd be
        //clobbering anything to do so (if so error out)
        if create_missing_dirs && is_dir(lindstat_res.st_mode) {
            cage.mkdir_syscall(format!("{:?}", ancestor).as_str(), S_IRWXA); //let's not mirror stat data
        } else {
            panic!("Lind fs path does not exist but should not be created {:?}", ancestor);
        }
    }

    //copy file contents into lind file system
    let host_fileobj = File::open(hostfilepath).unwrap();
    let mut filecontents: Vec<u8> = Vec::new();
    host_fileobj.read_to_end(&mut filecontents);

    let lindfd = cage.open_syscall(format!("{:?}", lindtruepath).as_str(), O_CREAT | O_TRUNC | O_WRONLY, S_IRWXA);
    let veclen = filecontents.len();
    let fileslice = filecontents.as_slice();
    let writtenlen = cage.write_syscall(lindfd, fileslice.as_ptr(), veclen);

    //confirm that write succeeded
    assert_eq!(veclen as i32, writtenlen);

    //get diagnostic data to print
    let mut lindstat_res: StatData;
    let stat_us = cage.fstat_syscall(lindfd, &mut lindstat_res);
    let inode = lindstat_res.st_ino;

    cage.close_syscall(lindfd);

    println!("Copied {:?} as {} ({})", hostfilepath, lindfilepath, inode);
}

fn visit_children<T>(cage: &Cage, path: String, dirvisitor: fn(&Cage, T), nondirvisitor: fn(&Cage, T)) {
}

fn lind_deltree(cage: &Cage, path: String) {
    let mut lindstat_res: StatData;
    let stat_us = cage.stat_syscall(path.as_str(), &mut lindstat_res);
    if !is_dir(lindstat_res.st_mode) {panic!("Delree must be run on a directory!");}
    visit_children(cage, path, |childcage, childpath| {
        lind_deltree(childcage, childpath);
    }, |childcage, childpath| {
        childcage.unlink_syscall(childpath.as_str());
    });
    cage.rmdir_syscall(path.as_str());
}

fn lind_tree(cage: &Cage, path: String, indentlevel: usize) {
    let mut lindstat_res: StatData;
    let stat_us = cage.stat_syscall(path.as_str(), &mut lindstat_res);
    if !is_dir(lindstat_res.st_mode) {panic!("Tree must be run on a directory!");}
    visit_children(cage, path, |childcage, (childpath, childindentlevel)| {
        print!("{}", "|   ".repeat(childindentlevel));
        if childindentlevel > 0 {
            print!("{}", "|---");
        }
        println!("{}", childpath);
        lind_tree(childcage, childpath, childindentlevel + 1);
    }, |childcage, (childpath, childindentlevel)| {
        print!("{}", "|   ".repeat(childindentlevel));
        if childindentlevel > 0 {
            print!("{}", "|---");
        }
        println!("{}", childpath);
    });
}

fn lind_ls(cage: &Cage, path: String) {
    let mut lindstat_res: StatData;
    let stat_us = cage.stat_syscall(path.as_str(), &mut lindstat_res);
    if is_dir(lindstat_res.st_mode) {
        visit_children(cage, path, |childcage, childpath: String| {
            print!("{}/ ", childpath);
        }, |childcage, childpath| {
            print!("{} ", childpath);
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
    let args = env::args();
    let utilcage = Cage{cageid: 0,
                        cwd: interface::RustLock::new(interface::RustRfc::new(interface::RustPathBuf::from("/"))),
                        parent: 0, 
                        filedescriptortable: interface::RustLock::new(interface::RustHashMap::new())};

    args.next();//first arg is executable, we don't care
    let command = if let Some(cmd) = args.next() {
        cmd.as_str()
    } else {
        print_usage();
        return; //print usage
    };

    match command {
        "help" | "usage" => {
            print_usage();
        }
        "cp" => {
            let source = args.next().expect("cp needs 2 arguments");
            let dest = args.next().expect("cp needs 2 arguments");
            args.next().and_then::<String, fn(String) -> Option<String>>(|_| panic!("cp cannot take more than 2 arguments"));
            cp_dir_into_lind(&utilcage, interface::RustPathBuf::from(source), dest, true);
        }
        "update" => {
            let source = args.next().expect("update needs 2 arguments");
            let dest = args.next().expect("update needs 2 arguments");
            args.next().and_then::<String, fn(String) -> Option<String>>(|_| panic!("update cannot take more than 2 arguments"));
            update_dir_into_lind(&utilcage, interface::RustPathBuf::from(source), dest);
        }
        "ls" => {
            let file = args.next().expect("ls needs 1 argument");
            args.next().and_then::<String, fn(String) -> Option<String>>(|_| panic!("ls cannot take more than 1 argument"));
            lind_ls(&utilcage, file);
        }
        "tree" => {
            let rootdir = if let Some(dirstr) = args.next() {
                dirstr.as_str()
            } else {"/"}.to_string();
            lind_tree(&utilcage, rootdir, 0);
        }
        "format" => {
            *FS_METADATA.write().unwrap() = FilesystemMetadata::blank_fs_init();
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
            return; //error message later
        }
    }
    lindrustfinalize();
}
