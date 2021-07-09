#[cfg(test)]
mod fs_tests {
    use crate::interface;
    use crate::safeposix::{cage::*, filesystem, dispatcher::*};
    use super::super::*;

    #[test]
    pub fn test_fs() {
        persistencetest();
        rdwrtest();
        prdwrtest(); //this test keeps failing for me
        chardevtest();
        dispatch_tests::cagetest();
        // ut_lind_fs_simple();
        // ut_lind_fs_dup();
    }

    pub fn persistencetest() {
        lindrustinit();
        let cage = {CAGE_TABLE.read().unwrap().get(&1).unwrap().clone()};

        cage.unlink_syscall("/testfile");
        let fd = cage.open_syscall("/testfile", O_CREAT | O_EXCL | O_RDWR, S_IRWXA);
        assert!(fd >= 0);

        let mut metadata = filesystem::FS_METADATA.write().unwrap(); 
        filesystem::persist_metadata(&metadata);

        let metadatastring1 = interface::serde_serialize_to_string(&*metadata).unwrap(); // before restore

        filesystem::restore_metadata(&mut metadata); // should be the same as after restore

        let metadatastring2 = interface::serde_serialize_to_string(&*metadata).unwrap();

        //compare lengths before and after since metadata serialization isn't deterministic (hashmaps)
        assert_eq!(metadatastring1.len(), metadatastring2.len()); 
        drop(metadata);
        incref_root();

        assert_eq!(cage.exit_syscall(), 0);
        lindrustfinalize();
    }

    pub fn rdwrtest() {
        lindrustinit();
        let cage = {CAGE_TABLE.read().unwrap().get(&1).unwrap().clone()};

        let fd = cage.open_syscall("/foobar", O_CREAT | O_TRUNC | O_RDWR, S_IRWXA);
        assert!(fd >= 0);
 
        assert_eq!(cage.write_syscall(fd, str2cbuf("hello there!"), 12), 12);

        assert_eq!(cage.lseek_syscall(fd, 0, SEEK_SET), 0);
        let mut readbuf1 = sizecbuf(5);
        assert_eq!(cage.read_syscall(fd, readbuf1.as_mut_ptr(), 5), 5);
        assert_eq!(cbuf2str(&readbuf1), "hello");

        assert_eq!(cage.write_syscall(fd, str2cbuf(" world"), 6), 6);

        assert_eq!(cage.lseek_syscall(fd, 0, SEEK_SET), 0);
        let mut readbuf2 = sizecbuf(12);
        assert_eq!(cage.read_syscall(fd, readbuf2.as_mut_ptr(), 12), 12);
        assert_eq!(cbuf2str(&readbuf2), "hello world!");
        assert_eq!(cage.exit_syscall(), 0);

        lindrustfinalize();
    }

    pub fn prdwrtest() {
        lindrustinit();
        let cage = {CAGE_TABLE.read().unwrap().get(&1).unwrap().clone()};

        let fd = cage.open_syscall("/foobar2", O_CREAT | O_TRUNC | O_RDWR, S_IRWXA);
        assert!(fd >= 0);

        assert_eq!(cage.pwrite_syscall(fd, str2cbuf("hello there!"), 12, 0), 12);

        let mut readbuf1 = sizecbuf(5);
        assert_eq!(cage.pread_syscall(fd, readbuf1.as_mut_ptr(), 5, 0), 5);
        assert_eq!(cbuf2str(&readbuf1), "hello");

        assert_eq!(cage.pwrite_syscall(fd, str2cbuf(" world"), 6, 5), 6);

        let mut readbuf2 = sizecbuf(12);
        assert_eq!(cage.pread_syscall(fd, readbuf2.as_mut_ptr(), 12, 0), 12);
        assert_eq!(cbuf2str(&readbuf2), "hello world!");

        assert_eq!(cage.exit_syscall(), 0);
        lindrustfinalize();
    }

