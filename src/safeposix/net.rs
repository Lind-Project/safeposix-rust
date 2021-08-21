use crate::interface;
use crate::interface::errnos::{Errno, syscall_error};
use super::syscalls::net_constants::*;
use super::cage::{Cage, FileDescriptor};

pub static NET_METADATA: interface::RustLazyGlobal<interface::RustRfc<interface::RustLock<NetMetadata>>> =
    interface::RustLazyGlobal::new(||
        interface::RustRfc::new(interface::RustLock::new(NetMetadata {
            porttable: interface::RustHashMap::new(),
            used_port_set: interface::RustHashSet::new(),
            listening_port_set: interface::RustHashSet::new(),
            socket_object_table: interface::RustHashMap::new(),
            writersblock_state: interface::RustAtomicBool::new(false)
        }))
    ); //we want to check if fs exists before doing a blank init, but not for now

#[derive(Debug, Hash, Eq, PartialEq)]
pub enum PortType {
    IPv4UDP, IPv4TCP, IPv6UDP, IPv6TCP

}
pub fn mux_port(addr: interface::GenIpaddr, port: u16, domain: i32, istcp: bool) -> (interface::GenIpaddr, u16, PortType) {
    match  domain {
        PF_INET => (addr, port, if istcp {PortType::IPv4TCP} else {PortType::IPv4UDP}),
        PF_INET6  => (addr, port, if istcp {PortType::IPv6TCP} else {PortType::IPv6UDP}),
        _ => panic!("How did you manage to set an unsupported domain on the socket?")
    }
}

pub struct NetMetadata {
    pub porttable: interface::RustHashMap<interface::GenSockaddr, Vec<interface::RustRfc<interface::RustLock<FileDescriptor>>>>,
    pub used_port_set: interface::RustHashSet<(interface::GenIpaddr, u16, PortType)>,
    pub listening_port_set: interface::RustHashSet<(interface::GenIpaddr, u16, PortType)>,
    pub socket_object_table: interface::RustHashMap<i32, interface::Socket>,
    pub writersblock_state: interface::RustAtomicBool
}

//Because other processes on the OS may allocate ephemeral ports, we allocate them from high to
//low whereas the OS allocates them from low to high
const EPHEMERAL_PORT_RANGE_START: u16 = 32768; //sane default on linux
const EPHEMERAL_PORT_RANGE_END: u16 = 60999;
pub const TCPPORT: bool = true;
pub const UDPPORT: bool = false;

impl NetMetadata {
    pub fn _get_available_udp_port(&mut self, addr: interface::GenIpaddr, domain: i32) -> Result<u16, i32> {
        let mut porttuple = mux_port(addr, 0, domain, UDPPORT);
        for port in (EPHEMERAL_PORT_RANGE_START ..= EPHEMERAL_PORT_RANGE_END).rev().map(|x| x.to_be()) {
            porttuple.1 = port;
            if !self.used_port_set.contains(&porttuple) {
                self.used_port_set.insert(porttuple);
                return Ok(port);
            }
        }
        return Err(syscall_error(Errno::EADDRINUSE, "bind", "No available ephemeral port could be found"));
    }
    pub fn _get_available_tcp_port(&mut self, addr: interface::GenIpaddr, domain: i32) -> Result<u16, i32> {
        let mut porttuple = mux_port(addr.clone(), 0, domain, TCPPORT);
        for port in (EPHEMERAL_PORT_RANGE_START ..= EPHEMERAL_PORT_RANGE_END).rev().map(|x| x.to_be()) {
            porttuple.1 = port;
            if !self.used_port_set.contains(&porttuple) {
                self.used_port_set.insert(porttuple);
                return Ok(port);
            }
        }
        return Err(syscall_error(Errno::EADDRINUSE, "bind", "No available ephemeral port could be found"));
    }

    fn get_next_socketobjectid(&mut self) -> Option<i32> {
        for i in MINSOCKOBJID..MAXSOCKOBJID {
            if !self.socket_object_table.contains_key(&i) {
                return Some(i);
            }
        }
        return None;
    }

    pub fn _reserve_localport(&mut self, addr: interface::GenIpaddr, port: u16, protocol: i32, domain: i32) -> Result<u16, i32> {
        if protocol == IPPROTO_UDP {
            if port == 0 {
                self._get_available_udp_port(addr, domain)
            } else if !self.used_port_set.contains(&mux_port(addr.clone(), port, domain, UDPPORT)) {
                self.used_port_set.insert(mux_port(addr, port, domain, UDPPORT));
                Ok(port)
            } else {
                Err(syscall_error(Errno::EADDRINUSE, "bind", "The given address is already in use"))
            }
        } else if protocol == IPPROTO_TCP {
            if port == 0 {
                self._get_available_tcp_port(addr, domain)
            } else if !self.used_port_set.contains(&mux_port(addr.clone(), port, domain, TCPPORT)) {
                self.used_port_set.insert(mux_port(addr, port, domain, TCPPORT));
                Ok(port)
            } else {
                Err(syscall_error(Errno::EADDRINUSE, "bind", "The given address is already in use"))
            }
        } else {
            panic!("Unknown protocol was set on socket somehow");
        }
    }

    pub fn _release_localport(&mut self, addr: interface::GenIpaddr, port: u16, protocol: i32, domain: i32) -> Result<(), i32> {
        if protocol == IPPROTO_TCP && self.used_port_set.remove(&mux_port(addr.clone(), port, domain, TCPPORT)) == true {
            return Ok(());
        }
        else if protocol == IPPROTO_UDP && self.used_port_set.remove(&mux_port(addr.clone(), port, domain, UDPPORT)) == true {
            return Ok(());
        }
        return Err(syscall_error(Errno::EINVAL, "release", "provided port is not being used"));
    }

    pub fn insert_into_socketobjecttable(&mut self, sock: interface::Socket) -> Result<i32, i32> {
        if let Some(id) = self.get_next_socketobjectid() {
            self.socket_object_table.insert(id, sock);
            Ok(id)
        } else {
            Err(syscall_error(Errno::ENFILE, "bind", "The maximum number of sockets for the process have been created"))
        }
    }
}
