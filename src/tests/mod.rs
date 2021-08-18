mod fs_tests;
mod pipe_tests;
mod networking_tests;

use crate::interface;
use crate::safeposix::{cage::*, filesystem::*};

#[cfg(test)]
mod main_tests {
    use crate::tests::networking_tests::net_tests::net_tests;
    use crate::tests::fs_tests::fs_tests::test_fs;
    use crate::tests::pipe_tests::pipe_tests::test_pipe;

    #[test]
    pub fn tests() {
        println!("FS TESTS");
        test_fs();

        println!("NET TESTS");
        net_tests();
        
        println!("PIPE TESTS");
        // test_pipe();
    }
}

pub fn str2cbuf(ruststr: &str) -> *mut u8 {
    let cbuflenexpected = ruststr.len();
    let (ptr, len, _) = ruststr.to_string().into_raw_parts();
    assert_eq!(len, cbuflenexpected);
    return ptr;
}

pub fn sizecbuf<'a>(size: usize) -> Box<[u8]> {
    let v = vec![0u8; size];
    v.into_boxed_slice()
    //buf.as_mut_ptr() as *mut u8
}

pub fn cbuf2str(buf: &[u8]) -> &str {
    std::str::from_utf8(buf).unwrap()
}
