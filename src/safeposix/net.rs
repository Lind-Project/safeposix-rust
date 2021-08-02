use crate::interface;
use super::syscalls::net_constants::*;
use super::syscalls::errnos::*;
use super::cage::{Cage, FileDescriptor};

pub static NET_METADATA: interface::RustLazyGlobal<interface::RustRfc<interface::RustLock<NetMetadata>>> =
    interface::RustLazyGlobal::new(||
        interface::RustRfc::new(interface::RustLock::new(NetMetadata {
            porttable: interface::RustHashMap::new(),
            used_udp_port_set: interface::RustHashSet::new(),
            used_tcp_port_set: interface::RustHashSet::new(),
            socket_object_table: interface::RustHashMap::new()

                }))
    ); //we want to check if fs exists before doing a blank init, but not for now

pub struct NetMetadata {
    pub porttable: interface::RustHashMap<interface::RustSockAddr, Vec<interface::RustRfc<interface::RustLock<FileDescriptor>>>>,
    pub used_udp_port_set: interface::RustHashSet<u16>,
    pub used_tcp_port_set: interface::RustHashSet<u16>,
    pub socket_object_table: interface::RustHashMap<i32, GeneralizedSocket>
}

pub enum GeneralizedSocket {
    Udp(interface::RustUdpSocket)
}

const EPHEMERAL_PORT_RANGE_START: u16 = 32768; //sane defaullt on linux
const EPHEMERAL_PORT_RANGE_END: u16 = 60999;

impl NetMetadata {
    fn _get_available_udp_port(&mut self) -> Result<u16, i32> {
        for port in EPHEMERAL_PORT_RANGE_START ..= EPHEMERAL_PORT_RANGE_END {
            if !self.used_udp_port_set.contains(&port) {
                self.used_udp_port_set.insert(port);
                return Ok(port);
            }
        }
        return Err(syscall_error(Errno::EADDRINUSE, "bind", "No available ephemeral port could be found"));
    }
    fn _get_available_tcp_port(&mut self) -> Result<u16, i32> {
        for port in EPHEMERAL_PORT_RANGE_START ..= EPHEMERAL_PORT_RANGE_END {
            if !self.used_tcp_port_set.contains(&port) {
                self.used_tcp_port_set.insert(port);
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

    pub fn _reserve_localport(&mut self, port: u16, protocol: i32) -> Result<u16, i32> {
        if protocol == IPPROTO_UDP {
            if port == 0 {
                self._get_available_udp_port()
            } else if !self.used_udp_port_set.contains(&port) {
                self.used_tcp_port_set.insert(port);
                Ok(port)
            } else {
                Err(syscall_error(Errno::EADDRINUSE, "bind", "The given address is already in use"))
            }
        } else if protocol == IPPROTO_TCP {
            if port == 0 {
                self._get_available_tcp_port()
            } else if !self.used_tcp_port_set.contains(&port) {
                self.used_tcp_port_set.insert(port);
                Ok(port)
            } else {
                Err(syscall_error(Errno::EADDRINUSE, "bind", "The given address is already in use"))
            }
        } else {
            panic!("Unknown protocol was set on socket somehow");
        }
    }

    pub fn insert_into_socketobjecttable(&mut self, sock: GeneralizedSocket) -> Result<i32, i32> {
        if let Some(id) = self.get_next_socketobjectid() {
            self.socket_object_table.insert(id, sock);
            Ok(id)
        } else {
            Err(syscall_error(Errno::ENFILE, "bind", "The maximum number of sockets for the process have been created"))
        }
    }
}
