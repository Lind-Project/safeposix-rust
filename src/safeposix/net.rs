use crate::interface;
use crate::interface::errnos::{Errno, syscall_error};
use super::syscalls::net_constants::*;
use super::cage::{Cage, FileDescriptor};

//Because other processes on the OS may allocate ephemeral ports, we allocate them from high to
//low whereas the OS allocates them from low to high
//Additionally, we can't tell whether a port is truly rebindable, this is because even when a port
//is closed sometimes there still is cleanup that the OS needs to do (for ephemeral ports which end
//up in the TIME_WAIT state). Therefore, we will assign ephemeral ports rather than simply from the
//highest available one, in a cyclic fashion skipping over unavailable ports. While this still may
//cause issues if specific port adresses in the ephemeral port range are allocated and closed before
//an ephemeral port would be bound there, it is much less likely that this will happen and is easy
//to avoid and nonstandard in user programs. See the code for _get_available_udp_port and its tcp
//counterpart for the implementation details.
const EPHEMERAL_PORT_RANGE_START: u16 = 32768; //sane default on linux
const EPHEMERAL_PORT_RANGE_END: u16 = 60999;
pub const TCPPORT: bool = true;
pub const UDPPORT: bool = false;

pub static NET_METADATA: interface::RustLazyGlobal<interface::RustRfc<NetMetadata>> =
    interface::RustLazyGlobal::new(||
        interface::RustRfc::new(NetMetadata {
            used_port_set: interface::RustHashMap::new(),
            next_ephemeral_port_tcpv4: interface::RustRfc::new(interface::RustLock::new(EPHEMERAL_PORT_RANGE_END)),
            next_ephemeral_port_udpv4: interface::RustRfc::new(interface::RustLock::new(EPHEMERAL_PORT_RANGE_END)),
            next_ephemeral_port_tcpv6: interface::RustRfc::new(interface::RustLock::new(EPHEMERAL_PORT_RANGE_END)),
            next_ephemeral_port_udpv6: interface::RustRfc::new(interface::RustLock::new(EPHEMERAL_PORT_RANGE_END)),
            listening_port_set: interface::RustHashSet::new(),
            socket_object_table: interface::RustHashMap::new(),
            domain_socket_table: interface::RustHashMap::new(),
            pending_conn_table: interface::RustHashMap::new(),
        })
    ); //we want to check if fs exists before doing a blank init, but not for now

//A list of all network devices present on the machine
//It is populated from a file that should be present prior to running rustposix, see
//the implementation of read_netdevs for specifics
pub static NET_DEVICES_LIST: interface::RustLazyGlobal<Vec<interface::GenIpaddr>> = interface::RustLazyGlobal::new(|| interface::read_netdevs());

#[derive(Debug, Hash, Eq, PartialEq, Clone)]
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
    pub used_port_set: interface::RustHashMap<(u16, PortType), Vec<(interface::GenIpaddr, u32)>>, //maps port tuple to whether rebinding is allowed: 0 means there's a user but rebinding is not allowed, positive number means that many users, rebinding is allowed
    next_ephemeral_port_tcpv4: interface::RustRfc<interface::RustLock<u16>>,
    next_ephemeral_port_udpv4: interface::RustRfc<interface::RustLock<u16>>,
    next_ephemeral_port_tcpv6: interface::RustRfc<interface::RustLock<u16>>,
    next_ephemeral_port_udpv6: interface::RustRfc<interface::RustLock<u16>>,
    pub listening_port_set: interface::RustHashSet<(interface::GenIpaddr, u16, PortType)>,
    pub socket_object_table: interface::RustHashMap<i32, interface::RustRfc<interface::RustLock<interface::Socket>>>,
    pub domain_socket_table: interface::RustHashMap<usize, interface::GenSockaddr>,
    pub pending_conn_table: interface::RustHashMap<u16, Vec<(Result<interface::Socket, i32>, interface::GenSockaddr)>>
}

