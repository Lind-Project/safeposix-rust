#[cfg(test)]
mod fs_tests {
    use crate::interface;
    use crate::safeposix::{cage::*, filesystem, dispatcher::*};
    use super::super::*;
    extern crate test;

    #[test]
    pub fn test_fs() {
        ut_lind_fs_simple(); // has to go first, else the data files created screw with link count test

        ut_lind_fs_chmod();
        ut_lind_fs_dir_chdir();
        ut_lind_fs_dup();
        ut_lind_fs_dup2();
        persistencetest();
        rdwrtest();
        prdwrtest();
        chardevtest();
        dispatch_tests::cagetest();
    }

    #[bench]
    fn rwtimer(b: &mut test::bench::Bencher) {
        b.bench(|_| {
        lindrustinit();
        let cage = {CAGE_TABLE.read().unwrap().get(&1).unwrap().clone()};

        let bcount: i32 = 128 * 1024;
        let textstring = std::iter::repeat("1234567890ABCDEF").take(bcount as usize).collect::<String>();
        let textstr = textstring.as_str();
        let textbuf = str2cbuf(textstr);
        let mut otherbuf = sizecbuf(bcount as usize);

        let mut timerstart = interface::starttimer();
        let fd = cage.open_syscall("16MBhex", O_CREAT | O_RDWR, S_IRWXA);
        println!("{}ns for rwtimer to open", interface::readtimer(timerstart).as_nanos());
        assert_ne!(fd, -1);

        timerstart = interface::starttimer();
        let writeres = cage.write_syscall(fd, textbuf, bcount as usize);
        println!("{}ns for rwtimer to write", interface::readtimer(timerstart).as_nanos());
        assert_eq!(writeres, bcount);

        timerstart = interface::starttimer();
        let lseekres = cage.lseek_syscall(fd, 0, SEEK_SET);
        println!("{}ns for rwtimer to lseek", interface::readtimer(timerstart).as_nanos());
        assert_eq!(lseekres, 0);

        timerstart = interface::starttimer();
        let readres = cage.read_syscall(fd, otherbuf.as_mut_ptr(), (bcount + 1) as usize);
        println!("{}ns for rwtimer to read", interface::readtimer(timerstart).as_nanos());
        assert_eq!(readres, bcount);
        assert_eq!(cbuf2str(&otherbuf), textstr);

        timerstart = interface::starttimer();
        let exitres = cage.exit_syscall();
        println!("{}ns for rwtimer to exit", interface::readtimer(timerstart).as_nanos());
        lindrustfinalize();
        });
    }

    pub fn persistencetest() {
        lindrustinit();
        let cage = {CAGE_TABLE.read().unwrap().get(&1).unwrap().clone()};

        cage.unlink_syscall("/testfile");
        let fd = cage.open_syscall("/testfile", O_CREAT | O_EXCL | O_RDWR, S_IRWXA);
        assert!(fd >= 0);

        assert_eq!(cage.close_syscall(fd), 0);
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

        //let's test exit's ability to close everything
        assert_ne!(cage.filedescriptortable.read().unwrap().len(), 0);
        assert_eq!(cage.exit_syscall(), 0);
        assert_eq!(cage.filedescriptortable.read().unwrap().len(), 0);

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

        //let's test lindrustfinalize's ability to call exit to close everything
        assert_ne!(cage.filedescriptortable.read().unwrap().len(), 0);
        lindrustfinalize();
        assert_eq!(cage.filedescriptortable.read().unwrap().len(), 0);
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
        assert_eq!(cage.close_syscall(fd), 0);

        let fd2 = cage.open_syscall("./urandom", O_RDWR, S_IRWXA);
        assert!(fd2 >= 0);
        let mut readbufrand = sizecbuf(1000);
        assert_eq!(cage.read_syscall(fd2, readbufrand.as_mut_ptr(), 1000), 1000);
        assert_eq!(cage.close_syscall(fd2), 0);
        assert_eq!(cage.exit_syscall(), 0);
        lindrustfinalize();
    }



