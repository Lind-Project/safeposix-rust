/* Benchmarks for the microvisor implementation.  In general, I'm not doing
 * results checking / assertations to avoid adding bias to the results.  */


// I hate allowing this, but this is apparently a known issue for a lot of 
// code with CStrings.  https://github.com/rust-lang/rust/issues/78691  
// I've tried to sanity check where this occurs, but please, please, please 
// double check these parts of the code!
#![allow(temporary_cstring_as_ptr)]

use criterion::{criterion_group, criterion_main, Criterion};

use rustposix::safeposix::cage::*;
use rustposix::interface;

use std::ffi::*;


fn sizecbuf<'a>(size: usize) -> Box<[u8]> {
    let v = vec![0u8; size];
    v.into_boxed_slice()
}



pub fn basic_rustposix_benchmark(c: &mut Criterion) {

    // I'm following the initialization workflow from the unit tests here.
    //
    // I'm using the lindrustinit to set up cages and the file system.  
    rustposix::safeposix::dispatcher::lindrustinit(0);

    // Since all system calls are a method of a cage object, I also need this
    // reference. 
    let cage = interface::cagetable_getref(1);

    // --- COMPARING get*id CALLS ACROSS Lind + Native OS kernel ---
    let mut group = c.benchmark_group("Compare get*ids");

    // These should be quite different, so use a log axis..
    group.plot_config(criterion::PlotConfiguration::default().summary_scale(criterion::AxisScale::Logarithmic));

    // let's have a combined benchmark of all of the get*id* system calls
    // in RustPOSIX...  I'm not running these separately, because they should
    // not vary too much.
    group.bench_function("Lind get*ids", |b| b.iter(|| 
        {
            cage.getpid_syscall();
            cage.getppid_syscall();
            cage.getgid_syscall();
            cage.getegid_syscall();
            cage.getuid_syscall();
            cage.geteuid_syscall();
        }
    ));
    // For comparison let's time the native OS...
    group.bench_function("Native OS kernel get*ids", |b| b.iter(|| 
        {
            unsafe{
                libc::getpid();
                libc::getppid();
                libc::getgid();
                libc::getegid();
                libc::getuid();
                libc::geteuid();
            }
        }
    ));
    group.finish();




    // --- COMPARING open / close CALLS ACROSS Lind + Native OS kernel ---
    let mut group = c.benchmark_group("Compare fs:open+close");

    // Should be similar.  Use a linear scale...
    group.plot_config(criterion::PlotConfiguration::default().summary_scale(criterion::AxisScale::Linear));

    // Let's see how fast various file system calls are
    group.bench_function("Lind open+close", |b| b.iter(|| 
        {
            let fd = cage.open_syscall("foo",O_CREAT | O_TRUNC | O_WRONLY,S_IRWXA);
            cage.close_syscall(fd);
        }
    ));

    // For comparison let's time the native OS...
    group.bench_function("Native OS kernel open+close", |b| b.iter(|| 
        {
            unsafe{
                let fd = libc::open(CString::new("/tmp/foo").unwrap().as_ptr(),O_CREAT | O_TRUNC | O_WRONLY,S_IRWXA);
                libc::close(fd);
            }
        }
    ));
    group.finish();

    

    

    // --- COMPARING read + write + lseek CALLS ACROSS Lind + Native OS kernel ---
    let mut group = c.benchmark_group("Compare fs:write+read");

    // Should be similar.  Use a linear scale...
    group.plot_config(criterion::PlotConfiguration::default().summary_scale(criterion::AxisScale::Linear));

    let fd = cage.open_syscall("foo",O_CREAT | O_TRUNC | O_WRONLY,S_IRWXA);
    // Let's see how fast various file system calls are
    group.bench_function("Lind write+read", |b| b.iter(|| 
        {
            let _ = cage.write_syscall(fd,CString::new("Well, hello there!!!").unwrap().as_ptr(),20);
            cage.lseek_syscall(fd,0,SEEK_SET);
            let mut read_buffer = sizecbuf(20);
            cage.read_syscall(fd,read_buffer.as_mut_ptr(), 20);
            cage.lseek_syscall(fd,0,SEEK_SET);
        }
    ));

    cage.close_syscall(fd);
    cage.unlink_syscall("foo");


    let fd: c_int;

    unsafe {
        fd = libc::open(CString::new("/tmp/foo").unwrap().as_ptr(),O_CREAT | O_TRUNC | O_WRONLY,S_IRWXA);
    }

    // For comparison let's time the native OS...
    group.bench_function("Native OS kernel read+write", |b| b.iter(|| 
        {
            unsafe{
                let _ = libc::write(fd,CString::new("Well, hello there!!!").unwrap().as_ptr() as *const c_void,20);
                let mut read_buffer = sizecbuf(20);
                libc::lseek(fd,0,SEEK_SET);
                libc::read(fd,read_buffer.as_mut_ptr() as *mut c_void, 20);
                libc::lseek(fd,0,SEEK_SET);
            }
        }
    ));
    unsafe {
        libc::close(fd);
        libc::unlink(CString::new("/tmp/foo").unwrap().as_ptr());
    }

    group.finish();
    rustposix::safeposix::dispatcher::lindrustfinalize();





}

criterion_group!(benches, basic_rustposix_benchmark);
criterion_main!(benches);

