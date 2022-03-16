#[allow(unused_parens)]
#[cfg(test)]
pub mod fs_tests {
    use crate::interface;
    use crate::safeposix::{cage::*, dispatcher::*, filesystem};
    use super::super::*;
    use std::os::unix::fs::PermissionsExt;
    use std::fs::OpenOptions;

    pub fn test_fs() {
        ut_lind_fs_simple(); // has to go first, else the data files created screw with link count test

        // lindrustinit(0);
        // load_fs_special_files(&CAGE_TABLE.get(&1).unwrap(), None);
        // lindrustfinalize();

        // ut_lind_fs_broken_close();
        // ut_lind_fs_chmod();
        // ut_lind_fs_dir_chdir();
        // ut_lind_fs_dir_mode();
        // ut_lind_fs_dir_multiple();
        // ut_lind_fs_dup();
        // ut_lind_fs_dup2();
        // ut_lind_fs_fcntl();
        // ut_lind_fs_ioctl();
        // ut_lind_fs_fdflags();
        // ut_lind_fs_file_link_unlink();
        // ut_lind_fs_file_lseek_past_end();
        // ut_lind_fs_fstat_complex();
        // ut_lind_fs_getuid();
        // ut_lind_fs_load_fs();
        // ut_lind_fs_mknod();
        // ut_lind_fs_multiple_open();
        // ut_lind_fs_persistence_setup();
        // // ut_lind_fs_persistence_test();
        // ut_lind_fs_rename();
        // ut_lind_fs_rmdir();
        // ut_lind_fs_stat_file_complex();
        // ut_lind_fs_stat_file_mode();
        // ut_lind_fs_statfs();
        // ut_lind_fs_ftruncate();
        // ut_lind_fs_truncate();
        // ut_lind_fs_getdents();
        // ut_lind_fs_dir_chdir_getcwd();
        // // persistencetest();
        // rdwrtest();
        // prdwrtest();
        // chardevtest();
        // ut_lind_fs_exec_cloexec();
    }



    pub fn ut_lind_fs_simple() {
        lindrustinit(0);
        let cage = {CAGE_TABLE.get(&1).unwrap().clone()};

        assert_eq!(cage.access_syscall("/", F_OK), 0);
        assert_eq!(cage.access_syscall("/", X_OK|R_OK), 0);

        let mut statdata2 = StatData::default();

        assert_eq!(cage.stat_syscall("/", &mut statdata2), 0);
        //ensure that there are two hard links

        assert_eq!(statdata2.st_nlink, 3); //becomes six when data files are left from previous tests

        //ensure that there is no associated size
        assert_eq!(statdata2.st_size, 0);
        
        assert_eq!(cage.exit_syscall(EXIT_SUCCESS), EXIT_SUCCESS);
        lindrustfinalize();
    }



    pub fn persistencetest() {
        lindrustinit(0);
        let cage = {CAGE_TABLE.get(&1).unwrap().clone()};

        cage.unlink_syscall("/testfile");
        let fd = cage.open_syscall("/testfile", O_CREAT | O_EXCL | O_RDWR, S_IRWXA);
        assert!(fd >= 0);

        assert_eq!(cage.close_syscall(fd), 0);
        filesystem::persist_metadata(*filesystem::FS_METADATA);

        let metadatastring1 = interface::serde_serialize_to_bytes(&*filesystem::FS_METADATA).unwrap(); // before restore

        // filesystem::restore_metadata(&mut metadata); // should be the same as after restore

        let metadatastring2 = interface::serde_serialize_to_bytes(&*filesystem::FS_METADATA).unwrap();

        //compare lengths before and after since metadata serialization isn't deterministic (hashmaps)
        assert_eq!(metadatastring1.len(), metadatastring2.len()); 
        incref_root();//for util cage first
        incref_root();//then for init cage

        assert_eq!(cage.exit_syscall(EXIT_SUCCESS), EXIT_SUCCESS);
        lindrustfinalize();
    }



    pub fn rdwrtest() {
        lindrustinit(0);
        let cage = {CAGE_TABLE.get(&1).unwrap().clone()};

        let fd = cage.open_syscall("/foobar", O_CREAT | O_TRUNC | O_RDWR, S_IRWXA);
        assert!(fd >= 0);
 
        assert_eq!(cage.write_syscall(fd, str2cbuf("hello there!"), 12), 12);

        assert_eq!(cage.lseek_syscall(fd, 0, SEEK_SET), 0);
        let mut read_buf1 = sizecbuf(5);
        assert_eq!(cage.read_syscall(fd, read_buf1.as_mut_ptr(), 5), 5);
        assert_eq!(cbuf2str(&read_buf1), "hello");

        assert_eq!(cage.write_syscall(fd, str2cbuf(" world"), 6), 6);

        assert_eq!(cage.lseek_syscall(fd, 0, SEEK_SET), 0);
        let mut read_buf2 = sizecbuf(12);
        assert_eq!(cage.read_syscall(fd, read_buf2.as_mut_ptr(), 12), 12);
        assert_eq!(cbuf2str(&read_buf2), "hello world!");

        //let's test exit's ability to close everything
        assert_ne!(cage.filedescriptortable.len(), 0);
        assert_eq!(cage.exit_syscall(EXIT_SUCCESS), EXIT_SUCCESS);
        assert_eq!(cage.filedescriptortable.len(), 0);

        lindrustfinalize();
    }