impl NetMetadata {
    fn initialize_port(&self, tup: &(interface::GenIpaddr, u16, PortType), rebindability: u32) -> bool {
        let used_port_tup = (tup.1, tup.2.clone());
        if tup.0.is_unspecified() {
            let tupclone = used_port_tup.clone();
            let entry = self.used_port_set.entry(tupclone.clone());
            match entry {
                interface::RustHashEntry::Occupied(_) => {
                    return false;
                }
                interface::RustHashEntry::Vacant(v) => {
                    let mut intervec = vec!();
                    for interface_addr in &*NET_DEVICES_LIST {
                        intervec.push((interface_addr.clone(), rebindability));
                    }
                    v.insert(intervec);
                }
            }
            true
        } else {
            match self.used_port_set.entry(used_port_tup) {
                interface::RustHashEntry::Occupied(mut o) => {
                    let addrsused = o.get_mut();
                    for addrtup in addrsused.clone() {
                        if addrtup.0 == tup.0 {
                            return false;
                        }
                    }
                    addrsused.push((tup.0.clone(), rebindability));
                }
                interface::RustHashEntry::Vacant(v) => {
                    v.insert(vec![(tup.0.clone(), rebindability)]);
                }
            }
            true
        }
    }
    pub fn _get_available_udp_port(&self, addr: interface::GenIpaddr, domain: i32, rebindability: bool) -> Result<u16, i32> {
        if !NET_DEVICES_LIST.contains(&addr) {
            return Err(syscall_error(Errno::EADDRNOTAVAIL, "bind", "Specified network device is not set up for lind or does not exist!"));
        }
        let mut porttuple = mux_port(addr, 0, domain, UDPPORT);

        //start from the starting location we specified in a previous attempt to get an ephemeral port
        let mut next_ephemeral = if domain == AF_INET {
            self.next_ephemeral_port_udpv4.write()
        } else if domain == AF_INET6 {
            self.next_ephemeral_port_udpv6.write()
        } else {unreachable!()};
        for range in [(EPHEMERAL_PORT_RANGE_START ..= *next_ephemeral), (*next_ephemeral + 1 ..= EPHEMERAL_PORT_RANGE_END)] {
            for ne_port in range.rev() {
                let port = ne_port.to_be(); //ports are stored in network endian order
                porttuple.1 = port;

                //if we think we can bind to this port
                if self.initialize_port(&porttuple, if rebindability {1} else {0}) {//rebindability of 0 means not rebindable, 1 means it's rebindable and there's 1 bound to it
                    *next_ephemeral -= 1;
                    if *next_ephemeral < EPHEMERAL_PORT_RANGE_START {
                        *next_ephemeral = EPHEMERAL_PORT_RANGE_END;
                    }
                    return Ok(port);
                }
            }
        }
        return Err(syscall_error(Errno::EADDRINUSE, "bind", "No available ephemeral port could be found"));
    }
    pub fn _get_available_tcp_port(&self, addr: interface::GenIpaddr, domain: i32, rebindability: bool) -> Result<u16, i32> {
        if !NET_DEVICES_LIST.contains(&addr) {
            return Err(syscall_error(Errno::EADDRNOTAVAIL, "bind", "Specified network device is not set up for lind or does not exist!"));
        }
        let mut porttuple = mux_port(addr.clone(), 0, domain, TCPPORT);

        //start from the starting location we specified in a previous attempt to get an ephemeral port
        let mut next_ephemeral = if domain == AF_INET {
            self.next_ephemeral_port_tcpv4.write()
        } else if domain == AF_INET6 {
            self.next_ephemeral_port_tcpv6.write()
        } else {unreachable!()};
        for range in [(EPHEMERAL_PORT_RANGE_START ..= *next_ephemeral), (*next_ephemeral + 1 ..= EPHEMERAL_PORT_RANGE_END)] {
            for ne_port in range.rev() {
                let port = ne_port.to_be(); //ports are stored in network endian order
                porttuple.1 = port;

                if self.initialize_port(&porttuple, if rebindability {1} else {0}) { //rebindability of 0 means not rebindable, 1 means it's rebindable and there's 1 bound to it

                    *next_ephemeral -= 1;
                    if *next_ephemeral < EPHEMERAL_PORT_RANGE_START {
                        *next_ephemeral = EPHEMERAL_PORT_RANGE_END;
                    }

                    return Ok(port);
                }
            }
        }
        return Err(syscall_error(Errno::EADDRINUSE, "bind", "No available ephemeral port could be found"));
    }

