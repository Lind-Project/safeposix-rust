#[cfg(test)]
pub mod pipe_tests {
    use crate::interface;
    use crate::safeposix::{cage::*, filesystem, dispatcher::*};
    use super::super::*;
    use std::os::unix::fs::PermissionsExt;
    use std::fs::OpenOptions;
    use std::time::Instant;

    #[test]
    pub fn test_pipe() {
        // These can't really run until we figure out a better testing system/fsutils
        ut_lind_write_pipefile();
        print!("{{");
        for n in 4..17 {
            print!("\"{:?}\": [", n);
            for i in 1..11 {
                ut_lind_fs_pipe(n);
                if i != 10 { print!(", ") };
            }
            print!("]");
            if n != 16 {print!(", ")};
        }
        println!("}}");
    }


    pub fn ut_lind_write_pipefile() {

        let byte_chunk: usize = 131072;
        let num_writes: usize = 8192;

        lindrustinit(0);

        let cage = {CAGE_TABLE.get(&1).unwrap().clone()};


        let filefd = cage.open_syscall("test1gb.txt", O_CREAT | O_WRONLY, S_IRWXA);
        
        let mut buf: Vec<u8> = vec!['A' as u8; byte_chunk];
        let bufptr = buf.as_mut_ptr();

        for _i in 0..num_writes {
            cage.write_syscall(filefd, bufptr, byte_chunk);
        }

        assert_eq!(cage.close_syscall(filefd), 0);
        assert_eq!(cage.exit_syscall(EXIT_SUCCESS), EXIT_SUCCESS);
        lindrustfinalize();
    }


    pub fn ut_lind_fs_pipe(n: u32) {


        let gig = usize::pow(2, 30);
        let byte_chunk = usize::pow(2, n);
        let num_writes = gig/byte_chunk;

        let now = Instant::now();
        lindrustinit(0);

        let cage1 = {CAGE_TABLE.get(&1).unwrap().clone()};

        let mut pipefds = PipeArray {readfd: -1, writefd: -1};
        assert_eq!(cage1.pipe_syscall(&mut pipefds), 0);
        assert_eq!(cage1.fork_syscall(2), 0);

        let sender = std::thread::spawn(move || {

            let cage2 = {CAGE_TABLE.get(&2).unwrap().clone()};

            assert_eq!(cage2.close_syscall(pipefds.writefd), 0);
            assert_eq!(cage2.dup2_syscall(pipefds.readfd, 0), 0);
            assert_eq!(cage2.close_syscall(pipefds.readfd), 0);


            let mut bytes_read: usize = 1;

            let mut buf: Vec<u8> = Vec::with_capacity(gig);
            let mut bufptr = buf.as_mut_ptr();
            let mut buflen: usize = 0;
            let readchunk = usize::pow(2, 17);

            while bytes_read != 0 {
                bytes_read = cage2.read_syscall(0, bufptr, readchunk) as usize;
                unsafe {
                    bufptr = bufptr.add(bytes_read);
                    buf.set_len(buflen + bytes_read);
                    buflen += bytes_read;
                }
            }
            assert_eq!(cage2.close_syscall(0), 0);

            assert_eq!(cage2.exit_syscall(EXIT_SUCCESS), EXIT_SUCCESS);

        });
        
        assert_eq!(cage1.close_syscall(pipefds.readfd), 0);
        assert_eq!(cage1.dup2_syscall(pipefds.writefd, 1), 1);
        assert_eq!(cage1.close_syscall(pipefds.writefd), 0);

        let filefd = cage1.open_syscall("test1gb.txt", O_RDONLY, S_IRWXA);
        
        for _i in 0..num_writes {

            let mut buf: Vec<u8> = Vec::with_capacity(byte_chunk);
            let bufptr = buf.as_mut_ptr();
            unsafe { buf.set_len(byte_chunk); }

            cage1.read_syscall(filefd, bufptr, byte_chunk);
            cage1.write_syscall(1, bufptr, byte_chunk);
        }
        assert_eq!(cage1.close_syscall(filefd), 0);

        assert_eq!(cage1.close_syscall(1), 0);

        sender.join().unwrap();

        assert_eq!(cage1.exit_syscall(EXIT_SUCCESS), EXIT_SUCCESS);

        lindrustfinalize();
        print!("{}", now.elapsed().as_millis());
    }
}
