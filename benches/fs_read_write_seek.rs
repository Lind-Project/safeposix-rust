/* Benchmarks for the microvisor implementation.  In general, I'm not doing
 * results checking / assertations to avoid adding bias to the results.  */


use criterion::{criterion_group, criterion_main, Criterion};

use rustposix::interface;

use std::ffi::*;

use rustposix::safeposix::cage::*;

use rustposix::tests;

// Using this to include my criterion settings from a single shared file.
// I did not use "use" or "mod" because benches/ isn't in the crate's usual
// namespace and I didn't want to either make a separate crate with a single, 
// tiny file or add this file to the rustposix crate.
mod global_criterion_settings;


pub fn run_benchmark(c: &mut Criterion) {

    // I'm following the initialization workflow from the unit tests here.
    //
    // I'm using the lindrustinit to set up cages and the file system.  
    rustposix::safeposix::dispatcher::lindrustinit(0);

    // Since all system calls are a method of a cage object, I also need this
    // reference. 
    let cage = interface::cagetable_getref(1);



        // --- COMPARING read + write + lseek CALLS ACROSS Lind + Native OS kernel ---
    let mut group = c.benchmark_group("Compare fs:write+read+lseek");

    // Should be similar.  Use a linear scale...
    group.plot_config(criterion::PlotConfiguration::default().summary_scale(criterion::AxisScale::Linear));

    let fd = cage.open_syscall("foo",O_CREAT | O_TRUNC | O_WRONLY,S_IRWXA);
    // Let's see how fast various file system calls are
    group.bench_function("Lind write+read+lseek", |b| b.iter(||
        {
            let _ = cage.write_syscall(fd,tests::str2cbuf("Well, hello there!!!"),20);
            cage.lseek_syscall(fd,0,SEEK_SET);
            let mut read_buffer = tests::sizecbuf(20);
            cage.read_syscall(fd,read_buffer.as_mut_ptr(), 20);
            cage.lseek_syscall(fd,0,SEEK_SET);
        }
    ));

    cage.close_syscall(fd);
    cage.unlink_syscall("foo");


    let fd: c_int;

    unsafe {
        fd = libc::open(tests::str2cbuf("/tmp/foo"),O_CREAT | O_TRUNC | O_WRONLY,S_IRWXA);
    }

    // For comparison let's time the native OS...
    group.bench_function("Native OS kernel write+read+lseek", |b| b.iter(||
        {
            unsafe{
                let _ = libc::write(fd,tests::str2cbuf("Well, hello there!!!") as *const c_void,20);
                let mut read_buffer = tests::sizecbuf(20);
                libc::lseek(fd,0,SEEK_SET);
                libc::read(fd,read_buffer.as_mut_ptr() as *mut c_void, 20);
                libc::lseek(fd,0,SEEK_SET);
            }
        }
    ));
    unsafe {
        libc::close(fd);
        libc::unlink(tests::str2cbuf("/tmp/foo"));
    }
    group.finish();




    // This cleans up in ways I do not fully understand.  I think it ensures
    // the file system is cleaned up
    rustposix::safeposix::dispatcher::lindrustfinalize();


}

criterion_group!(name=benches; 
                 // Add the global settings here so we don't type it everywhere
                 config=global_criterion_settings::get_criterion(); 
                 targets=run_benchmark);
criterion_main!(benches);