    pub fn prdwrtest() {
        lindrustinit(0);
        let cage = {CAGE_TABLE.get(&1).unwrap().clone()};

        let fd = cage.open_syscall("/foobar2", O_CREAT | O_TRUNC | O_RDWR, S_IRWXA);
        assert!(fd >= 0);

        assert_eq!(cage.pwrite_syscall(fd, str2cbuf("hello there!"), 12, 0), 12);

        let mut read_buf1 = sizecbuf(5);
        assert_eq!(cage.pread_syscall(fd, read_buf1.as_mut_ptr(), 5, 0), 5);
        assert_eq!(cbuf2str(&read_buf1), "hello");

        assert_eq!(cage.pwrite_syscall(fd, str2cbuf(" world"), 6, 5), 6);

        let mut read_buf2 = sizecbuf(12);
        assert_eq!(cage.pread_syscall(fd, read_buf2.as_mut_ptr(), 12, 0), 12);
        assert_eq!(cbuf2str(&read_buf2), "hello world!");

        //let's test lindrustfinalize's ability to call exit to close everything
        assert_ne!(cage.filedescriptortable.len(), 0);
        lindrustfinalize();
        assert_eq!(cage.filedescriptortable.len(), 0);
    }



    pub fn chardevtest() {
        lindrustinit(0);
        let cage = {CAGE_TABLE.get(&1).unwrap().clone()};

        let fd = cage.open_syscall("/dev/zero", O_RDWR, S_IRWXA);
        assert!(fd >= 0);

        assert_eq!(cage.pwrite_syscall(fd, str2cbuf("Lorem ipsum dolor sit amet, consectetur adipiscing elit"), 55, 0), 55);

        let mut read_bufzero = sizecbuf(1000);
        assert_eq!(cage.pread_syscall(fd, read_bufzero.as_mut_ptr(), 1000, 0), 1000);
        assert_eq!(cbuf2str(&read_bufzero), std::iter::repeat("\0").take(1000).collect::<String>().as_str());

        assert_eq!(cage.chdir_syscall("dev"), 0);
        assert_eq!(cage.close_syscall(fd), 0);

        let fd2 = cage.open_syscall("./urandom", O_RDWR, S_IRWXA);
        assert!(fd2 >= 0);
        let mut read_bufrand = sizecbuf(1000);
        assert_eq!(cage.read_syscall(fd2, read_bufrand.as_mut_ptr(), 1000), 1000);
        assert_eq!(cage.close_syscall(fd2), 0);
        assert_eq!(cage.exit_syscall(EXIT_SUCCESS), EXIT_SUCCESS);
        lindrustfinalize();
    }



    pub fn ut_lind_fs_broken_close() {

        //testing a muck up with the inode table where the regular close does not work as intended

        lindrustinit(0);
        let cage = {CAGE_TABLE.get(&1).unwrap().clone()};

        //write should work
        let mut fd = cage.open_syscall("/broken_close_file", O_CREAT | O_EXCL | O_RDWR, S_IRWXA);
        assert_eq!(cage.write_syscall(fd, str2cbuf("Hello There!"), 12), 12);
        assert_eq!(cage.close_syscall(fd), 0);

        //close the file and then open it again... and then close it again
        fd = cage.open_syscall("/broken_close_file", O_RDWR, S_IRWXA);
        assert_eq!(cage.close_syscall(fd), 0);

        //let's try some things with connect
        //we are going to open a socket with a UDP specification...
        let sockfd = cage.socket_syscall(AF_INET, SOCK_STREAM, 0);

        //bind should not be interesting
        let mut sockad = interface::GenSockaddr::V4(interface::SockaddrV4::default());
        sockad.set_family(AF_INET as u16);
        assert_eq!(cage.bind_syscall(sockfd, &sockad), 0);

        fd = cage.open_syscall("/broken_close_file", O_RDWR, S_IRWXA);
        assert_eq!(cage.close_syscall(fd), 0);

        fd = cage.open_syscall("/broken_close_file", O_RDWR, S_IRWXA);
        assert_eq!(cage.close_syscall(fd), 0);

        assert_eq!(cage.exit_syscall(EXIT_SUCCESS), EXIT_SUCCESS);
        lindrustfinalize();
    }




    pub fn ut_lind_fs_chmod() {
        lindrustinit(0);
        let cage = {CAGE_TABLE.get(&1).unwrap().clone()};

        let flags: i32 = O_TRUNC | O_CREAT | O_RDWR;
        let filepath = "/chmodTestFile";

        let mut statdata = StatData::default();

        let fd = cage.open_syscall(filepath, flags, S_IRWXA);
        assert_eq!(cage.stat_syscall(filepath, &mut statdata), 0);
        assert_eq!(statdata.st_mode, S_IRWXA | S_IFREG as u32);

        assert_eq!(cage.chmod_syscall(filepath, S_IRUSR | S_IRGRP), 0);
        assert_eq!(cage.stat_syscall(filepath, &mut statdata), 0);
        assert_eq!(statdata.st_mode, S_IRUSR | S_IRGRP | S_IFREG as u32);

        assert_eq!(cage.chmod_syscall(filepath, S_IRWXA), 0);
        assert_eq!(cage.stat_syscall(filepath, &mut statdata), 0);
        assert_eq!(statdata.st_mode, S_IRWXA | S_IFREG as u32);

        assert_eq!(cage.close_syscall(fd), 0);
        assert_eq!(cage.exit_syscall(EXIT_SUCCESS), EXIT_SUCCESS);
        lindrustfinalize();
    }



    pub fn ut_lind_fs_dir_chdir() {
        lindrustinit(0);
        let cage = {CAGE_TABLE.get(&1).unwrap().clone()};
        
        //testing the ability to make and change to directories

        assert_eq!(cage.mkdir_syscall("/subdir1", S_IRWXA, None), 0);
        assert_eq!(cage.mkdir_syscall("/subdir1/subdir2", S_IRWXA, None), 0);
        assert_eq!(cage.mkdir_syscall("/subdir1/subdir2/subdir3", 0, None), 0);
        
        assert_eq!(cage.access_syscall("subdir1", F_OK), 0);
        assert_eq!(cage.chdir_syscall("subdir1"), 0);

        assert_eq!(cage.access_syscall("subdir2", F_OK), 0);
        assert_eq!(cage.chdir_syscall(".."), 0);

        assert_eq!(cage.access_syscall("subdir1", F_OK), 0);
        assert_eq!(cage.chdir_syscall("/subdir1/subdir2/subdir3"), 0);
        assert_eq!(cage.access_syscall("../../../subdir1", F_OK), 0);

        assert_eq!(cage.exit_syscall(EXIT_SUCCESS), EXIT_SUCCESS);
        lindrustfinalize();
    }



