use criterion::{criterion_group, criterion_main, Criterion};

use rustposix::safeposix::cage::*;
use rustposix::interface;


fn get_dummy_cage() -> Cage {

    // I chose random numbers for values here, so if a value is returned, you
    // know which call was made by the result.
    let cageobj = Cage {
        cageid: 17, 
        cwd: interface::RustLock::new(interface::RustRfc::new(interface::RustPathBuf::from("/"))),
        parent: 15,
        filedescriptortable: Vec::new(),
        cancelstatus: interface::RustAtomicBool::new(false),
        getgid: interface::RustAtomicI32::new(7),
        getuid: interface::RustAtomicI32::new(8),
        getegid: interface::RustAtomicI32::new(9),
        geteuid: interface::RustAtomicI32::new(10),
        rev_shm: interface::Mutex::new(vec!()),
        mutex_table: interface::RustLock::new(vec!()),
        cv_table: interface::RustLock::new(vec!()),
        sem_table: interface::RustHashMap::new(),
        thread_table: interface::RustHashMap::new(),
        signalhandler: interface::RustHashMap::new(),
        sigset: interface::RustHashMap::new(),
        pendingsigset: interface::RustHashMap::new(),
        main_threadid: interface::RustAtomicU64::new(0),
        interval_timer: interface::IntervalTimer::new(20)

    };
    return cageobj;
}
    

pub fn basic_rustposix_benchmark(c: &mut Criterion) {

    // I'm not setting this up properly because I think I don't need that much
    // state.  I just want to call system calls.
    let a = get_dummy_cage();

    // --- COMPARING get*id CALLS ACROSS Lind + Native OS kernel ---
    let mut group = c.benchmark_group("Compare get*ids");

    // These should be quite different, so use a log axis..
    group.plot_config(criterion::PlotConfiguration::default().summary_scale(criterion::AxisScale::Logarithmic));

    // let's have a combined benchmark of all of the get*id* system calls
    // in RustPOSIX...  I'm not running these separately, because they should
    // not vary too much.
    group.bench_function("Lind get*ids", |b| b.iter(|| 
        {
            a.getpid_syscall();
            a.getppid_syscall();
            a.getgid_syscall();
            a.getegid_syscall();
            a.getuid_syscall();
            a.geteuid_syscall();
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
    group.finish()

}

criterion_group!(benches, basic_rustposix_benchmark);
criterion_main!(benches);