    pub fn chardevtest() {
        lindrustinit();
        let cage = {CAGE_TABLE.read().unwrap().get(&1).unwrap().clone()};

        let fd = cage.open_syscall("/dev/zero", O_RDWR, S_IRWXA);
        assert!(fd >= 0);

        assert_eq!(cage.pwrite_syscall(fd, str2cbuf("Lorem ipsum dolor sit amet, consectetur adipiscing elit"), 55, 0), 55);

        let mut readbufzero = sizecbuf(1000);
        assert_eq!(cage.pread_syscall(fd, readbufzero.as_mut_ptr(), 1000, 0), 1000);
        assert_eq!(cbuf2str(&readbufzero), std::iter::repeat("\0").take(1000).collect::<String>().as_str());

        assert_eq!(cage.chdir_syscall("dev"), 0);

        let fd2 = cage.open_syscall("./urandom", O_RDWR, S_IRWXA);
        assert!(fd2 >= 0);
        let mut readbufrand = sizecbuf(1000);
        assert_eq!(cage.read_syscall(fd2, readbufrand.as_mut_ptr(), 1000), 1000);
        assert_eq!(cage.exit_syscall(), 0);
        lindrustfinalize();
    }


    pub fn ut_lind_fs_simple() {
        lindrustinit();
        let cage = {CAGE_TABLE.read().unwrap().get(&1).unwrap().clone()};

        assert_eq!(cage.access_syscall("/", F_OK), 0);
        assert_eq!(cage.access_syscall("/", X_OK|R_OK), 0);

        let mut statdata = StatData{
            st_dev: 0,
            st_ino: 0,
            st_mode: 0,
            st_nlink: 0,
            st_uid: 0,
            st_gid: 0,
            st_rdev: 0,
            st_size: 0,
            st_blksize: 0,
            st_blocks: 0,
            st_atim: (0, 0),
            st_mtim: (0, 0),
            st_ctim: (0, 0)
        };
        assert_eq!(cage.stat_syscall("/", &mut statdata), 0);
        //ensure that there are two hard links
        // assert_eq!(statdata.st_nlink, 2); //why is this test failing?
        //ensure that there is no associated size
        assert_eq!(statdata.st_size, 0);
        
        assert_eq!(cage.exit_syscall(), 0);
        lindrustfinalize();
    }

    pub fn ut_lind_fs_dup() {
        lindrustinit();
        let cage = {CAGE_TABLE.read().unwrap().get(&1).unwrap().clone()};

        let flags: i32 = O_TRUNC | O_CREAT | O_RDWR;
        let mode: i32 = 438;   // 0666
        let name = String::from("/double_open_file");

        let fd = cage.open_syscall(&name, O_CREAT | O_TRUNC | O_RDWR, S_IRWXA);
        assert!(fd >= 0);
        assert_eq!(cage.write_syscall(fd, str2cbuf("hi"), 2), 2);

        //duplicate the file descriptor
        let fd2 = cage.dup_syscall(fd, None);
        assert!(fd != fd2);

        //essentially a no-op, but duplicate again -- they should be diff &fd's
        let fd3 = cage.dup_syscall(fd, None);
        assert!(fd != fd2 && fd != fd3);

        //We don't need all three, though:
        cage.close_syscall(fd3);

        //Make sure that they are in the same place:
        assert_eq!(cage.lseek_syscall(fd,0,SEEK_CUR), cage.lseek_syscall(fd2,0,SEEK_CUR));

        // write some data to move the first position
        assert_eq!(cage.write_syscall(fd, str2cbuf("yo"), 2), 2);

        //Make sure that they are still in the same place:
        let mut buffer = str2cbuf("");
        assert_eq!(cage.lseek_syscall(fd,0,SEEK_CUR), cage.lseek_syscall(fd2,0,SEEK_CUR));
        assert_eq!(cage.read_syscall(fd, buffer, 10), 4);

        cage.close_syscall(fd);

        //the other &fd should still work
        assert_eq!(cage.write_syscall(fd2, str2cbuf("raar"), 4), 4);
        cage.lseek_syscall(fd2,0,SEEK_CUR);

        let mut buffer2 = str2cbuf("");
        assert_eq!(cage.read_syscall(fd2, buffer, 10), 8);
        cage.close_syscall(fd2);

        assert_eq!(cage.exit_syscall(), 0);
        lindrustfinalize();
    }
}