    pub fn ut_lind_fs_dir_mode() {
        lindrustinit(0);
        let cage = {CAGE_TABLE.get(&1).unwrap().clone()};

        let filepath1 = "/subdirDirMode1";
        let filepath2 = "/subdirDirMode2";
        
        let mut statdata = StatData::default();

        assert_eq!(cage.mkdir_syscall(filepath1, S_IRWXA, None), 0);
        assert_eq!(cage.stat_syscall(filepath1, &mut statdata), 0);
        assert_eq!(statdata.st_mode, S_IRWXA | S_IFDIR as u32);
        
        assert_eq!(cage.mkdir_syscall(filepath2, 0, None), 0);
        assert_eq!(cage.stat_syscall(filepath2, &mut statdata), 0);
        assert_eq!(statdata.st_mode, S_IFDIR as u32);

        assert_eq!(cage.exit_syscall(EXIT_SUCCESS), EXIT_SUCCESS);
        lindrustfinalize();
    }



    pub fn ut_lind_fs_dir_multiple() {
        lindrustinit(0);
        let cage = {CAGE_TABLE.get(&1).unwrap().clone()};

        assert_eq!(cage.mkdir_syscall("/subdirMultiple1", S_IRWXA, None), 0);
        assert_eq!(cage.mkdir_syscall("/subdirMultiple1/subdirMultiple2", S_IRWXA, None), 0);
        assert_eq!(cage.mkdir_syscall("/subdirMultiple1/subdirMultiple2/subdirMultiple3", 0, None), 0);

        let mut statdata = StatData::default();

        //ensure that the file is a dir with all of the correct bits on for nodes
        assert_eq!(cage.stat_syscall("/subdirMultiple1/subdirMultiple2", &mut statdata), 0);
        assert_eq!(statdata.st_mode, S_IRWXA | S_IFDIR as u32);

        assert_eq!(cage.stat_syscall("/subdirMultiple1/subdirMultiple2/subdirMultiple3", &mut statdata), 0);
        assert_eq!(statdata.st_mode, S_IFDIR as u32);

        assert_eq!(cage.exit_syscall(EXIT_SUCCESS), EXIT_SUCCESS);
        lindrustfinalize();
    }



    pub fn ut_lind_fs_dup() {
        lindrustinit(0);
        let cage = {CAGE_TABLE.get(&1).unwrap().clone()};

        let flags: i32 = O_TRUNC | O_CREAT | O_RDWR;
        let filepath = "/dupfile";

        let fd = cage.open_syscall(filepath, flags, S_IRWXA);
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
        assert_eq!(cage.close_syscall(fd3), 0);

        assert_eq!(cage.lseek_syscall(fd, 0, SEEK_END), 2);
        assert_eq!(cage.lseek_syscall(fd2, 0, SEEK_END), 2);

        // write some data to move the first position
        assert_eq!(cage.write_syscall(fd, str2cbuf("34"), 2), 2);

        //Make sure that they are still in the same place:
        let mut buffer = sizecbuf(4);
        assert_eq!(cage.lseek_syscall(fd, 0, SEEK_SET), cage.lseek_syscall(fd2, 0, SEEK_SET));
        assert_eq!(cage.read_syscall(fd, buffer.as_mut_ptr(), 4), 4);
        assert_eq!(cbuf2str(&buffer), "1234");

        assert_eq!(cage.close_syscall(fd), 0);

        //the other &fd should still work
        assert_eq!(cage.lseek_syscall(fd2,0,SEEK_END), 4);
        assert_eq!(cage.write_syscall(fd2, str2cbuf("5678"), 4), 4);

        assert_eq!(cage.lseek_syscall(fd2, 0, SEEK_SET), 0);
        let mut buffer2 = sizecbuf(8);
        assert_eq!(cage.read_syscall(fd2, buffer2.as_mut_ptr(), 8), 8);
        assert_eq!(cage.close_syscall(fd2), 0);
        assert_eq!(cbuf2str(&buffer2), "12345678");

        assert_eq!(cage.exit_syscall(EXIT_SUCCESS), EXIT_SUCCESS);
        lindrustfinalize();
    }



    pub fn ut_lind_fs_dup2() {
        lindrustinit(0);
        let cage = {CAGE_TABLE.get(&1).unwrap().clone()};

        let flags: i32 = O_TRUNC | O_CREAT | O_RDWR;
        let filepath = "/dup2file";

        let fd = cage.open_syscall(filepath, flags, S_IRWXA);

        assert_eq!(cage.write_syscall(fd, str2cbuf("12"), 2), 2);

        //trying to dup fd into fd + 1
        let _fd2: i32 = cage.dup2_syscall(fd, fd+1 as i32);

        //should be a no-op since the last line did the same thing
        let fd2: i32 = cage.dup2_syscall(fd, fd+1 as i32);

        //read/write tests for the files
        assert_eq!(cage.lseek_syscall(fd, 0, SEEK_END), cage.lseek_syscall(fd2, 0, SEEK_END));
        assert_eq!(cage.write_syscall(fd, str2cbuf("34"), 2), 2);
        assert_eq!(cage.lseek_syscall(fd, 0, SEEK_SET), cage.lseek_syscall(fd2, 0, SEEK_SET));

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
        assert_eq!(cage.exit_syscall(EXIT_SUCCESS), EXIT_SUCCESS);
        lindrustfinalize();
    }



