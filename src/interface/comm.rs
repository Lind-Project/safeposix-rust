// // Authors: Nicholas Renner and Jonathan Singer
// //
// //

use std::mem::size_of;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::fs::read_to_string;
use std::str::from_utf8;

extern crate libc;

static NET_DEV_FILENAME: &str = "net_devices";

static mut UD_ID_COUNTER: AtomicUsize = AtomicUsize::new(0);

#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub enum GenSockaddr {
    Unix(SockaddrUnix),
    V4(SockaddrV4),
    V6(SockaddrV6)
}
impl GenSockaddr {
    pub fn port(&self) -> u16 {
        match self {
            GenSockaddr::Unix(_unixaddr) => unreachable!(),
            GenSockaddr::V4(v4addr) => v4addr.sin_port,
            GenSockaddr::V6(v6addr) => v6addr.sin6_port
        }
    }
    pub fn set_port(&mut self, port: u16) {
        match self {
            GenSockaddr::Unix(_unixaddr) => unreachable!(),
            GenSockaddr::V4(v4addr) => v4addr.sin_port = port,
            GenSockaddr::V6(v6addr) => v6addr.sin6_port = port
        };
    }

    pub fn addr(&self) -> GenIpaddr {
        match self {
            GenSockaddr::Unix(_unixaddr) => unreachable!(),
            GenSockaddr::V4(v4addr) => GenIpaddr::V4(v4addr.sin_addr),
            GenSockaddr::V6(v6addr) => GenIpaddr::V6(v6addr.sin6_addr)
        }
    }

    pub fn set_addr(&mut self, ip: GenIpaddr){
        match self {
            GenSockaddr::Unix(_unixaddr) => unreachable!(),
            GenSockaddr::V4(v4addr) => v4addr.sin_addr = if let GenIpaddr::V4(v4ip) = ip {v4ip} else {unreachable!()},
            GenSockaddr::V6(v6addr) => v6addr.sin6_addr = if let GenIpaddr::V6(v6ip) = ip {v6ip} else {unreachable!()}
        };
    }
    
    pub fn set_family(&mut self, family: u16){
        match self {
            GenSockaddr::Unix(unixaddr) => unixaddr.sun_family = family,
            GenSockaddr::V4(v4addr) => v4addr.sin_family = family,
            GenSockaddr::V6(v6addr) => v6addr.sin6_family = family
        };
    }

    pub fn get_family(&self) -> u16 {
        match self {
            GenSockaddr::Unix(unixaddr) => unixaddr.sun_family,
            GenSockaddr::V4(v4addr) => v4addr.sin_family,
            GenSockaddr::V6(v6addr) => v6addr.sin6_family
        }
    }

    pub fn path(&self) -> &str {
        match self {
            GenSockaddr::Unix(unixaddr) => {
                let pathiter = &mut unixaddr.sun_path.split(|idx| *idx == 0);
                let pathslice = pathiter.next().unwrap().clone();
                let path = from_utf8(pathslice).unwrap();
                path
            }
            GenSockaddr::V4(_v4addr) => unreachable!(),
            GenSockaddr::V6(_v6addr) => unreachable!()
        }
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
    pub fn from_string(string: &str) -> Option<Self> {
        let v4candidate: Vec<&str> = string.split('.').collect();
        let v6candidate: Vec<&str> = string.split(':').collect();
        let v4l = v4candidate.len();
        let v6l = v6candidate.len();
        if v4l == 1 && v6l > 1 {
            //then we should try parsing it as an ipv6 address
            let mut shortarr = [0u8; 16];
            let mut shortindex = 0;
            let mut encountered_doublecolon = false;
            for short in v6candidate {

                if short.is_empty() {
                    //you can only have a double colon once in an ipv6 address
                    if encountered_doublecolon {
                        return None;
                    }
                    encountered_doublecolon = true;

                    let numzeros = 8 - v6l + 1; //+1 to account for this empty string element
                    if numzeros == 0 {
                        return None;
                    }
                    shortindex += numzeros;
                } else {
                    //ok we can actually parse the element in this case
                    if let Ok(b) = short.parse::<u16>() {
                        //manually handle big endianness
                        shortarr[2*shortindex] = (b >> 8) as u8;
                        shortarr[2*shortindex + 1] = (b & 0xff) as u8;
                        shortindex += 1;
                    } else {
                        return None;
                    }
                }
            }
            return Some(Self::V6(V6Addr{s6_addr: shortarr}));
        } else if v4l == 4 && v6l == 1 {
            //then we should try parsing it as an ipv4 address
            let mut bytearr = [0u8; 4];
            let mut shortindex = 0;
            for byte in v4candidate {
                if let Ok(b) = byte.parse::<u8>() {
                    bytearr[shortindex] = b;
                    shortindex += 1;
                } else {
                    return None;
                }
            }
            return Some(Self::V4(V4Addr{s_addr: u32::from_ne_bytes(bytearr)}));
        } else {
            return None;
        }
    }
}

#[repr(C)]
#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub struct SockaddrUnix {
    pub sun_family: u16,
    pub sun_path: [u8; 108]
}

