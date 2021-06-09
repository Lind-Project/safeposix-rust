// get thread id via Thread

// retreive cage table


use crate::interface;

//use static cage_table; //?? not sure how I do this

#[repr(C)]
pub union Arg {
  //list datatypes
}


pub extern "C" fn dispatcher(callnum: i32, arg1: Arg, arg2: Arg, arg3: Arg, arg4: Arg, arg5: Arg, arg6: Arg) -> i32 {
    
    let cageid = interface::rust_gettid(); //figure this out

    // need to match based on if cage exists
    let current_cage = cage_table[cageid];

    //implement syscall method calling using matching
    // match based on syscall, take cagetable lock as read for all except fork/exec, which need write

    match {} //blah blah

    return code;
}