    pub fn ut_lind_fs_fcntl() {
        lindrustinit(0);
        let cage = {CAGE_TABLE.get(&1).unwrap().clone()};

        let sockfd = cage.socket_syscall(AF_INET, SOCK_STREAM, 0);
        let filefd = cage.open_syscall("/fcntl_file", O_CREAT | O_EXCL, S_IRWXA);

        //set the setfd flag
        assert_eq!(cage.fcntl_syscall(sockfd, F_SETFD, O_CLOEXEC), 0);

        //checking to see if the wrong flag was set or not
        assert_eq!(cage.fcntl_syscall(sockfd, F_GETFD, 0), O_CLOEXEC);

        //let's get some more flags on the filefd
        assert_eq!(cage.fcntl_syscall(filefd, F_SETFL, O_RDONLY|O_NONBLOCK), 0);

        //checking if the flags are updated...
        assert_eq!(cage.fcntl_syscall(filefd, F_GETFL, 0), 2048);

        assert_eq!(cage.close_syscall(filefd), 0);
        assert_eq!(cage.close_syscall(sockfd), 0);

        assert_eq!(cage.exit_syscall(EXIT_SUCCESS), EXIT_SUCCESS);
        lindrustfinalize();
    }

    pub fn ut_lind_fs_ioctl() {
        lindrustinit(0);
        let cage = {CAGE_TABLE.get(&1).unwrap().clone()};
        
        let mut arg0: i32 = 0;
        let mut arg1: i32 = 1;

        let union0: IoctlPtrUnion = IoctlPtrUnion {int_ptr : &mut arg0};
        let union1: IoctlPtrUnion = IoctlPtrUnion {int_ptr : &mut arg1};

        let sockfd = cage.socket_syscall(AF_INET, SOCK_STREAM, 0);
        let filefd = cage.open_syscall("/ioctl_file", O_CREAT | O_EXCL, S_IRWXA);

        //try to use FIONBIO for a non-socket
        assert_eq!(cage.ioctl_syscall(filefd, FIONBIO, union0), -(Errno::ENOTTY as i32));

        //clear the O_NONBLOCK flag
        assert_eq!(cage.ioctl_syscall(sockfd, FIONBIO, union0), 0);

        //checking to see if the flag was updated
        assert_eq!(cage.fcntl_syscall(sockfd, F_GETFL, 0)&O_NONBLOCK, 0);

        //set the O_NONBLOCK flag
        assert_eq!(cage.ioctl_syscall(sockfd, FIONBIO, union1), 0);

        //checking to see if the flag was updated
        assert_eq!(cage.fcntl_syscall(sockfd, F_GETFL, 0)&O_NONBLOCK, O_NONBLOCK);

        //clear the O_NONBLOCK flag
        assert_eq!(cage.ioctl_syscall(sockfd, FIONBIO, union0), 0);

        //checking to see if the flag was updated
        assert_eq!(cage.fcntl_syscall(sockfd, F_GETFL, 0)&O_NONBLOCK, 0);

        assert_eq!(cage.close_syscall(filefd), 0);
        assert_eq!(cage.close_syscall(sockfd), 0);

        assert_eq!(cage.exit_syscall(EXIT_SUCCESS), EXIT_SUCCESS);
        lindrustfinalize();
    }

    pub fn ut_lind_fs_fdflags() {
        lindrustinit(0);
        let cage = {CAGE_TABLE.get(&1).unwrap().clone()};

        let path = "/fdFlagsFile";

        let fd = cage.creat_syscall(path, S_IRWXA);
        assert_eq!(cage.close_syscall(fd), 0);

        let read_fd = cage.open_syscall(path, O_RDONLY, S_IRWXA);
        assert_eq!(cage.lseek_syscall(read_fd, 0, SEEK_SET), 0);
        assert_eq!(cage.write_syscall(read_fd, str2cbuf("Hello! This should not write."), 28), -(Errno::EBADF as i32));

        let mut buf = sizecbuf(100);
        assert_eq!(cage.lseek_syscall(read_fd, 0, SEEK_SET), 0);

        //this fails because nothing is written to the readfd (the previous write was unwritable)
        assert_eq!(cage.read_syscall(read_fd, buf.as_mut_ptr(), 100), 0);
        assert_eq!(cage.close_syscall(read_fd), 0);

        let write_fd = cage.open_syscall(path, O_WRONLY, S_IRWXA);
        let mut buf2 = sizecbuf(100);
        assert_eq!(cage.lseek_syscall(write_fd, 0, SEEK_SET), 0);
        assert_eq!(cage.read_syscall(write_fd, buf2.as_mut_ptr(), 100), -(Errno::EBADF as i32));

        assert_eq!(cage.lseek_syscall(write_fd, 0, SEEK_SET), 0);
        assert_eq!(cage.write_syscall(write_fd, str2cbuf("Hello! This should write."), 24), 24);
        assert_eq!(cage.close_syscall(write_fd), 0);

        assert_eq!(cage.exit_syscall(EXIT_SUCCESS), EXIT_SUCCESS);
        lindrustfinalize();
    }



    pub fn ut_lind_fs_file_link_unlink() {
        lindrustinit(0);
        let cage = {CAGE_TABLE.get(&1).unwrap().clone()};

        let path = "/fileLink";
        let path2 = "/fileLink2";

        let fd = cage.open_syscall(path, O_CREAT | O_EXCL | O_WRONLY, S_IRWXA);
        assert_eq!(cage.lseek_syscall(fd, 0, SEEK_SET), 0);
        assert_eq!(cage.write_syscall(fd, str2cbuf("hi"), 2), 2);

        let mut statdata = StatData::default();

        assert_eq!(cage.stat_syscall(path, &mut statdata), 0);
        assert_eq!(statdata.st_size, 2);
        assert_eq!(statdata.st_nlink, 1);

        let mut statdata2 = StatData::default();

        //make sure that this has the same traits as the other file that we linked
        // and make sure that the link count on the orig file has increased
        assert_eq!(cage.link_syscall(path, path2), 0);
        assert_eq!(cage.stat_syscall(path, &mut statdata), 0);
        assert_eq!(cage.stat_syscall(path2, &mut statdata2), 0);
        assert!(statdata == statdata2);
        assert_eq!(statdata.st_nlink, 2);

        //now we unlink
        assert_eq!(cage.unlink_syscall(path), 0);
        assert_eq!(cage.stat_syscall(path2, &mut statdata2), 0);
        assert_eq!(statdata2.st_nlink, 1);

        //it shouldn't work to stat the orig since it is gone
        assert_ne!(cage.stat_syscall(path, &mut statdata), 0);
        assert_eq!(cage.unlink_syscall(path2), 0);

        assert_eq!(cage.exit_syscall(EXIT_SUCCESS), EXIT_SUCCESS);
        lindrustfinalize();
    }



