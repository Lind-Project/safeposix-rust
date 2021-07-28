// Network related constants

use crate::interface;

// Define constants using static or const
// Imported into net_calls file

#[derive(Debug)]
#[repr(C)]
pub struct EpollEvent {
    pub events: u32,
    pub fd: i32 
    //in native this is a union which could be one of a number of things
    //however, we only support EPOLL_CTL subcommands which take the fd
}