pub fn new_sockaddr_unix(family: u16, path: &[u8]) -> SockaddrUnix {
    let pathlen = path.len();    
    let mut array_path : [u8; 108] = [0; 108];
    array_path[0..pathlen].copy_from_slice(path);
    SockaddrUnix{ sun_family: family, sun_path: array_path }
}

pub fn gen_ud_path() -> String {
    let mut owned_path: String = "/tmp/sock".to_owned();
    unsafe {
        let id = UD_ID_COUNTER.fetch_add(1, Ordering::Relaxed);
        owned_path.push_str(&id.to_string());
    }
    owned_path.clone()
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
    pub refcnt: i32,
    raw_sys_fd: i32
}

impl Socket {
    pub fn new(domain: i32, socktype: i32, protocol: i32) -> Socket {
        let fd = unsafe {libc::socket(domain, socktype, protocol)};
        if fd < 0 {panic!("Socket creation failed when it should never fail");}
        Self {refcnt: 1, raw_sys_fd: fd}
    }

    pub fn bind(&self, addr: &GenSockaddr) -> i32 {
        let (finalsockaddr, addrlen) = match addr {
            GenSockaddr::V6(addrref6) => {((addrref6 as *const SockaddrV6).cast::<libc::sockaddr>(), size_of::<SockaddrV6>())}
            GenSockaddr::V4(addrref) => {((addrref as *const SockaddrV4).cast::<libc::sockaddr>(), size_of::<SockaddrV4>())}
            _ => { unreachable!() }
        };
        unsafe {libc::bind(self.raw_sys_fd, finalsockaddr, addrlen as u32)}
    }

    pub fn connect(&self, addr: &GenSockaddr) -> i32 {
        let (finalsockaddr, addrlen) = match addr {
            GenSockaddr::V6(addrref6) => {((addrref6 as *const SockaddrV6).cast::<libc::sockaddr>(), size_of::<SockaddrV6>())}
            GenSockaddr::V4(addrref) => {((addrref as *const SockaddrV4).cast::<libc::sockaddr>(), size_of::<SockaddrV4>())}
            _ => { unreachable!() }

        };
        unsafe {libc::connect(self.raw_sys_fd, finalsockaddr, addrlen as u32)}
    }

    pub fn sendto(&self, buf: *const u8, len: usize, addr: Option<&GenSockaddr>) -> i32 {
        let (finalsockaddr, addrlen) = match addr {
            Some(GenSockaddr::V6(addrref6)) => {((addrref6 as *const SockaddrV6).cast::<libc::sockaddr>(), size_of::<SockaddrV6>())}
            Some(GenSockaddr::V4(addrref)) => {((addrref as *const SockaddrV4).cast::<libc::sockaddr>(), size_of::<SockaddrV4>())}
            Some(_) => { unreachable!() }
            None => {(std::ptr::null::<libc::sockaddr>() as *const libc::sockaddr, 0)}
        };
        unsafe {libc::sendto(self.raw_sys_fd, buf as *const libc::c_void, len, 0, finalsockaddr, addrlen as u32) as i32}
    }

    pub fn recvfrom(&self, buf: *mut u8, len: usize, addr: &mut Option<&mut GenSockaddr>) -> i32 {
        let (finalsockaddr, mut addrlen) = match addr {
            Some(GenSockaddr::V6(ref mut addrref6)) => {((addrref6 as *mut SockaddrV6).cast::<libc::sockaddr>(), size_of::<SockaddrV6>() as u32)}
            Some(GenSockaddr::V4(ref mut addrref)) => {((addrref as *mut SockaddrV4).cast::<libc::sockaddr>(), size_of::<SockaddrV4>() as u32)}
            Some(_) => { unreachable!() }
            None => {(std::ptr::null::<libc::sockaddr>() as *mut libc::sockaddr, 0)}
        };
        unsafe {libc::recvfrom(self.raw_sys_fd, buf as *mut libc::c_void, len, 0, finalsockaddr, &mut addrlen as *mut u32) as i32}
    }

    pub fn recvfrom_nonblocking(&self, buf: *mut u8, len: usize, addr: &mut Option<&mut GenSockaddr>) -> i32 {
        let (finalsockaddr, mut addrlen) = match addr {
            Some(GenSockaddr::V6(ref mut addrref6)) => {((addrref6 as *mut SockaddrV6).cast::<libc::sockaddr>(), size_of::<SockaddrV6>() as u32)}
            Some(GenSockaddr::V4(ref mut addrref)) => {((addrref as *mut SockaddrV4).cast::<libc::sockaddr>(), size_of::<SockaddrV4>() as u32)}
            Some(_) => { unreachable!() }
            None => {(std::ptr::null::<libc::sockaddr>() as *mut libc::sockaddr, 0)}
        };
        self.set_nonblocking();
        let retval = unsafe {libc::recvfrom(self.raw_sys_fd, buf as *mut libc::c_void, len, 0, finalsockaddr, &mut addrlen as *mut u32) as i32};
        self.set_blocking();
        retval
    }