    pub fn ut_lind_fs_file_lseek_past_end() {
        lindrustinit(0);
        let cage = {CAGE_TABLE.get(&1).unwrap().clone()};

        let path = "/lseekPastEnd";

        let fd = cage.open_syscall(path, O_CREAT | O_EXCL | O_RDWR, S_IRWXA);
        assert_eq!(cage.write_syscall(fd, str2cbuf("hello"), 5), 5);

        //seek past the end and then write
        assert_eq!(cage.lseek_syscall(fd, 10, SEEK_SET), 10);
        assert_eq!(cage.write_syscall(fd, str2cbuf("123456"), 6), 6);

        let mut buf = sizecbuf(16);
        assert_eq!(cage.lseek_syscall(fd, 0, SEEK_SET), 0);
        assert_eq!(cage.read_syscall(fd, buf.as_mut_ptr(), 20), 16);
        assert_eq!(cbuf2str(&buf), "hello\0\0\0\0\0123456");

        assert_eq!(cage.close_syscall(fd), 0);
        assert_eq!(cage.exit_syscall(EXIT_SUCCESS), EXIT_SUCCESS);
        lindrustfinalize();
    }



    pub fn ut_lind_fs_fstat_complex() {
        lindrustinit(0);

        let cage = {CAGE_TABLE.get(&1).unwrap().clone()};
        let path = "/complexFile";

        let fd = cage.open_syscall(path, O_CREAT | O_WRONLY, S_IRWXA);
        assert_eq!(cage.write_syscall(fd, str2cbuf("testing"), 4), 4);

        let mut statdata = StatData::default();

        assert_eq!(cage.fstat_syscall(fd, &mut statdata), 0);
        assert_eq!(statdata.st_size, 4);
        assert_eq!(statdata.st_nlink, 1);

        assert_eq!(cage.close_syscall(fd), 0);
        assert_eq!(cage.exit_syscall(EXIT_SUCCESS), EXIT_SUCCESS);
        lindrustfinalize();
    }



    pub fn ut_lind_fs_getuid() {
        lindrustinit(0);
        let cage = {CAGE_TABLE.get(&1).unwrap().clone()};

        //let's get the initial -1s out of the way
        cage.getgid_syscall();
        cage.getegid_syscall();
        cage.getuid_syscall();
        cage.geteuid_syscall();

        //testing to make sure that all of the gid and uid values are good to go when system is initialized
        assert_eq!(cage.getgid_syscall() as u32, DEFAULT_GID);
        assert_eq!(cage.getegid_syscall() as u32, DEFAULT_GID);
        assert_eq!(cage.getuid_syscall() as u32, DEFAULT_UID);
        assert_eq!(cage.geteuid_syscall() as u32, DEFAULT_UID);

        assert_eq!(cage.exit_syscall(EXIT_SUCCESS), EXIT_SUCCESS);
        lindrustfinalize();
    }



    pub fn ut_lind_fs_load_fs() {
        lindrustinit(0);
        let cage = {CAGE_TABLE.get(&1).unwrap().clone()};

        let mut statdata = StatData::default();

        //testing that all of the dev files made it out safe and sound
        cage.stat_syscall("/dev", &mut statdata);

        assert_eq!(cage.stat_syscall("/dev/null", &mut statdata), 0);
        assert_eq!(statdata.st_rdev, makedev(&DevNo {major: 1, minor: 3}));
        
        assert_eq!(cage.stat_syscall("/dev/random", &mut statdata), 0);
        assert_eq!(statdata.st_rdev, makedev(&DevNo {major: 1, minor: 8}));

        assert_eq!(cage.stat_syscall("/dev/urandom", &mut statdata), 0);
        assert_eq!(statdata.st_rdev, makedev(&DevNo {major: 1, minor: 9}));

        assert_eq!(cage.exit_syscall(EXIT_SUCCESS), EXIT_SUCCESS);
        lindrustfinalize();
    }



    pub fn ut_lind_fs_persistence_setup() {
        lindrustinit(0);
        let cage = {CAGE_TABLE.get(&1).unwrap().clone()};

        let path1 = "/simpleFileName";
        let path2 = "/simpelFileName2";
        let fd = cage.open_syscall(path1, O_CREAT | O_EXCL | O_RDWR, S_IRWXA);
        //testing that the read and write work as expected

        //just read the first 5 bytes of the file
        let mut read_buf = sizecbuf(5);
        assert_eq!(cage.write_syscall(fd, str2cbuf("Hello there!"), 12), 12);
        assert_eq!(cage.lseek_syscall(fd, 0, SEEK_SET), 0);
        assert_eq!(cage.read_syscall(fd, read_buf.as_mut_ptr(), 5), 5);
        assert_eq!(cbuf2str(&read_buf), "Hello");

        let mut read_buf2 = sizecbuf(12);
        assert_eq!(cage.write_syscall(fd, str2cbuf(" World"), 6), 6);
        assert_eq!(cage.lseek_syscall(fd, 0, SEEK_SET), 0);
        assert_eq!(cage.read_syscall(fd, read_buf2.as_mut_ptr(), 12), 12);
        assert_eq!(cbuf2str(&read_buf2), "Hello World!");

        //close the file descriptor
        assert_eq!(cage.close_syscall(fd), 0);
    
        //open another one and then remove it
        let fd2 = cage.open_syscall(path2, O_CREAT | O_EXCL | O_RDWR, S_IRWXA);
        let message = "================================================================================================";
        assert_eq!(cage.write_syscall(fd2, str2cbuf(message), message.len()), message.len() as i32);
         
        //close the file descriptor
        assert_eq!(cage.unlink_syscall(path2), 0);

        //have to retieve the metadata lock after the open syscall gets it
        {
            persist_metadata(*filesystem::FS_METADATA);
            let path = OpenOptions::new().read(false).write(true).open(METADATAFILENAME.clone());
            let result = path.unwrap().metadata().unwrap().permissions();
            assert_ne!(result.mode() & (S_IWUSR | S_IWGRP | S_IWOTH), 0);
        }

        assert_eq!(cage.close_syscall(fd2), 0);
        assert_eq!(cage.exit_syscall(EXIT_SUCCESS), EXIT_SUCCESS);
        lindrustfinalize();
    }



