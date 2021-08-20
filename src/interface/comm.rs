// // Authors: Nicholas Renner and Jonathan Singer
// //
// //

use std::mem::size_of;
use std::ffi::CString;
extern crate libc;

#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub enum GenSockaddr {
    V4(SockaddrV4),
    V6(SockaddrV6)
}
impl GenSockaddr {
    pub fn port(&self) -> u16 {
        match self {
            GenSockaddr::V4(v4addr) => v4addr.sin_port,
            GenSockaddr::V6(v6addr) => v6addr.sin6_port
        }
    }
    pub fn set_port(&mut self, port: u16) {
        match self {
            GenSockaddr::V4(v4addr) => v4addr.sin_port = port,
            GenSockaddr::V6(v6addr) => v6addr.sin6_port = port
        };
    }

    pub fn addr(&self) -> GenIpaddr {
        match self {
            GenSockaddr::V4(v4addr) => GenIpaddr::V4(v4addr.sin_addr),
            GenSockaddr::V6(v6addr) => GenIpaddr::V6(v6addr.sin6_addr)
        }
    }
    pub fn set_addr(&mut self, ip: GenIpaddr){
        match self {
            GenSockaddr::V4(v4addr) => v4addr.sin_addr = if let GenIpaddr::V4(v4ip) = ip {v4ip} else {unreachable!()},
            GenSockaddr::V6(v6addr) => v6addr.sin6_addr = if let GenIpaddr::V6(v6ip) = ip {v6ip} else {unreachable!()}
        };
    }
    
    pub fn set_family(&mut self, family: u16){
        match self {
            GenSockaddr::V4(v4addr) => v4addr.sin_family = family,
            GenSockaddr::V6(v6addr) => v6addr.sin6_family = family
        };
    }
}

#[derive(Debug, Hash, Eq, PartialEq, Clone, Copy)]
pub enum GenIpaddr {
    V4(V4Addr),
    V6(V6Addr)
}

impl GenIpaddr {
    pub fn is_unspecified(&self) -> bool {
        match self {
            GenIpaddr::V4(v4ip) => v4ip.s_addr == 0,
            GenIpaddr::V6(v6ip) => v6ip.s6_addr == [0; 16],
        }
    }
}

#[repr(C)]
pub union SockaddrAll {
    pub sockaddr_in: *mut SockaddrV4,
    pub sockaddr_in6: *mut SockaddrV6
}

#[repr(C)]
#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy, Default)]
pub struct V4Addr {
    pub s_addr: u32
}
#[repr(C)]
#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy, Default)]
pub struct SockaddrV4 {
    pub sin_family: u16,
    pub sin_port: u16,
    pub sin_addr: V4Addr,
    pub padding: u64
}

#[repr(C)]
#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy, Default)]
pub struct V6Addr {
    pub s6_addr: [u8; 16]
}
#[repr(C)]
#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy, Default)]
pub struct SockaddrV6 {
    pub sin6_family: u16,
    pub sin6_port: u16,
    pub sin6_flowinfo: u32,
    pub sin6_addr: V6Addr,
    pub sin6_scope_id: u32
}

#[derive(Debug)]
pub struct Socket {
    raw_sys_fd: i32//make private right after
}

impl Socket {
    pub fn new(domain: i32, socktype: i32, protocol: i32) -> Socket {
        let fd = unsafe {libc::socket(domain, socktype, protocol)};
        if fd < 0 {panic!("Socket creation failed when it should never fail");}
        unsafe {libc::fcntl(fd, libc::F_SETFL, libc::O_NONBLOCK);}
        Socket {raw_sys_fd: fd}
    }

    pub fn bind(&self, addr: &GenSockaddr) -> i32 {
        let (finalsockaddr, addrlen) = match addr {
            GenSockaddr::V6(addrref6) => {((addrref6 as *const SockaddrV6).cast::<libc::sockaddr>(), size_of::<SockaddrV6>())}
            GenSockaddr::V4(addrref) => {((addrref as *const SockaddrV4).cast::<libc::sockaddr>(), size_of::<SockaddrV4>())}
        };
        unsafe {libc::bind(self.raw_sys_fd, finalsockaddr, addrlen as u32)}
    }

