/* Benchmarks for the microvisor implementation.  In general, I'm not doing
 * results checking / assertations to avoid adding bias to the results.  */

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};

use rustposix::interface;

use std::ffi::*;

use std::time::Duration;

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

    // --- COMPARING read + write w/o lseek CALLS ACROSS Lind + Native OS kernel ---
    // This is separated because writeat and readat do a lot of seeking.  It's
    // useful to have a comparison which does not.
    let mut group = c.benchmark_group("Compare fs:write+read");

    // Should be similar.  Use a linear scale...
    group.plot_config(
        criterion::PlotConfiguration::default().summary_scale(criterion::AxisScale::Linear),
    );

    // First do this for Lind

    // Reduce the time to reduce disk space needed and go faster.
    // Default is 5s...
    group.measurement_time(Duration::from_secs(2));

    // Shorten the warm up time as well from 3s to this...
    group.warm_up_time(Duration::from_secs(1));

    // Iterate for different buffer sizes...
    for buflen in [1, 64, 1024, 65536].iter() {
        let deststring = tests::str2cbuf(
            &String::from_utf8(vec![b'X'; *buflen]).expect("error building string"),
        );

        // The size of the buffer and the amount we expect to read and write.
        // I need to type convert this because it's a usize by default.
        // I'm lazily converting with as here because it's not feasible to
        // test values where usize would overflow this.
        let expected_retval = *buflen as i32;

        let fd = cage.open_syscall("foo", O_CREAT | O_TRUNC | O_RDWR, S_IRWXA);
        // Let's see how fast various file system calls are
        group.bench_with_input(
            BenchmarkId::new("TF02:Lind write", buflen),
            buflen,
            |b, buflen| {
                b.iter(|| {
                    assert_eq!(cage.write_syscall(fd, deststring, *buflen),expected_retval);
                })
            },
        );

        // I'll read the file length so I don't overrun this with my reads...
        let file_length = cage.lseek_syscall(fd, 0, SEEK_CUR);

        cage.lseek_syscall(fd, 0, SEEK_SET);

        // My current position when reading...
        let mut pos = 0;

        let mut read_buffer = tests::sizecbuf(*buflen);

        group.bench_with_input(
            BenchmarkId::new("TF02:Lind read", buflen),
            buflen,
            |b, buflen| {
                b.iter(|| {
                    // Track the file pointer so you can backtrack if you make
                    // it to the end of the file.  This avoids having a bunch
                    // of garbage, 0 length reads skew the results...
                    pos += expected_retval;
                    if file_length <= pos {
                        cage.lseek_syscall(fd, 0, SEEK_SET);
                        pos = 0;
                    }
                    assert_eq!(cage.read_syscall(fd, read_buffer.as_mut_ptr(), *buflen),expected_retval);
                })
            },
        );

        cage.close_syscall(fd);
        cage.unlink_syscall("foo");
    }

    // Now do this for Native

    // Iterate for different buffer sizes...
    for buflen in [1, 64, 1024, 65536].iter() {
        let fd: c_int;
        let c_str = CString::new("/tmp/foo").unwrap();

        // The size of the buffer and the amount we expect to read and write.
        // I need to type convert this because it's a usize by default.
        // I'm lazily converting with as here because it's not feasible to
        // test values where usize would overflow this.
        // NOTE: This has a different type than Lind, which is i32.  I think
        // this is likely okay.
        let expected_retval = *buflen as isize;

        let path = c_str.into_raw() as *const u8;

        unsafe {
            fd = libc::open(path, O_CREAT | O_TRUNC | O_RDWR, S_IRWXA);
        }

        let deststring = tests::str2cbuf(
            &String::from_utf8(vec![b'X'; *buflen]).expect("error building string"),
        );

        // For comparison let's time the native OS...
        group.bench_with_input(
            BenchmarkId::new("TF02:Native write", buflen),
            buflen,
            |b, buflen| {
                b.iter(|| unsafe {
                    assert_eq!(libc::write(fd, deststring as *const c_void, *buflen),expected_retval);
                })
            },
        );

        // I'll read the file length so I don't overrun this with my reads...
        let file_length:isize;
        unsafe {
            file_length = libc::lseek(fd, 0, SEEK_CUR) as isize;

            // reset the file position
            libc::lseek(fd, 0, SEEK_SET);
        }

        // My current position when reading...
        let mut pos = 0;

        let mut read_buffer = tests::sizecbuf(*buflen);

        // For comparison let's time the native OS...
        group.bench_with_input(
            BenchmarkId::new("TF02:Native read", buflen),
            buflen,
            |b, buflen| {
                b.iter(|| unsafe {
                    // Track the file pointer so you can backtrack if you make
                    // it to the end of the file.  This avoids having a bunch
                    // of garbage, 0 length reads skew the results...
                    pos += expected_retval;
                    if file_length <= pos {
                        libc::lseek(fd, 0, SEEK_SET);
                        pos = 0;
                    }
                    assert_eq!(libc::read(fd, read_buffer.as_mut_ptr() as *mut c_void, *buflen),expected_retval);
                })
            },
        );

        unsafe {
            libc::close(fd);
            libc::unlink(path);
        }
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