    pub fn ut_lind_fs_persistence_test() {

        //check that the setup was run first
        {
            persist_metadata(*filesystem::FS_METADATA);
            // let path = normpath(convpath(METADATAFILENAME), &cage);
            let path = OpenOptions::new().read(false).write(true).open(METADATAFILENAME.clone());
            let result = path.unwrap().metadata().unwrap().permissions();
            assert_ne!(result.mode() & (S_IWUSR | S_IWGRP | S_IWOTH), 0);

            //restore the metadata
            // restore_metadata(&mut metadata);
        }

        lindrustinit(0);
        let cage = {CAGE_TABLE.get(&1).unwrap().clone()};
        //taken from the set up call:
        let path1 = "/simpleFileName";

        //if everything works, then try to open the files from the metadata
        //it should exist
        let fd = cage.open_syscall(path1, O_CREAT | O_EXCL | O_RDWR, S_IRWXA);

        assert_ne!(cage.close_syscall(fd), 0);
        assert_eq!(cage.exit_syscall(EXIT_SUCCESS), EXIT_SUCCESS);
        lindrustfinalize();
    }



    pub fn ut_lind_fs_mknod() {
        // let's create /dev/null
        lindrustinit(0);
        let cage = {CAGE_TABLE.get(&1).unwrap().clone()};
        let dev = makedev(&DevNo {major: 1, minor: 3});
        let path = "/null";

        //now we are going to mknod /dev/null with create, read and write flags and permissions
        //and then makr sure that it exists
        assert_eq!(cage.mknod_syscall(path, S_IFCHR as u32, dev, None), 0);
        let fd = cage.open_syscall(path, O_RDWR, S_IRWXA);

        //checking the metadata of the file:
        let mut statdata = StatData::default();

        //should be a chr file, so let's check this
        let mut buf = sizecbuf(4);
        assert_eq!(cage.fstat_syscall(fd, &mut statdata), 0);
        assert_eq!(statdata.st_mode & S_FILETYPEFLAGS as u32, S_IFCHR as u32);
        assert_eq!(statdata.st_rdev, dev);
        assert_eq!(cage.write_syscall(fd, str2cbuf("test"), 4), 4);
        assert_eq!(cage.read_syscall(fd, buf.as_mut_ptr(), 4), 0);
        assert_eq!(cbuf2str(&buf), "\0\0\0\0");
        assert_eq!(cage.close_syscall(fd), 0);


        let mut statdata2 = StatData::default();

        //try it again with /dev/random
        let dev2 = makedev(&DevNo {major: 1, minor: 8});
        let path2 = "/random";

        //making the node and then making sure that it exists
        assert_eq!(cage.mknod_syscall(path2, S_IFCHR as u32, dev2, None), 0);
        let fd2 = cage.open_syscall(path2, O_RDWR, S_IRWXA);
        
        let mut buf2 = sizecbuf(4);
        assert_eq!(cage.fstat_syscall(fd2, &mut statdata2), 0);
        assert_eq!(statdata2.st_mode & S_FILETYPEFLAGS as u32, S_IFCHR as u32);
        assert_eq!(statdata2.st_rdev, dev2);
        assert_eq!(cage.write_syscall(fd2, str2cbuf("testing"), 7), 7);
        assert_ne!(cage.read_syscall(fd2, buf2.as_mut_ptr(), 7), 0);
        assert_eq!(cage.close_syscall(fd2), 0);

        assert_eq!(cage.exit_syscall(EXIT_SUCCESS), EXIT_SUCCESS);
        lindrustfinalize();
    }



    pub fn ut_lind_fs_multiple_open() {
        lindrustinit(0);
        let cage = {CAGE_TABLE.get(&1).unwrap().clone()};

        //try to open several files at once -- the fd's should not be overwritten
        let fd1 = cage.open_syscall("/foo", O_CREAT | O_EXCL | O_RDWR, S_IRWXA);
        let fd2 = cage.open_syscall("/foo", O_RDWR, S_IRWXA);
        assert_ne!(fd1, fd2);

        let flags: i32 = O_TRUNC | O_CREAT | O_RDWR;
        let mode: u32 = 0o666;   // 0666
        let name = "double_open_file";

        let mut read_buf = sizecbuf(2);
        let fd3 = cage.open_syscall(name, flags, mode);
        assert_eq!(cage.write_syscall(fd3, str2cbuf("hi"), 2), 2);
        assert_eq!(cage.lseek_syscall(fd3, 0, SEEK_SET), 0);
        assert_eq!(cage.read_syscall(fd3, read_buf.as_mut_ptr(), 2), 2);
        assert_eq!(cbuf2str(&read_buf), "hi");
        

        let _fd4 = cage.open_syscall(name, flags, mode);
        let mut buf = sizecbuf(5);
        assert_eq!(cage.lseek_syscall(fd3, 2, SEEK_SET), 2);
        assert_eq!(cage.write_syscall(fd3, str2cbuf("boo"), 3), 3);
        assert_eq!(cage.lseek_syscall(fd3, 0, SEEK_SET), 0);
        assert_eq!(cage.read_syscall(fd3, buf.as_mut_ptr(), 5), 5);
        assert_eq!(cbuf2str(&buf), "\0\0boo");

        assert_eq!(cage.exit_syscall(EXIT_SUCCESS), EXIT_SUCCESS);
        lindrustfinalize();
    }


    

