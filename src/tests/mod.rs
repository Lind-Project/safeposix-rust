mod fs_tests;

use crate::interface;
use crate::safeposix::{cage::*};

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

pub fn init_cage() -> Cage {
    let mut cage = Cage{cageid: 0,
                        cwd: interface::RustLock::new(interface::RustRfc::new(interface::RustPathBuf::from("/"))), 
                        parent: 0, 
                        filedescriptortable: interface::RustLock::new(interface::RustHashMap::new())};
    cage.load_lower_handle_stubs();
    cage
}