    pub fn connect(&self, addr: &GenSockaddr) -> i32 {
        let (finalsockaddr, addrlen) = match addr {
            GenSockaddr::V6(addrref6) => {((addrref6 as *const SockaddrV6).cast::<libc::sockaddr>(), size_of::<SockaddrV6>())}
            GenSockaddr::V4(addrref) => {((addrref as *const SockaddrV4).cast::<libc::sockaddr>(), size_of::<SockaddrV4>())}
        };
        unsafe {libc::fcntl(self.raw_sys_fd, libc::F_SETFL, 0);}
        let f = unsafe {libc::connect(self.raw_sys_fd, finalsockaddr, addrlen as u32)};
        unsafe {libc::fcntl(self.raw_sys_fd, libc::F_SETFL, libc::O_NONBLOCK);}
        f
    }

    pub fn sendto(&self, buf: *mut u8, len: usize, addr: Option<&GenSockaddr>) -> i32 {
        let (finalsockaddr, addrlen) = match addr {
            Some(GenSockaddr::V6(addrref6)) => {((addrref6 as *const SockaddrV6).cast::<libc::sockaddr>(), size_of::<SockaddrV6>())}
            Some(GenSockaddr::V4(addrref)) => {((addrref as *const SockaddrV4).cast::<libc::sockaddr>(), size_of::<SockaddrV4>())}
            None => {(std::ptr::null::<libc::sockaddr>() as *const libc::sockaddr, 0)}
        };
        unsafe {libc::sendto(self.raw_sys_fd, buf as *const libc::c_void, len, libc::MSG_DONTWAIT, finalsockaddr, addrlen as u32) as i32}
    }

    pub fn recvfrom(&self, buf: *mut u8, len: usize, addr: &mut Option<&mut GenSockaddr>) -> i32 {
        let (finalsockaddr, mut addrlen) = match addr {
            Some(GenSockaddr::V6(ref mut addrref6)) => {((addrref6 as *mut SockaddrV6).cast::<libc::sockaddr>(), size_of::<SockaddrV6>() as u32)}
            Some(GenSockaddr::V4(ref mut addrref)) => {((addrref as *mut SockaddrV4).cast::<libc::sockaddr>(), size_of::<SockaddrV4>() as u32)}
            None => {(std::ptr::null::<libc::sockaddr>() as *mut libc::sockaddr, 0)}
        };
        unsafe {libc::recvfrom(self.raw_sys_fd, buf as *mut libc::c_void, len, libc::MSG_DONTWAIT, finalsockaddr, &mut addrlen as *mut u32) as i32}
    }

    pub fn listen(&self, backlog: i32) -> i32 {
        unsafe {libc::listen(self.raw_sys_fd, backlog)}
    }

    pub fn accept(&self, isv4: bool) -> (Result<Self, i32>, GenSockaddr) {
        return if isv4 {
            let mut inneraddrbuf = SockaddrV4::default();
            let mut sadlen = size_of::<SockaddrV4>() as u32;
            let newfd = unsafe{libc::accept(self.raw_sys_fd, (&mut inneraddrbuf as *mut SockaddrV4).cast::<libc::sockaddr>(), &mut sadlen as *mut u32)};

            if newfd < 0 {
                (Err(newfd), GenSockaddr::V4(inneraddrbuf))
            } else {
                unsafe {libc::fcntl(newfd, libc::F_SETFL, libc::O_NONBLOCK);}
                (Ok(Self{raw_sys_fd: newfd}), GenSockaddr::V4(inneraddrbuf))
            }
        } else {
            let mut inneraddrbuf = SockaddrV6::default();
            let mut sadlen = size_of::<SockaddrV6>() as u32;
            let newfd = unsafe{libc::accept(self.raw_sys_fd, (&mut inneraddrbuf as *mut SockaddrV6).cast::<libc::sockaddr>(), &mut sadlen as *mut u32)};

            if newfd < 0 {
                (Err(newfd), GenSockaddr::V6(inneraddrbuf))
            } else {
                unsafe {libc::fcntl(newfd, libc::F_SETFL, libc::O_NONBLOCK);}
                (Ok(Self{raw_sys_fd: newfd}), GenSockaddr::V6(inneraddrbuf))
            }
        };
    }
}

impl Drop for Socket {
    fn drop(&mut self) {
        unsafe { libc::close(self.raw_sys_fd); }
    }
}