    pub fn ut_lind_fs_rmdir() {
        lindrustinit(0);
        let cage = {CAGE_TABLE.get(&1).unwrap().clone()};

        let path = "/parent_dir/dir";
        assert_eq!(cage.mkdir_syscall("/parent_dir", S_IRWXA, None), 0);
        assert_eq!(cage.mkdir_syscall(path, S_IRWXA, None), 0);
        assert_eq!(cage.rmdir_syscall(path), 0);

        assert_eq!(cage.exit_syscall(EXIT_SUCCESS), EXIT_SUCCESS);
        lindrustfinalize();
    }



    pub fn ut_lind_fs_stat_file_complex() {
        lindrustinit(0);
        let cage = {CAGE_TABLE.get(&1).unwrap().clone()};
        let fd = cage.open_syscall("/fooComplex", O_CREAT | O_EXCL | O_WRONLY, S_IRWXA);

        assert_eq!(cage.write_syscall(fd, str2cbuf("hi"), 2), 2);
        
        let mut statdata = StatData::default();
        let mut statdata2 = StatData::default();

        assert_eq!(cage.fstat_syscall(fd, &mut statdata), 0);
        assert_eq!(statdata.st_size, 2);
        assert_eq!(statdata.st_nlink, 1);

        assert_eq!(cage.link_syscall("/fooComplex", "/barComplex"), 0);
        assert_eq!(cage.stat_syscall("/fooComplex", &mut statdata), 0);
        assert_eq!(cage.stat_syscall("/barComplex", &mut statdata2), 0);

        //check that they are the same and that the link count is 0
        assert!(statdata == statdata2);
        assert_eq!(statdata.st_nlink, 2);

        assert_eq!(cage.exit_syscall(EXIT_SUCCESS), EXIT_SUCCESS);
        lindrustfinalize();
    }



    pub fn ut_lind_fs_stat_file_mode() {
        lindrustinit(0);
        let cage = {CAGE_TABLE.get(&1).unwrap().clone()};
        let path = "/fooFileMode";
        let _fd = cage.open_syscall(path, O_CREAT | O_EXCL | O_WRONLY, S_IRWXA);

        let mut statdata = StatData::default();
        assert_eq!(cage.stat_syscall(path, &mut statdata), 0);
        assert_eq!(statdata.st_mode, S_IRWXA | S_IFREG as u32);

        //make a file without permissions and check that it is a reg file without permissions
        let path2 = "/fooFileMode2";
        let _fd2 = cage.open_syscall(path2, O_CREAT | O_EXCL | O_WRONLY, 0);
        assert_eq!(cage.stat_syscall(path2, &mut statdata), 0);
        assert_eq!(statdata.st_mode, S_IFREG as u32);

        //check that stat can be done on the current (root) dir
        assert_eq!(cage.stat_syscall(".", &mut statdata), 0);

        assert_eq!(cage.exit_syscall(EXIT_SUCCESS), EXIT_SUCCESS);
        lindrustfinalize();
    }


    
    pub fn  ut_lind_fs_statfs() {
        lindrustinit(0);
        let cage = {CAGE_TABLE.get(&1).unwrap().clone()};
        let mut fsdata = FSData::default();

        assert_eq!(cage.statfs_syscall("/", &mut fsdata), 0);
        assert_eq!(fsdata.f_type, 0xBEEFC0DE);
        assert_eq!(fsdata.f_bsize, 4096);

        assert_eq!(cage.exit_syscall(EXIT_SUCCESS), EXIT_SUCCESS);
        lindrustfinalize();
    }

    
    
    pub fn ut_lind_fs_rename() {
        lindrustinit(0);
        let cage = {CAGE_TABLE.get(&1).unwrap().clone()};

        let old_path = "/test_dir";
        assert_eq!(cage.mkdir_syscall(old_path, S_IRWXA, None), 0);
        assert_eq!(cage.rename_syscall(old_path, "/test_dir_renamed"), 0);

        assert_eq!(cage.exit_syscall(EXIT_SUCCESS), EXIT_SUCCESS);
        lindrustfinalize();
    }

    pub fn ut_lind_fs_ftruncate() {
        lindrustinit(0);
        let cage = {CAGE_TABLE.get(&1).unwrap().clone()};

        let fd = cage.open_syscall("/ftruncate", O_CREAT | O_TRUNC | O_RDWR, S_IRWXA);
        assert!(fd >= 0);

        // check if ftruncate() works for extending file with null bytes
        assert_eq!(cage.write_syscall(fd, str2cbuf("Hello there!"), 12), 12);
        assert_eq!(cage.ftruncate_syscall(fd, 15), 0);
        assert_eq!(cage.lseek_syscall(fd, 0, SEEK_SET), 0);
        let mut buf = sizecbuf(15);
        assert_eq!(cage.read_syscall(fd, buf.as_mut_ptr(), 15), 15);
        assert_eq!(cbuf2str(&buf), "Hello there!\0\0\0");

        // check if ftruncate() works for cutting off extra bytes
        assert_eq!(cage.ftruncate_syscall(fd, 5), 0);
        assert_eq!(cage.lseek_syscall(fd, 0, SEEK_SET), 0);
        let mut buf1 = sizecbuf(7);
        assert_eq!(cage.read_syscall(fd, buf1.as_mut_ptr(), 7), 5);
        assert_eq!(cbuf2str(&buf1), "Hello\0\0");

        assert_eq!(cage.exit_syscall(EXIT_SUCCESS), EXIT_SUCCESS);
        lindrustfinalize();
    }