    fn insert_to_next_socketobjectid(&self, sock: interface::Socket) -> Option<i32> {
        for i in MINSOCKOBJID..MAXSOCKOBJID {
            if let interface::RustHashEntry::Vacant(v) = self.socket_object_table.entry(i) {
                v.insert(interface::RustRfc::new(interface::RustLock::new(sock)));
                return Some(i);
            }
        }
        return None;
    }

    pub fn _reserve_localport(&self, addr: interface::GenIpaddr, port: u16, protocol: i32, domain: i32, rebindability: bool) -> Result<u16, i32> {
        if !NET_DEVICES_LIST.contains(&addr) {
            return Err(syscall_error(Errno::EADDRNOTAVAIL, "bind", "Specified network device is not set up for lind or does not exist!"));
        }

        let muxed;
        if protocol == IPPROTO_UDP {
            if port == 0 {
                return self._get_available_udp_port(addr, domain, rebindability); //assign ephemeral port
            } else {
                muxed = mux_port(addr, port, domain, UDPPORT);
            }
        } else if protocol == IPPROTO_TCP {
            if port == 0 {
                return self._get_available_tcp_port(addr, domain, rebindability); //assign ephemeral port
            } else {
                muxed = mux_port(addr, port, domain, TCPPORT);
            }
        } else {
            panic!("Unknown protocol was set on socket somehow");
        }

        let usedport_muxed = (muxed.1, muxed.2);
        let entry = self.used_port_set.entry(usedport_muxed);
        match entry {
            interface::RustHashEntry::Occupied(mut userentry) => {
                for portuser in userentry.get_mut() {
                    if portuser.0 == muxed.0 {
                        if portuser.1 == 0 {
                            return Err(syscall_error(Errno::EADDRINUSE, "reserve port", "port is already in use"));
                        } else {
                            portuser.1 += 1;
                        }
                        break;
                    }
                }
            }
            interface::RustHashEntry::Vacant(v) => {
                v.insert(vec![(muxed.0.clone(), if rebindability {1} else {0})]);
            }
        }
        Ok(port)
    }

    pub fn _release_localport(&self, addr: interface::GenIpaddr, port: u16, protocol: i32, domain: i32) -> Result<(), i32> {
        if !NET_DEVICES_LIST.contains(&addr) {
            return Err(syscall_error(Errno::EADDRNOTAVAIL, "bind", "Specified network device is not set up for lind or does not exist!"));
        }

        let muxed;
        if protocol == IPPROTO_TCP { 
            muxed = mux_port(addr.clone(), port, domain, TCPPORT);
        } else if protocol == IPPROTO_UDP {
            muxed = mux_port(addr.clone(), port, domain, UDPPORT);
        } else {
            return Err(syscall_error(Errno::EINVAL, "release", "provided port has nonsensical protocol"));
        }

        let usedport_muxed = (muxed.1, muxed.2);
        let entry = self.used_port_set.entry(usedport_muxed);
        match entry {
            interface::RustHashEntry::Occupied(mut userentry) => {
                let mut index = 0;
                let userarr = userentry.get_mut();
                for portuser in userarr.clone() {
                    if portuser.0 == muxed.0 {
                        //if it's rebindable and we're removing the last bound port or it's just not rebindable
                        if portuser.1 <= 1 {
                            drop(portuser);
                            if userarr.len() == 1 {
                                drop(userarr);
                                userentry.remove();
                            } else {
                                userarr.swap_remove(index);
                            }
                        } else { //if it's rebindable and there are others bound to it
                            userarr[index].1 -= 1;
                        }
                        return Ok(());
                    }
                    index += 1;
                }
                unreachable!();
            }
            interface::RustHashEntry::Vacant(_) => {
                return Err(syscall_error(Errno::EINVAL, "release", "provided port is not being used"));
            }
        }
    }

    pub fn insert_into_socketobjecttable(&self, sock: interface::Socket) -> Result<i32, i32> {
        if let Some(id) = self.insert_to_next_socketobjectid(sock) {
            Ok(id)
        } else {
            Err(syscall_error(Errno::ENFILE, "bind", "The maximum number of sockets for the process have been created"))
        }
    }
}
