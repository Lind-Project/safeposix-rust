use criterion::{black_box, criterion_group, criterion_main, Criterion};

use rustposix::safeposix::cage::*;
use rustposix::interface;

// This is just a dummy benchmark to see if there is a problem with the
// benchmarker.  We can remove this once things are well setup and tested.
fn fibonacci(n: u64) -> u64 {
    match n {
        0 => 1,
        1 => 1,
        n => fibonacci(n-1) + fibonacci(n-2),
    }
}


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

    // First, just do a basic benchmark to show the runner works...
    c.bench_function("fib 20", |b| b.iter(|| fibonacci(black_box(20))));
    let a = get_dummy_cage();

    // Now, let's have a combined benchmark of all of the get*id* system calls
    // in RustPOSIX...  I'm not running these separately, because they should
    // not vary too much.
    c.bench_function("get*ids", |b| b.iter(|| 
        {
            a.getpid_syscall();
            a.getppid_syscall();
            a.getgid_syscall();
            a.getegid_syscall();
            a.getuid_syscall();
            a.geteuid_syscall();
        }
    ));
}

criterion_group!(benches, basic_rustposix_benchmark);
criterion_main!(benches);