    pub fn ut_lind_fs_truncate() {
        lindrustinit(0);
        let cage = {CAGE_TABLE.get(&1).unwrap().clone()};

        let path = String::from("/truncate");
        let fd = cage.open_syscall(&path, O_CREAT | O_TRUNC | O_RDWR, S_IRWXA);
        assert!(fd >= 0);

        // check if truncate() works for extending file with null bytes
        assert_eq!(cage.write_syscall(fd, str2cbuf("Hello there!"), 12), 12);
        assert_eq!(cage.truncate_syscall(&path, 15), 0);
        assert_eq!(cage.lseek_syscall(fd, 0, SEEK_SET), 0);
        let mut buf = sizecbuf(15);
        assert_eq!(cage.read_syscall(fd, buf.as_mut_ptr(), 15), 15);
        assert_eq!(cbuf2str(&buf), "Hello there!\0\0\0");

        // check if truncate() works for cutting off extra bytes
        assert_eq!(cage.truncate_syscall(&path, 5), 0);
        assert_eq!(cage.lseek_syscall(fd, 0, SEEK_SET), 0);
        let mut buf1 = sizecbuf(7);
        assert_eq!(cage.read_syscall(fd, buf1.as_mut_ptr(), 7), 5);
        assert_eq!(cbuf2str(&buf1), "Hello\0\0");

        assert_eq!(cage.exit_syscall(EXIT_SUCCESS), EXIT_SUCCESS);
        lindrustfinalize();
    }

    pub fn ut_lind_fs_getdents() {
        lindrustinit(0);
        let cage = {CAGE_TABLE.get(&1).unwrap().clone()};

        let bufsize = 50;
        let mut vec = vec![0u8; bufsize as usize];
        let baseptr: *mut u8 = &mut vec[0];
        
        assert_eq!(cage.mkdir_syscall("/getdents", S_IRWXA, None), 0);
        let fd = cage.open_syscall("/getdents", O_RDWR, S_IRWXA);
        assert_eq!(cage.getdents_syscall(fd, baseptr, bufsize as u32), 48);

        unsafe{
            let first_dirent = baseptr as *mut interface::ClippedDirent;
            assert!((*first_dirent).d_off == 24);
            let reclen_matched: bool = ((*first_dirent).d_reclen == 24);
            assert_eq!(reclen_matched, true);
            
            let nameoffset = baseptr.wrapping_offset(interface::CLIPPED_DIRENT_SIZE as isize);
            let returnedname = interface::RustCStr::from_ptr(nameoffset as *const i8);
            let name_matched: bool = (returnedname == interface::RustCStr::from_bytes_with_nul(b".\0").unwrap()) | (returnedname == interface::RustCStr::from_bytes_with_nul(b"..\0").unwrap());
            assert_eq!(name_matched, true);
            
            let second_dirent = baseptr.wrapping_offset(24) as *mut interface::ClippedDirent;
            assert!((*second_dirent).d_off >= 48);
        }

        assert_eq!(cage.close_syscall(fd), 0);
        assert_eq!(cage.exit_syscall(EXIT_SUCCESS), EXIT_SUCCESS);
        lindrustfinalize();
    }

    pub fn ut_lind_fs_dir_chdir_getcwd() {
        lindrustinit(0);
        let cage = {CAGE_TABLE.get(&1).unwrap().clone()};
        let needed = "/subdir1\0".as_bytes().to_vec().len();

        let needed_u32: u32 = needed as u32;

        let mut buf = vec![0u8; needed];
        let bufptr: *mut u8 = &mut buf[0];

        assert_eq!(cage.chdir_syscall("/"), 0);
        assert_eq!(cage.getcwd_syscall(bufptr, 0), -(Errno::ERANGE as i32));
        assert_eq!(cage.getcwd_syscall(bufptr, 1), -(Errno::ERANGE as i32));
        assert_eq!(cage.getcwd_syscall(bufptr, 2), 0);
        assert_eq!(std::str::from_utf8(&buf).unwrap(), "/\0\0\0\0\0\0\0\0");

        cage.mkdir_syscall("/subdir1", S_IRWXA, None);
        assert_eq!(cage.access_syscall("subdir1", F_OK), 0);
        assert_eq!(cage.chdir_syscall("subdir1"), 0);

        assert_eq!(cage.getcwd_syscall(bufptr, 0), -(Errno::ERANGE as i32));
        assert_eq!(cage.getcwd_syscall(bufptr, needed_u32-1), -(Errno::ERANGE as i32));
        assert_eq!(cage.getcwd_syscall(bufptr, needed_u32), 0);
        assert_eq!(std::str::from_utf8(&buf).unwrap(), "/subdir1\0");

        assert_eq!(cage.exit_syscall(EXIT_SUCCESS), EXIT_SUCCESS);
        lindrustfinalize();
    }

    pub fn ut_lind_fs_exec_cloexec() {
        lindrustinit(0);
        let cage = {CAGE_TABLE.get(&1).unwrap().clone()};
        let mut uselessstatdata = StatData::default();

        let fd1 = cage.open_syscall("/cloexecuted", O_CREAT | O_TRUNC | O_RDWR | O_CLOEXEC, S_IRWXA);
        let fd2 = cage.open_syscall("/cloexekept", O_CREAT | O_TRUNC | O_RDWR, S_IRWXA);
        assert!(fd1 > 0);
        assert!(fd2 > 0);
        assert_eq!(cage.fstat_syscall(fd1, &mut uselessstatdata), 0);
        assert_eq!(cage.fstat_syscall(fd2, &mut uselessstatdata), 0);

        assert_eq!(cage.exec_syscall(2), 0);

        let execcage = {CAGE_TABLE.get(&2).unwrap().clone()};

        assert_eq!(execcage.fstat_syscall(fd1, &mut uselessstatdata), -(Errno::EBADF as i32));
        assert_eq!(execcage.fstat_syscall(fd2, &mut uselessstatdata), 0);

        assert_eq!(execcage.close_syscall(fd2), 0);
        assert_eq!(cage.unlink_syscall("/cloexecuted"), 0);
        assert_eq!(cage.unlink_syscall("/cloexekept"), 0);

        assert_eq!(execcage.exit_syscall(EXIT_SUCCESS), EXIT_SUCCESS);
        lindrustfinalize();
    }
}