    pub fn listen(&self, backlog: i32) -> i32 {
        unsafe {libc::listen(self.raw_sys_fd, backlog)}
    }

    pub fn set_blocking(&self) -> i32 {
        unsafe{libc::fcntl(self.raw_sys_fd, libc::F_SETFL, 0)}
    }

    pub fn set_nonblocking(&self) -> i32 {
        unsafe{libc::fcntl(self.raw_sys_fd, libc::F_SETFL, libc::O_NONBLOCK)}
    }

    pub fn accept(&self, isv4: bool) -> (Result<Self, i32>, GenSockaddr) {
        return if isv4 {
            let mut inneraddrbuf = SockaddrV4::default();
            let mut sadlen = size_of::<SockaddrV4>() as u32;
            let newfd = unsafe{libc::accept(self.raw_sys_fd, (&mut inneraddrbuf as *mut SockaddrV4).cast::<libc::sockaddr>(), &mut sadlen as *mut u32)};

            if newfd < 0 {
                (Err(newfd), GenSockaddr::V4(inneraddrbuf))
            } else {
                (Ok(Self{refcnt: 1, raw_sys_fd: newfd}), GenSockaddr::V4(inneraddrbuf))
            }
        } else {
            let mut inneraddrbuf = SockaddrV6::default();
            let mut sadlen = size_of::<SockaddrV6>() as u32;
            let newfd = unsafe{libc::accept(self.raw_sys_fd, (&mut inneraddrbuf as *mut SockaddrV6).cast::<libc::sockaddr>(), &mut sadlen as *mut u32)};

            if newfd < 0 {
                (Err(newfd), GenSockaddr::V6(inneraddrbuf))
            } else {
                (Ok(Self{refcnt: 1, raw_sys_fd: newfd}), GenSockaddr::V6(inneraddrbuf))
            }
        };
    }

    pub fn nonblock_accept(&self, isv4: bool) -> (Result<Self, i32>, GenSockaddr) {
        return if isv4 {
            let mut inneraddrbuf = SockaddrV4::default();
            let mut sadlen = size_of::<SockaddrV4>() as u32;
            self.set_nonblocking();
            let newfd = unsafe{libc::accept(self.raw_sys_fd, (&mut inneraddrbuf as *mut SockaddrV4).cast::<libc::sockaddr>(), &mut sadlen as *mut u32)};
            self.set_blocking();

            if newfd < 0 {
                (Err(newfd), GenSockaddr::V4(inneraddrbuf))
            } else {
                (Ok(Self{refcnt: 1, raw_sys_fd: newfd}), GenSockaddr::V4(inneraddrbuf))
            }
        } else {
            let mut inneraddrbuf = SockaddrV6::default();
            let mut sadlen = size_of::<SockaddrV6>() as u32;
            self.set_nonblocking();
            let newfd = unsafe{libc::accept(self.raw_sys_fd, (&mut inneraddrbuf as *mut SockaddrV6).cast::<libc::sockaddr>(), &mut sadlen as *mut u32)};
            self.set_blocking();

            if newfd < 0 {
                (Err(newfd), GenSockaddr::V6(inneraddrbuf))
            } else {
                (Ok(Self{refcnt: 1, raw_sys_fd: newfd}), GenSockaddr::V6(inneraddrbuf))
            }
        };
    }

    pub fn setsockopt(&self, level: i32, optname: i32, optval: i32) -> i32 {
        let valbuf = optval;
        let sor =  unsafe{libc::setsockopt(self.raw_sys_fd, level, optname, (&valbuf as *const i32).cast::<libc::c_void>(), size_of::<i32>() as u32)};
        sor
    }

    pub fn check_rawconnection(&self) -> bool {
        let mut valbuf = 0;
        let mut len = size_of::<i32>() as u32;
        let ret =  unsafe{libc::getsockopt(self.raw_sys_fd, libc::SOL_SOCKET, libc::SO_ERROR, (&mut valbuf as *mut i32).cast::<libc::c_void>(), &mut len as *mut u32)};
        (ret == 0) && (valbuf == 0) // if return val is 0 and error is 0 it's connected
    }
}

impl Drop for Socket {
    fn drop(&mut self) {
        unsafe { libc::close(self.raw_sys_fd); }
    }
}

pub fn read_netdevs() -> Vec<GenIpaddr> {
    let mut ips = vec!();
    for net_device in read_to_string(NET_DEV_FILENAME).expect("No net_devices file present!").split('\n') {
        if net_device == "" {continue;}
        let genipopt = GenIpaddr::from_string(net_device);
        ips.push(genipopt.expect("Could not parse device ip address from net_devices file"));
    }
    return ips;
}