    pub fn ut_lind_fs_chmod() {
        lindrustinit();
        let cage = {CAGE_TABLE.read().unwrap().get(&1).unwrap().clone()};

        let flags: i32 = O_TRUNC | O_CREAT | O_RDWR;
        let filepath = String::from("/chmodTestFile");

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

        let fd = cage.open_syscall(&filepath, flags, S_IRWXA);
        assert_eq!(cage.stat_syscall(&filepath, &mut statdata), 0);
        assert_eq!(statdata.st_mode, S_IRWXA | S_IFREG as u32);

        cage.chmod_syscall(&filepath, S_IRUSR | S_IRGRP);
        assert_eq!(cage.stat_syscall(&filepath, &mut statdata), 0);
        assert_eq!(statdata.st_mode, S_IRUSR | S_IRGRP | S_IFREG as u32);

        cage.chmod_syscall(&filepath, S_IRWXA);
        assert_eq!(cage.stat_syscall(&filepath, &mut statdata), 0);
        assert_eq!(statdata.st_mode, S_IRWXA | S_IFREG as u32);

        cage.close_syscall(fd);
        assert_eq!(cage.exit_syscall(), 0);
        lindrustfinalize();
    }



    pub fn ut_lind_fs_dir_chdir() {
        lindrustinit();
        let cage = {CAGE_TABLE.read().unwrap().get(&1).unwrap().clone()};
        
        //testing the ability to make and change to directories

        assert_eq!(cage.mkdir_syscall(&String::from("/subdir1"), S_IRWXA), 0);
        assert_eq!(cage.mkdir_syscall(&String::from("/subdir1/subdir2"), S_IRWXA), 0);
        assert_eq!(cage.mkdir_syscall(&String::from("/subdir1/subdir2/subdir3"), 0), 0);
        
        assert_eq!(cage.access_syscall(&String::from("subdir1"), F_OK), 0);
        assert_eq!(cage.chdir_syscall("subdir1"), 0);

        assert_eq!(cage.access_syscall(&String::from("subdir2"), F_OK), 0);
        assert_eq!(cage.chdir_syscall(".."), 0);

        assert_eq!(cage.access_syscall(&String::from("subdir1"), F_OK), 0);
        assert_eq!(cage.chdir_syscall("/subdir1/subdir2/subdir3"), 0);
        assert_eq!(cage.access_syscall(&String::from("../../../subdir1"), F_OK), 0);

        assert_eq!(cage.exit_syscall(), 0);
        lindrustfinalize();
    }



    pub fn ut_lind_fs_dup() {
        lindrustinit();
        let cage = {CAGE_TABLE.read().unwrap().get(&1).unwrap().clone()};

        let flags: i32 = O_TRUNC | O_CREAT | O_RDWR;
        let mode: i32 = 438;   // 0666
        let filepath = String::from("/dupfile");

        let fd = cage.open_syscall(&filepath, flags, S_IRWXA);
        let mut temp_buffer = sizecbuf(2);
        assert!(fd >= 0);
        assert_eq!(cage.write_syscall(fd, str2cbuf("12"), 2), 2);
        assert_eq!(cage.lseek_syscall(fd, 0, SEEK_SET), 0);
        assert_eq!(cage.read_syscall(fd, temp_buffer.as_mut_ptr(), 2), 2);
        assert_eq!(cbuf2str(&temp_buffer), "12");

        //duplicate the file descriptor
        let fd2 = cage.dup_syscall(fd, None);
        assert!(fd != fd2);

        //essentially a no-op, but duplicate again -- they should be diff &fd's
        let fd3 = cage.dup_syscall(fd, None);
        assert!(fd != fd2 && fd != fd3);

        //We don't need all three, though:
        cage.close_syscall(fd3);

        assert_eq!(cage.lseek_syscall(fd, 0, SEEK_END), 2);
        assert_eq!(cage.lseek_syscall(fd2, 0, SEEK_END), 2);

        // write some data to move the first position
        assert_eq!(cage.write_syscall(fd, str2cbuf("34"), 2), 2);

        //Make sure that they are still in the same place:
        let mut buffer = sizecbuf(4);
        assert_eq!(cage.lseek_syscall(fd, 0, SEEK_SET), cage.lseek_syscall(fd2, 0, SEEK_SET));
        assert_eq!(cage.read_syscall(fd, buffer.as_mut_ptr(), 4), 4);
        assert_eq!(cbuf2str(&buffer), "1234");

        cage.close_syscall(fd);

        //the other &fd should still work
        assert_eq!(cage.write_syscall(fd2, str2cbuf("5678"), 4), 4);
        cage.lseek_syscall(fd2,0,SEEK_CUR);

        assert_eq!(cage.lseek_syscall(fd2, 0, SEEK_SET), 0);
        let mut buffer2 = sizecbuf(8);
        assert_eq!(cage.read_syscall(fd2, buffer2.as_mut_ptr(), 8), 8);
        cage.close_syscall(fd2);
        assert_eq!(cbuf2str(&buffer2), "12345678");

        assert_eq!(cage.exit_syscall(), 0);
        lindrustfinalize();
    }



