#![feature(once_cell)] //for synclazy
#![feature(rustc_private)] //for private crate imports for tests
#![feature(vec_into_raw_parts)]
#![feature(test)]
#![allow(unused_imports)]

mod interface;
mod safeposix;
mod tests;
use crate::safeposix::{cage::*, filesystem, dispatcher::*};

fn main() {
    for i in 0..100 {
    lindrustinit();
    let cage = {CAGE_TABLE.read().unwrap().get(&1).unwrap().clone()};

    let bcount: i32 = 128 * 1024;
    let textstring = std::iter::repeat("1234567890ABCDEF").take(bcount as usize / 16).collect::<String>();
    let textstr = textstring.as_str();
    let textbuf = tests::str2cbuf(textstr);
    let mut otherbuf = tests::sizecbuf(bcount as usize);

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
    assert_eq!(tests::cbuf2str(&otherbuf), textstr);

    timerstart = interface::starttimer();
    let exitres = cage.exit_syscall();
    println!("{}ns for rwtimer to exit", interface::readtimer(timerstart).as_nanos());
    lindrustfinalize();
    }
}