    pub fn ut_lind_fs_dup2() {
        lindrustinit();
        let cage = {CAGE_TABLE.read().unwrap().get(&1).unwrap().clone()};

        let flags: i32 = O_TRUNC | O_CREAT | O_RDWR;
        let mode: i32 = 438;   // 0666
        let filepath = String::from("/dup2file");

        let fd = cage.open_syscall(&filepath, flags, S_IRWXA);

        assert_eq!(cage.write_syscall(fd, str2cbuf("12"), 2), 2);

        let fd2: i32 = cage.dup2_syscall(fd, fd+1 as i32);

        //should be a no-op
        let fd2: i32 = cage.dup2_syscall(fd, fd+1 as i32);

        assert_eq!(cage.lseek_syscall(fd, 0, SEEK_CUR), cage.lseek_syscall(fd2, 0, SEEK_CUR));
        assert_eq!(cage.write_syscall(fd, str2cbuf("34"), 2), 2);
        assert_eq!(cage.lseek_syscall(fd, 0, SEEK_CUR), cage.lseek_syscall(fd2, 0, SEEK_CUR));

        let mut buffer = sizecbuf(4);
        assert_eq!(cage.lseek_syscall(fd2, 0, SEEK_SET), 0);
        assert_eq!(cage.read_syscall(fd, buffer.as_mut_ptr(), 4), 4);
        assert_eq!(cbuf2str(&buffer), "1234");

        assert_eq!(cage.close_syscall(fd), 0);

        let mut buffer2 = sizecbuf(8);
        assert_eq!(cage.lseek_syscall(fd2, 0, SEEK_END), 4);
        assert_eq!(cage.write_syscall(fd2, str2cbuf("5678"), 4), 4);
        
        assert_eq!(cage.lseek_syscall(fd2, 0, SEEK_SET), 0);
        assert_eq!(cage.read_syscall(fd2, buffer2.as_mut_ptr(), 8), 8);
        assert_eq!(cbuf2str(&buffer2), "12345678");

         assert_eq!(cage.close_syscall(fd2), 0);
        assert_eq!(cage.exit_syscall(), 0);
        lindrustfinalize();
    }



    pub fn ut_lind_fs_simple() {
        lindrustinit();
        let cage = {CAGE_TABLE.read().unwrap().get(&1).unwrap().clone()};

        assert_eq!(cage.access_syscall("/", F_OK), 0);
        assert_eq!(cage.access_syscall("/", X_OK|R_OK), 0);

        let mut statdata2 = StatData{
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

        assert_eq!(cage.stat_syscall("/", &mut statdata2), 0);
        //ensure that there are two hard links

        //TO DO: Fix the test underneath this
        assert_eq!(statdata2.st_nlink, 3); //now this is 6 no matter what?

        //ensure that there is no associated size
        assert_eq!(statdata2.st_size, 0);
        
        assert_eq!(cage.exit_syscall(), 0);
        lindrustfinalize();
    }
}
