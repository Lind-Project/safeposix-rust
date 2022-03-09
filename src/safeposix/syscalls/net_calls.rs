#![allow(dead_code)]
// Network related system calls
// outlines and implements all of the networking system calls that are being emulated/faked in Lind

use crate::interface;
use crate::interface::errnos::{Errno, syscall_error};

use super::net_constants::*;
use super::fs_constants::*;
use crate::safeposix::cage::{CAGE_TABLE, Cage, FileDescriptor::*, SocketDesc, EpollDesc, EpollEvent, FdTable, PollStruct};
use crate::safeposix::filesystem::*;
use crate::safeposix::net::*;

impl Cage {
    fn _socket_initializer(&self, domain: i32, socktype: i32, protocol: i32, blocking: bool, cloexec: bool) -> SocketDesc {
        let flags = if blocking {O_NONBLOCK} else {0} | if cloexec {O_CLOEXEC} else {0};

        let sockfd = SocketDesc {
            mode: S_IFSOCK | 0666, //rw-rw-rw- perms, which POSIX does too
            domain: domain,
            socktype: socktype,
            protocol: protocol,
            options: 0, //start with no options set
            sndbuf: 131070, //buffersize, which is only used by getsockopt
            rcvbuf: 262140, //buffersize, which is only used by getsockopt
            state: ConnState::NOTCONNECTED, //we start without a connection
            advlock: interface::RustRfc::new(interface::AdvisoryLock::new()),
            flags: flags,
            errno: 0,
            localaddr: None,
            remoteaddr: None,
            last_peek: interface::RustDeque::new(),
            socketobjectid: None
        };
        return sockfd;
    }

    fn _socket_inserter(&self, sockfd: SocketDesc) -> i32 {
        let mut fdtable = &self.filedescriptortable; 
        let wrappedsock = interface::RustRfc::new(interface::RustLock::new(Socket(sockfd)));

        let newfd = if let Some(fd) = self.get_next_fd(None, Some(&fdtable)) {
            fd
        } else {
            return syscall_error(Errno::ENFILE, "socket or sockpair", "no available file descriptor number could be found");
        };
        fdtable.insert(newfd, wrappedsock);
        newfd
    }

    fn _implicit_bind(&self, sockfdobj: &mut SocketDesc, optaddr: &Option<&mut interface::GenSockaddr>) -> i32 {
        if sockfdobj.localaddr.is_none() {
            let localaddr = match Self::assign_new_addr(sockfdobj, matches!(optaddr, Some(interface::GenSockaddr::V6(_))), sockfdobj.protocol & (1 << SO_REUSEPORT) != 0) {
                Ok(a) => a,
                Err(e) => return e,
            };

            let bindret = self.bind_inner_socket(sockfdobj, &localaddr, true);

            if bindret < 0 {
                match Errno::from_discriminant(interface::get_errno()) {
                    Ok(i) => {return syscall_error(i, "recvfrom", "syscall error from attempting to bind within recvfrom");},
                    Err(()) => panic!("Unknown errno value from socket recvfrom returned!"),
                };
            }
        }
        0
    }

    pub fn socket_syscall(&self, domain: i32, socktype: i32, protocol: i32) -> i32 {
        let real_socktype = socktype & 0x7; //get the type without the extra flags, it's stored in the last 3 bits
        let nonblocking = (socktype & SOCK_NONBLOCK) != 0;
        let cloexec = (socktype & SOCK_CLOEXEC) != 0;

        if nonblocking {
            return syscall_error(Errno::EOPNOTSUPP, "socket", "trying to create a non-blocking socket, which we don't yet support");
        }

        match domain {
            PF_INET => {
                match real_socktype {

                    SOCK_STREAM => {
                        //SOCK_STREAM defaults to TCP for protocol, otherwise protocol is unsupported
                        let newprotocol = if protocol == 0 {IPPROTO_TCP} else {protocol};

                        if newprotocol != IPPROTO_TCP {
                            return syscall_error(Errno::EOPNOTSUPP, "socket", "The only SOCK_STREAM implemented is TCP. Unknown protocol input.");
                        }
                        let sockfdobj = self._socket_initializer(domain, socktype, newprotocol, nonblocking, cloexec);
                        return self._socket_inserter(sockfdobj);

                    }

                    SOCK_DGRAM => {
                        //SOCK_DGRAM defaults to UDP for protocol, otherwise protocol is unsuported
                        let newprotocol = if protocol == 0 {IPPROTO_UDP} else {protocol};

                        if newprotocol != IPPROTO_UDP {
                            return syscall_error(Errno::EOPNOTSUPP, "socket", "The only SOCK_DGRAM implemented is UDP. Unknown protocol input.");
                        }
                        let sockfdobj = self._socket_initializer(domain, socktype, newprotocol, nonblocking, cloexec);
                        return self._socket_inserter(sockfdobj);
                    }

                    _ => {
                        return syscall_error(Errno::EOPNOTSUPP, "socket", "trying to use an unimplemented socket type");
                    }

                }
            }

            _ => {
                return syscall_error(Errno::EOPNOTSUPP, "socket", "trying to use an unimplemented domain");
            }
        }
    }

    //we assume we've converted into a RustSockAddr in the dispatcher
    pub fn bind_syscall(&self, fd: i32, localaddr: &interface::GenSockaddr) -> i32 {
        self.bind_inner(fd, localaddr, false)
    }

    fn bind_inner_socket(&self, sockfdobj: &mut SocketDesc, localaddr: &interface::GenSockaddr, prereserved: bool) -> i32 {
        if localaddr.get_family() != sockfdobj.domain as u16 {
            return syscall_error(Errno::EINVAL, "bind", "An address with an invalid family for the given domain was specified");
        }
        if sockfdobj.localaddr.is_some() {
            return syscall_error(Errno::EINVAL, "bind", "The socket is already bound to an address");
        }

        let mut mutmetadata = NET_METADATA.write().unwrap();
        let intent_to_rebind = sockfdobj.options & (1 << SO_REUSEPORT) != 0;

        let newlocalport = if prereserved {
            localaddr.port()
        } else {
            let localout = mutmetadata._reserve_localport(localaddr.addr(), localaddr.port(), sockfdobj.protocol, sockfdobj.domain, intent_to_rebind);
            if let Err(errnum) = localout {return errnum;}
            localout.unwrap()
        };

        let mut newsockaddr = localaddr.clone();
        newsockaddr.set_port(newlocalport);

        let sid = if let Some(id) = sockfdobj.socketobjectid {
            id
        } else {
            let sock = interface::Socket::new(sockfdobj.domain, sockfdobj.socktype, sockfdobj.protocol);
            let id = mutmetadata.insert_into_socketobjecttable(sock).unwrap();
            sockfdobj.socketobjectid = Some(id);
            id
        } ;
        let locksock = mutmetadata.socket_object_table.get(&sid).unwrap().clone();
        let sockobj = locksock.read().unwrap();
        let bindret = sockobj.bind(&newsockaddr);

        if bindret < 0 {
            match Errno::from_discriminant(interface::get_errno()) {
                Ok(i) => {return syscall_error(i, "sendto", "The libc call to bind failed!");},
                Err(()) => panic!("Unknown errno value from socket bind returned!"),
            };
        }

        sockfdobj.localaddr = Some(newsockaddr);
 
        0
    }

    pub fn bind_inner(&self, fd: i32, localaddr: &interface::GenSockaddr, prereserved: bool) -> i32 {
        let fdtable = &self.filedescriptortable;

        if let Some(wrappedfd) = fdtable.get(&fd) {
            let mut filedesc_enum = wrappedfd.write().unwrap();
            match &mut *filedesc_enum {
                Socket(sockfdobj) => {
                    self.bind_inner_socket(sockfdobj, localaddr, prereserved)
                }
                _ => {
                    syscall_error(Errno::ENOTSOCK, "bind", "file descriptor refers to something other than a socket")
                }
            }
        } else {
            syscall_error(Errno::EBADF, "bind", "invalid file descriptor")
        }
    }

    fn assign_new_addr(sockfdobj: &SocketDesc, isv6: bool, rebindability: bool) -> Result<interface::GenSockaddr, i32> {
        if let Some(addr) = &sockfdobj.localaddr {
            Ok(addr.clone())
        } else {
            let mut mutmetadata = NET_METADATA.write().unwrap();

            //This is the specified behavior for the berkeley sockets API
            let retval = if isv6 {
                let mut newremote = interface::GenSockaddr::V6(interface::SockaddrV6::default());
                let addr = interface::GenIpaddr::V6(interface::V6Addr::default());
                newremote.set_addr(addr);
                newremote.set_family(AF_INET6 as u16);
                newremote.set_port(match mutmetadata._reserve_localport(addr.clone(), 0, sockfdobj.protocol, sockfdobj.domain, rebindability) {
                    Ok(portnum) => portnum,
                    Err(errnum) => return Err(errnum),
                });
                newremote
            } else {
                let mut newremote = interface::GenSockaddr::V4(interface::SockaddrV4::default());
                let addr = interface::GenIpaddr::V4(interface::V4Addr::default());
                newremote.set_addr(addr);
                newremote.set_family(AF_INET as u16);
                newremote.set_port(match mutmetadata._reserve_localport(addr.clone(), 0, sockfdobj.protocol, sockfdobj.domain, rebindability) {
                    Ok(portnum) => portnum,
                    Err(errnum) => return Err(errnum),
                });
                newremote
            };

            Ok(retval)
        }
    }

    pub fn connect_syscall(&self, fd: i32, remoteaddr: &interface::GenSockaddr) -> i32 {
        let fdtable = &self.filedescriptortable;
        if let Some(wrappedfd) = fdtable.get(&fd) {
            let mut filedesc_enum = wrappedfd.write().unwrap();
            match &mut *filedesc_enum {
                Socket(sockfdobj) => {
                    if remoteaddr.get_family() != sockfdobj.domain as u16 {
                        return syscall_error(Errno::EINVAL, "connect", "An address with an invalid family for the given domain was specified");
                    }
                    if sockfdobj.state != ConnState::NOTCONNECTED {
                        return syscall_error(Errno::EISCONN, "connect", "The descriptor is already connected");
                    }

                    //for UDP, just set the addresses and return
                    if sockfdobj.protocol == IPPROTO_UDP {
                        sockfdobj.remoteaddr = Some(remoteaddr.clone());
                        match sockfdobj.localaddr {
                            Some(_) => return 0,
                            None => {
                                let localaddr = match Self::assign_new_addr(sockfdobj, matches!(remoteaddr, interface::GenSockaddr::V6(_)), sockfdobj.protocol & (1 << SO_REUSEPORT) != 0) {
                                    Ok(a) => a,
                                    Err(e) => return e,
                                };

                                //unlock fdtable so that we're fine to call bind_inner
                                return self.bind_inner_socket(sockfdobj, &localaddr, true);
                            }
                        };
                    } else if sockfdobj.protocol == IPPROTO_TCP {
                        //for TCP, actually create the internal socket object and connect it
                        let sid = Self::getsockobjid(&mut *sockfdobj);
                        let locksock = NET_METADATA.read().unwrap().socket_object_table.get(&sid).unwrap().clone();
                        let sockobj = locksock.read().unwrap();
                        if let None = sockfdobj.localaddr {
                            let localaddr = match Self::assign_new_addr(sockfdobj, matches!(remoteaddr, interface::GenSockaddr::V6(_)), sockfdobj.protocol & (1 << SO_REUSEPORT) != 0) {
                                Ok(a) => a,
                                Err(e) => return e,
                            };

                            let bindret = sockobj.bind(&localaddr);
                            if bindret < 0 {
                                match Errno::from_discriminant(interface::get_errno()) {
                                    Ok(i) => {return syscall_error(i, "connect", "The libc call to bind within connect failed");},
                                    Err(()) => panic!("Unknown errno value from socket bind within connect returned!"),
                                };
                            }

                            sockfdobj.localaddr = Some(localaddr);
                        };

                        let connectret = sockobj.connect(remoteaddr);
                        if connectret < 0 {
                            match Errno::from_discriminant(interface::get_errno()) {
                                Ok(i) => {return syscall_error(i, "connect", "The libc call to connect failed!");},
                                Err(()) => panic!("Unknown errno value from socket connect returned!"),
                            };

                        }

                        sockfdobj.remoteaddr = Some(remoteaddr.clone());
                        sockfdobj.state = ConnState::CONNECTED;
                        sockfdobj.errno = 0;
                        return 0;
                    } else {
                        return syscall_error(Errno::EOPNOTSUPP, "connect", "Unkown protocol in connect");
                    }
                }
                _ => {
                    return syscall_error(Errno::ENOTSOCK, "connect", "file descriptor refers to something other than a socket");
                }
            }
        } else {
            return syscall_error(Errno::EBADF, "connect", "invalid file descriptor");
        }
    }

    pub fn getsockobjid(sockfdobj: &mut SocketDesc) -> i32 {
        if let None = sockfdobj.socketobjectid {
            let sock = interface::Socket::new(sockfdobj.domain, sockfdobj.socktype, sockfdobj.protocol);
            sockfdobj.socketobjectid = Some(NET_METADATA.write().unwrap().insert_into_socketobjecttable(sock).unwrap());
        } 
        sockfdobj.socketobjectid.unwrap()
    }

    pub fn sendto_syscall(&self, fd: i32, buf: *const u8, buflen: usize, flags: i32, dest_addr: &interface::GenSockaddr) -> i32 {
        //if ip and port are not specified, shunt off to send
        if dest_addr.port() == 0 && dest_addr.addr().is_unspecified() {
            return self.send_syscall(fd, buf, buflen, flags);
        }

        let fdtable = &self.filedescriptortable;
        if let Some(wrappedfd) = fdtable.get(&fd) {
            let mut filedesc_enum = wrappedfd.write().unwrap();
            match &mut *filedesc_enum {
                Socket(sockfdobj) => {
                    if dest_addr.get_family() != sockfdobj.domain as u16 {
                        return syscall_error(Errno::EINVAL, "sendto", "An address with an invalid family for the given domain was specified");
                    }
                    if (flags & !MSG_NOSIGNAL) != 0 {
                        return syscall_error(Errno::EOPNOTSUPP, "sendto", "The flags are not understood!");
                    }

                    if sockfdobj.state == ConnState::CONNECTED || sockfdobj.state == ConnState::LISTEN {
                        return syscall_error(Errno::EISCONN, "sendto", "The descriptor is connected");
                    }

                    match sockfdobj.protocol {
                        //Sendto doesn't make sense for the TCP protocol, it's connection oriented
                        IPPROTO_TCP => {
                            return syscall_error(Errno::EISCONN, "sendto", "The descriptor is connection-oriented");
                        }

                        IPPROTO_UDP => {
                            let mut tmpdest = *dest_addr;
                            let ibindret = self._implicit_bind(&mut *sockfdobj, &Some(&mut tmpdest));
                            if ibindret < 0 {
                                return ibindret;
                            }

                            let sid = Self::getsockobjid(&mut *sockfdobj);

                            let mutmetadata = NET_METADATA.write().unwrap();
                            let sockobjwrapper = mutmetadata.socket_object_table.get(&sid).unwrap();
                            let sockobj = &*sockobjwrapper.read().unwrap();

                            //we don't mind if this fails for now and we will just get the error
                            //from calling sendto

                            let sockret = sockobj.sendto(buf, buflen, Some(dest_addr));

                            if sockret < 0 {
                                match Errno::from_discriminant(interface::get_errno()) {
                                    Ok(i) => {return syscall_error(i, "sendto", "The libc call to sendto failed!");},
                                    Err(()) => panic!("Unknown errno value from socket sendto returned!"),
                                };
                            } else {
                                return sockret;
                            }
                        }

                        _ => {
                            return syscall_error(Errno::EOPNOTSUPP, "sendto", "Unkown protocol in sendto");
                        }
                    }
                }

                _ => {
                    return syscall_error(Errno::ENOTSOCK, "sendto", "file descriptor refers to something other than a socket");
                }
            }
        } else {
            return syscall_error(Errno::EBADF, "sendto", "invalid file descriptor");
        }
    }
    pub fn send_syscall(&self, fd: i32, buf: *const u8, buflen: usize, flags: i32) -> i32 {
        let fdtable = &self.filedescriptortable;
        if let Some(wrappedfd) = fdtable.get(&fd) {
            let mut filedesc_enum = wrappedfd.write().unwrap();
            match &mut *filedesc_enum {
                Socket(sockfdobj) => {
                    if (flags & !MSG_NOSIGNAL) != 0 {
                        return syscall_error(Errno::EOPNOTSUPP, "send", "The flags are not understood!");
                    }

                    match sockfdobj.protocol {
                        IPPROTO_TCP => {
                            if sockfdobj.state != ConnState::CONNECTED {
                                return syscall_error(Errno::ENOTCONN, "send", "The descriptor is not connected");
                            }

                            let sid = Self::getsockobjid(&mut *sockfdobj);
                            let metadata = NET_METADATA.read().unwrap();
                            let sockobjwrapper = metadata.socket_object_table.get(&sid).unwrap();
                            let sockobj = &*sockobjwrapper.read().unwrap();

                            let retval = sockobj.sendto(buf, buflen, None);
                            if retval < 0 {
                                match Errno::from_discriminant(interface::get_errno()) {
                                    Ok(i) => {return syscall_error(i, "send", "The libc call to sendto failed!");},
                                    Err(()) => panic!("Unknown errno value from socket sendto returned!"),
                                };
                            } else {
                                return retval;
                            }
                        }

                        IPPROTO_UDP => {
                            let remoteaddr = match &sockfdobj.remoteaddr {
                                Some(x) => x.clone(),
                                None => {return syscall_error(Errno::ENOTCONN, "send", "The descriptor is not connected");},
                            };

                            //drop fdtable lock so as not to deadlock, this should not introduce
                            //any harmful race conditions
                            drop(filedesc_enum);
                            drop(fdtable);
                            //send from a udp socket is just shunted off to sendto with the remote address set
                            return self.sendto_syscall(fd, buf, buflen, flags, &remoteaddr);
                        }

                        _ => {
                            return syscall_error(Errno::EOPNOTSUPP, "send", "Unkown protocol in send");
                        }
                    }
                }
                _ => {
                    return syscall_error(Errno::ENOTSOCK, "send", "file descriptor refers to something other than a socket");
                }
            }
        } else {
            return syscall_error(Errno::EBADF, "send", "invalid file descriptor");
        }
    }

    pub fn recv_common(&self, fd: i32, buf: *mut u8, buflen: usize, flags: i32, addr: &mut Option<&mut interface::GenSockaddr>, fdtable: &FdTable) -> i32 {
        if let Some(wrappedfd) = fdtable.get(&fd) {
            let clonedfd = wrappedfd.clone();
            let mut filedesc_enum = clonedfd.write().unwrap();
            match &mut *filedesc_enum {
                Socket(ref mut sockfdobj) => {
                    match sockfdobj.protocol {
                        IPPROTO_TCP => {
                            if sockfdobj.state != ConnState::CONNECTED {
                                return syscall_error(Errno::ENOTCONN, "recvfrom", "The descriptor is not connected");
                            }
                            let sid = Self::getsockobjid(&mut *sockfdobj);
                            let locksock = NET_METADATA.read().unwrap().socket_object_table.get(&sid).unwrap().clone();
                            let sockobj = locksock.read().unwrap();

                            let mut newbuflen = buflen;
                            let mut newbufptr = buf;

                            //if we have peeked some data before, fill our buffer with that data before moving on
                            if !sockfdobj.last_peek.is_empty() {
                                let bytecount = interface::rust_min(sockfdobj.last_peek.len(), newbuflen);
                                interface::copy_fromrustdeque_sized(buf, bytecount, &sockfdobj.last_peek);
                                newbuflen -= bytecount;
                                newbufptr = newbufptr.wrapping_add(bytecount);

                                //if we're not still peeking data, consume the data we peeked from our peek buffer
                                //and if the bytecount is more than the length of the peeked data, then we remove the entire
                                //buffer
                                if flags & MSG_PEEK == 0 {
                                    sockfdobj.last_peek.drain(..(
                                        if bytecount > sockfdobj.last_peek.len() {sockfdobj.last_peek.len()} 
                                        else {bytecount}
                                    ));
                                }

                                if newbuflen == 0 {
                                    //if we've filled all of the buffer with peeked data, return
                                    return bytecount as i32;
                                }
                            }

                            let bufleft = newbufptr;
                            let buflenleft = newbuflen;

                            drop(fd);
                            drop(fdtable);
                            let retval = sockobj.recvfrom(bufleft, buflenleft, addr);

                            if retval < 0 {
                                //If we have already read from a peek but have failed to read more, exit!
                                if buflen != buflenleft {
                                    return (buflen - buflenleft) as i32;
                                }

                                match Errno::from_discriminant(interface::get_errno()) {
                                    Ok(i) => {return syscall_error(i, "recvfrom", "Internal call to recvfrom failed");},
                                    Err(()) => panic!("Unknown errno value from socket recvfrom returned!"),
                                };

                            }

                            let totalbyteswritten = (buflen - buflenleft) as i32 + retval;

                            if flags & MSG_PEEK != 0 {
                                //extend from the point after we read our previously peeked bytes
                                interface::extend_fromptr_sized(newbufptr, retval as usize, &mut sockfdobj.last_peek);
                            }

                            return totalbyteswritten;

                        }
                        IPPROTO_UDP => {
                            let ibindret = self._implicit_bind(&mut *sockfdobj, addr);
                            if ibindret < 0 {
                                return ibindret;
                            }

                            let sid = Self::getsockobjid(&mut *sockfdobj);
                            let locksock = NET_METADATA.read().unwrap().socket_object_table.get(&sid).unwrap().clone();
                            let sockobj = locksock.read().unwrap();

                            //if the remoteaddr is set and addr is not, use remoteaddr
                            let retval = if addr.is_none() && sockfdobj.remoteaddr.is_some() {
                                sockobj.recvfrom(buf, buflen, &mut sockfdobj.remoteaddr.as_mut())
                            } else {
                                sockobj.recvfrom(buf, buflen, addr)
                            };

                            if retval < 0 {
                                match Errno::from_discriminant(interface::get_errno()) {
                                    Ok(i) => {return syscall_error(i, "recvfrom", "syscall error from libc recvfrom");},
                                    Err(()) => panic!("Unknown errno value from socket recvfrom returned!"),
                                };
                                
                            } else {
                                return retval;
                            }
                        }

                        _ => {
                            return syscall_error(Errno::EOPNOTSUPP, "recvfrom", "Unkown protocol in recvfrom");
                        }
                    }
                }

                _ => {
                    return syscall_error(Errno::ENOTSOCK, "recvfrom", "file descriptor refers to something other than a socket");
                }
            }
        } else {
            return syscall_error(Errno::EBADF, "recvfrom", "invalid file descriptor");
        }
    }

    pub fn recvfrom_syscall(&self, fd: i32, buf: *mut u8, buflen: usize, flags: i32, addr: &mut Option<&mut interface::GenSockaddr>) -> i32 {
        let fdtable = &self.filedescriptortable;
        return self.recv_common(fd, buf, buflen, flags, addr, fdtable);
    }

    pub fn recv_syscall(&self, fd: i32, buf: *mut u8, buflen: usize, flags: i32) -> i32 {
        let fdtable = &self.filedescriptortable;
        return self.recv_common(fd, buf, buflen, flags, &mut None, fdtable);
    }

    //we currently ignore backlog
    pub fn listen_syscall(&self, fd: i32, _backlog: i32) -> i32 {
        let fdtable = &self.filedescriptortable;
        if let Some(wrappedfd) = fdtable.get(&fd) {
            let mut filedesc_enum = wrappedfd.write().unwrap();

            match &mut *filedesc_enum {
                Socket(sockfdobj) => {
                    match sockfdobj.state {
                        ConnState::LISTEN => {
                            return 0; //Already done!
                        }

                        ConnState::CONNECTED => {
                            return syscall_error(Errno::EOPNOTSUPP, "listen", "We don't support closing a prior socket connection on listen");
                        }

                        ConnState::NOTCONNECTED => {
                            if sockfdobj.protocol != IPPROTO_TCP {
                                return syscall_error(Errno::EOPNOTSUPP, "listen", "This protocol doesn't support listening");
                            }
                            let mut ladr;
                            let mut porttuple;
                            match sockfdobj.localaddr {
                                Some(sla) => {
                                    let mut mutmetadata = NET_METADATA.write().unwrap();
                                    ladr = sla.clone();
                                    porttuple = mux_port(ladr.addr().clone(), ladr.port(), sockfdobj.domain, TCPPORT);

                                    if mutmetadata.listening_port_set.contains(&porttuple) {
                                        match mutmetadata._get_available_tcp_port(ladr.addr().clone(), sockfdobj.domain, sockfdobj.options & (1 << SO_REUSEPORT) != 0) {
                                            Ok(port) => ladr.set_port(port),
                                            Err(i) => return i,
                                        }
                                        porttuple = mux_port(ladr.addr().clone(), ladr.port(), sockfdobj.domain, TCPPORT);
                                    }
                                }
                                None => {
                                    ladr = match Self::assign_new_addr(sockfdobj, sockfdobj.domain == AF_INET6, sockfdobj.protocol & (1 << SO_REUSEPORT) != 0) {
                                        Ok(a) => a,
                                        Err(e) => return e,
                                    };
                                    porttuple = mux_port(ladr.addr().clone(), ladr.port(), sockfdobj.domain, TCPPORT);
                                }
                            }
                            //get or create the socket and bind it before listening
                            let sid = Self::getsockobjid(sockfdobj);

                            let mut mutmetadata = NET_METADATA.write().unwrap();
                            mutmetadata.listening_port_set.insert(porttuple);

                            sockfdobj.state = ConnState::LISTEN;

                            let locksock = mutmetadata.socket_object_table.get(&sid).unwrap().clone();
                            let sockobj = locksock.read().unwrap();
                            if let None = sockfdobj.localaddr {
                                let bindret = sockobj.bind(&ladr);
                                if bindret < 0 {
                                    match Errno::from_discriminant(interface::get_errno()) {
                                        Ok(i) => {return syscall_error(i, "listen", "The libc call to bind within listen failed");},
                                        Err(()) => panic!("Unknown errno value from socket bind within listen returned!"),
                                    };
                                }
                            }
                            let listenret = sockobj.listen(5); //default backlog in repy for whatever reason, we replicate it
                            if listenret < 0 {
                                let lr = match Errno::from_discriminant(interface::get_errno()) {
                                    Ok(i) => syscall_error(i, "listen", "The libc call to listen failed!"),
                                    Err(()) => panic!("Unknown errno value from socket listen returned!"),
                                };
                                mutmetadata.listening_port_set.remove(&mux_port(ladr.addr().clone(), ladr.port(), sockfdobj.domain, TCPPORT));
                                sockfdobj.state = ConnState::CONNECTED;
                                return lr;
                            };
                            return 0;
                        }
                    }
                }

                _ => {
                    return syscall_error(Errno::ENOTSOCK, "listen", "file descriptor refers to something other than a socket");
                }
            }
        } else {
            return syscall_error(Errno::EBADF, "listen", "invalid file descriptor");
        }
    }

    pub fn netshutdown_syscall(&self, fd: i32, how: i32) -> i32 {
        let mut fdtable = &self.filedescriptortable;
        match how {
            SHUT_RD => {
                return syscall_error(Errno::EOPNOTSUPP, "netshutdown", "partial shutdown read is not implemented");
            }
            SHUT_WR => {
                return Self::_cleanup_socket(self, fd, true, fdtable);
            }
            SHUT_RDWR => {
                return Self::_cleanup_socket(self, fd, false, fdtable);
            }
            _ => {
                //See http://linux.die.net/man/2/shutdown for nuance to this error
                return syscall_error(Errno::EINVAL, "netshutdown", "the shutdown how argument passed is not supported");
            }
        }
    }

    pub fn _cleanup_socket(&self, fd: i32, partial: bool, fdtable: &FdTable) -> i32 {

        //The FdTable must always be passed.

        if let Some(wrappedfd) = fdtable.get(&fd) {
            let mut filedesc = wrappedfd.write().unwrap();
            if let Socket(sockfdobj) = &mut *filedesc {
                let mut mutmetadata = NET_METADATA.write().unwrap();
                let objectid = &sockfdobj.socketobjectid;

                if let Some(localaddr) = sockfdobj.localaddr.as_ref().clone() {
                    let release_ret_val = mutmetadata._release_localport(localaddr.addr(), localaddr.port(), sockfdobj.protocol, sockfdobj.domain);
                    if let Err(e) = release_ret_val {return e;}
                }
                if !partial {
                    if let None = objectid {} else {
                        mutmetadata.socket_object_table.remove(&objectid.unwrap());
                    }
                    sockfdobj.state = ConnState::NOTCONNECTED;
                }
            } else {return syscall_error(Errno::ENOTSOCK, "cleanup socket", "file descriptor is not a socket");}
        } else {
            return syscall_error(Errno::EBADF, "cleanup socket", "invalid file descriptor");
        }

        //We have to take this out of the match because the fdtable already has a mutable borrow
        //which means that I can't change it with a remove until after the fdtable mutable borrow is finished from the match statement
        //I know it is a bit confusing, but there isn't really another way to do this
        if !partial {
            fdtable.remove(&fd); 
        }
        return 0;
    }


    
    //calls accept on the socket object with value depending on ipv4 or ipv6
    pub fn accept_syscall(&self, fd: i32, addr: &mut interface::GenSockaddr) -> i32 {

        let fdtable = &self.filedescriptortable;
        if let Some(wrappedfd) = fdtable.get(&fd) {
            let unwrapclone = wrappedfd.clone();
            let mut filedesc_enum = unwrapclone.write().unwrap();
            match &mut *filedesc_enum {
                Socket(ref mut sockfdobj) => {
                    match sockfdobj.protocol {
                        IPPROTO_UDP => {
                            return syscall_error(Errno::EOPNOTSUPP, "accept", "Protocol does not support listening");
                        }
                        IPPROTO_TCP => {
                            if sockfdobj.state != ConnState::LISTEN {
                                return syscall_error(Errno::EINVAL, "accept", "Socket must be listening before accept is called");
                            }

                            let newfd = if let Some(fd) = self.get_next_fd(None, Some(&fdtable)) {
                                fd
                            } else {
                                return syscall_error(Errno::ENFILE, "accept", "no available file descriptor number could be found");
                            };

                            let mut mutmetadata = NET_METADATA.write().unwrap();
                            let (acceptedresult, remote_addr) = if let Some(mut vec) = mutmetadata.pending_conn_table.get_mut(&sockfdobj.localaddr.unwrap().port()) {
                                //if we got a pending connection in select/poll/whatever, return that here instead
                                let tup = vec.pop().unwrap(); //pending connection tuple recieved
                                if vec.is_empty() {
                                    mutmetadata.pending_conn_table.remove(&sockfdobj.localaddr.unwrap().port()); //remove port from pending conn table if no more pending conns exist for it
                                }
                                drop(fdtable);
                                drop(mutmetadata);
                                tup
                            } else {
                                let sid = Self::getsockobjid(&mut *sockfdobj);
                                let locksock = mutmetadata.socket_object_table.get(&sid).unwrap().clone();
                                let sockobj = locksock.read().unwrap();

                                drop(fdtable);
                                drop(mutmetadata);

                                match sockfdobj.domain {
                                    PF_INET => sockobj.accept(true),
                                    PF_INET6 => sockobj.accept(false),
                                    _ => panic!("Unknown domain in accepting socket"),
                                }
                            };

                            if let Err(_) = acceptedresult {
                                match Errno::from_discriminant(interface::get_errno()) {
                                    Ok(e) => {return syscall_error(e, "accept", "host system accept call failed");},
                                    Err(()) => panic!("Unknown errno value from socket send returned!"),
                                };
                            }

                            let acceptedsock = acceptedresult.unwrap();

                            //create new connected socket
                            let mut newsockobj = self._socket_initializer(sockfdobj.domain, sockfdobj.socktype, sockfdobj.protocol, false, false);
                            newsockobj.state = ConnState::CONNECTED;

                            let mut newaddr = sockfdobj.localaddr.clone().unwrap();
                            mutmetadata = NET_METADATA.write().unwrap();
                            let newport = match mutmetadata._reserve_localport(newaddr.addr(), 0, sockfdobj.protocol, sockfdobj.domain, false) {
                                Ok(portnum) => portnum,
                                Err(errnum) => return errnum,
                            };
                            newaddr.set_port(newport);

                            let newipaddr = newaddr.addr().clone();
                            newsockobj.localaddr = Some(newaddr);
                            newsockobj.remoteaddr = Some(remote_addr.clone());

                            //create socket object for new connected socket
                            newsockobj.socketobjectid = match mutmetadata.insert_into_socketobjecttable(acceptedsock) {
                                Ok(id) => Some(id),
                                Err(errnum) => {
                                    mutmetadata.listening_port_set.remove(&mux_port(newipaddr.clone(), newport, sockfdobj.domain, TCPPORT));
                                    return errnum;
                                }
                            };
                            drop(mutmetadata);

                            *addr = remote_addr; //populate addr with what address it connected to
                            let _domain = sockfdobj.domain;

                            //socket inserter code
                            let wrappedsock = interface::RustRfc::new(interface::RustLock::new(Socket(newsockobj)));

                            self.filedescriptortable.insert(newfd, wrappedsock);
                            
                            return newfd;
                        }
                        _ => {
                            return syscall_error(Errno::EOPNOTSUPP, "accept", "Unkown protocol in accept");
                        }
                    }
                }
                _ => {
                    return syscall_error(Errno::ENOTSOCK, "listen", "file descriptor refers to something other than a socket");
                }
            }
        } else {
            return syscall_error(Errno::EBADF, "listen", "invalid file descriptor");
        }
    }

    fn _nonblock_peek_read(&self, fd: i32, fdtable: &FdTable) -> bool{
        let flags = O_NONBLOCK | MSG_PEEK;
        let mut buf = [0u8; 1];
        let bufptr = buf.as_mut_ptr();
        let retval = self.recv_common(fd, bufptr, 1, flags, &mut None, fdtable);
        return retval >= 0; //it it's less than 0, it failed, it it's 0 peer is dead, 1 it succeeded, in the latter 2 it's true
    }

    //TODO: handle pipes
    pub fn select_syscall(&self, nfds: i32, readfds: &mut interface::RustHashSet<i32>, writefds: &mut interface::RustHashSet<i32>, exceptfds: &mut interface::RustHashSet<i32>, timeout: Option<interface::RustDuration>) -> i32 {
        //sockfds and writefds are not really implemented at the current moment.
        //They both always return success. However we have some intention of making
        //writefds work at some point for pipes? We have no such intention for exceptfds
        let mut new_readfds = interface::RustHashSet::<i32>::new();
        let mut new_writefds = interface::RustHashSet::<i32>::new();
        //let mut new_exceptfds = interface::RustHashSet::<i32>::new(); we don't ever support exceptional conditions
    
        if nfds < STARTINGFD || nfds >= MAXFD {
            return syscall_error(Errno::EINVAL, "select", "Number of FDs is wrong");
        }
    
        let start_time = interface::starttimer();
    
        let end_time = match timeout {
            Some(time) => time,
            None => interface::RustDuration::MAX
        };
    
        let mut retval = 0;
    
        loop { //we must block manually
            for fd in readfds.iter() {
                let fdtable = &self.filedescriptortable;
                if let Some(wrappedfd) = fdtable.get(&fd) {
                    let mut filedesc_enum = wrappedfd.write().unwrap();

                    match &mut *filedesc_enum {
                        Socket(ref mut sockfdobj) => {
                            if sockfdobj.state == ConnState::LISTEN {
                                let mut mutmetadata = NET_METADATA.write().unwrap();

                                if !mutmetadata.pending_conn_table.contains_key(&sockfdobj.localaddr.unwrap().port()) {
                                    let sid = Self::getsockobjid(&mut *sockfdobj);
                                    let locksock = mutmetadata.socket_object_table.get(&sid).unwrap().clone();
                                    let sockobj = locksock.read().unwrap();

                                    let listeningsocket = match sockfdobj.domain {
                                        PF_INET => sockobj.nonblock_accept(true),
                                        PF_INET6 => sockobj.nonblock_accept(false),
                                        _ => panic!("Unknown domain in accepting socket"),
                                    };
                                    drop(sockobj);
                                    if let Ok(_) = listeningsocket.0 {
                                        //save the pending connection for accept to do something with it
                                        mutmetadata.pending_conn_table.insert(sockfdobj.localaddr.unwrap().port(), vec!(listeningsocket));
                                    } else {
                                        //if it returned an error, then don't insert it into new_readfds
                                        continue;
                                    }
                                } //if it's already got a pending connection, add it!

                                //if we reach here there is a pending connection
                                new_readfds.insert(*fd);
                                retval += 1;
                            } else {
                                if sockfdobj.protocol == IPPROTO_UDP {
                                    new_readfds.insert(*fd);
                                    retval += 1;
                                } else {
                                    drop(sockfdobj);
                                    drop(filedesc_enum);
                                    if self._nonblock_peek_read(*fd, fdtable) {
                                        new_readfds.insert(*fd);
                                        retval += 1;
                                    }
                                }
                            }
                        }

                        //we don't support selecting streams
                        Stream(_) => {continue;}

                        //not supported yet
                        Pipe(_) => {
                            new_readfds.insert(*fd);
                            retval += 1;
                        }

                        //these file reads never block
                        _ => {
                            new_readfds.insert(*fd);
                            retval += 1;
                        }
                    }
                } else {
                    return syscall_error(Errno::EBADF, "select", "invalid file descriptor");
                }
            }

            let fdtable = &self.filedescriptortable;
            for fd in writefds.iter() {
                if let Some(wrappedfd) = fdtable.get(&fd) {
                    let mut filedesc_enum = wrappedfd.write().unwrap();
                    match &mut *filedesc_enum {
                        //we always say sockets are writable? Even though this is not true
                        Socket(_) => {
                            new_writefds.insert(*fd);
                            retval += 1;
                        }

                        //we always say streams are writable?
                        Stream(_) => {
                            new_writefds.insert(*fd);
                            retval += 1;
                        }

                        //not supported yet
                        Pipe(_) => {
                            new_writefds.insert(*fd);
                            retval += 1;
                        }

                        //these file writes never block
                        _ => {
                            new_writefds.insert(*fd);
                            retval += 1;
                        }
                    }
                } else {
                    return syscall_error(Errno::EBADF, "select", "invalid file descriptor");
                }

            }
            
            for fd in exceptfds.iter() {
                //we say none of them ever have exceptional conditions
                if !fdtable.contains_key(&fd) {
                    return syscall_error(Errno::EBADF, "select", "invalid file descriptor");
                }
            }

            if retval != 0 || interface::readtimer(start_time) > end_time {
                break;
            } else {
                interface::sleep(BLOCK_TIME);
            }
        }
        *readfds = new_readfds;
        *writefds = new_writefds;
        return retval;
    }

    pub fn getsockopt_syscall(&self, fd: i32, level: i32, optname: i32, optval: &mut i32) -> i32 {
        let fdtable = &self.filedescriptortable;
        
        if let Some(wrappedfd) = fdtable.get(&fd) {
            let mut filedesc = wrappedfd.write().unwrap();
            if let Socket(sockfdobj) = &mut *filedesc {
                //checking that we recieved SOL_SOCKET\
                match level {
                    SOL_UDP => {
                        return syscall_error(Errno::EOPNOTSUPP, "getsockopt", "UDP is not supported for getsockopt");
                    }
                    SOL_TCP => {
                        return syscall_error(Errno::EOPNOTSUPP, "getsockopt", "TCP options not remembered by getsockopt");
                    }
                    SOL_SOCKET => {
                        let optbit = 1 << optname;
                        match optname {
                            //indicate whether we are accepting connections or not in the moment
                            SO_ACCEPTCONN => {
                                if sockfdobj.state == ConnState::LISTEN {
                                    *optval = 1;
                                } else {
                                    *optval = 0;
                                }
                            }
                            //if the option is a stored binary option, just return it...
                            SO_LINGER | SO_KEEPALIVE | SO_SNDLOWAT | SO_RCVLOWAT | SO_REUSEPORT | SO_REUSEADDR => {
                                if sockfdobj.options & optbit == optbit {
                                    *optval = 1;
                                } else {
                                    *optval = 0;
                                }
                            }
                            //handling the ignored buffer settings:
                            SO_SNDBUF => {
                                *optval = sockfdobj.sndbuf;
                            }
                            SO_RCVBUF => {
                                *optval = sockfdobj.rcvbuf;
                            }
                            //returning the type if asked
                            SO_TYPE => {
                                *optval = sockfdobj.socktype;
                            }
                            //should always be true
                            SO_OOBINLINE => {
                                *optval = 1;
                            }
                            SO_ERROR => {
                                let tmp = sockfdobj.errno;
                                sockfdobj.errno = 0;
                                *optval = tmp;
                            }
                            _ => {
                                return syscall_error(Errno::EOPNOTSUPP, "getsockopt", "unknown optname passed into syscall");
                            }
                        }
                    }
                    _ => {
                        return syscall_error(Errno::EOPNOTSUPP, "getsockopt", "unknown level passed into syscall");
                    }
                }
            } else {
                return syscall_error(Errno::ENOTSOCK, "getsockopt", "the provided file descriptor is not a socket");
            }
        } else {
            return syscall_error(Errno::EBADF, "getsockopt", "the provided file descriptor is invalid");
        }
        return 0;
    }

    pub fn setsockopt_syscall(&self, fd: i32, level: i32, optname: i32, optval: i32) -> i32 {
        let fdtable = &self.filedescriptortable;
        
        if let Some(wrappedfd) = fdtable.get(&fd) {
            let mut filedesc = wrappedfd.write().unwrap();
            if let Socket(sockfdobj) = &mut *filedesc {
                //checking that we recieved SOL_SOCKET\
                match level {
                    SOL_UDP => {
                        return syscall_error(Errno::EOPNOTSUPP, "getsockopt", "UDP is not supported for getsockopt");
                    }
                    SOL_TCP => {
                        if optname == TCP_NODELAY {
                            return 0;
                        }
                        return syscall_error(Errno::EOPNOTSUPP, "getsockopt", "TCP options not remembered by getsockopt");
                    }
                    SOL_SOCKET => {
                        let optbit = 1 << optname;
                        match optname {
                            SO_ACCEPTCONN | SO_TYPE | SO_SNDLOWAT | SO_RCVLOWAT => {
                                let error_string = format!("Cannot set option using setsockopt. {}", optname);
                                return syscall_error(Errno::ENOPROTOOPT, "setsockopt", &error_string);
                            }
                            SO_LINGER | SO_KEEPALIVE => {
                                if optval == 0 {
                                    sockfdobj.options &= !optbit;
                                } else {
                                    //optval should always be 1 or 0.
                                    sockfdobj.options |= optbit;
                                }


                                return 0;
                            }

                            SO_REUSEPORT | SO_REUSEADDR => {
                                let mut newoptions = sockfdobj.options;
                                //now let's set this if we were told to
                                if optval != 0 {
                                    //optval should always be 1 or 0.
                                    newoptions |= optbit;
                                } else {
                                    newoptions &= !optbit;
                                }

                                if newoptions != sockfdobj.options {
                                    let sid = Self::getsockobjid(&mut *sockfdobj);
                                    let locksock = NET_METADATA.read().unwrap().socket_object_table.get(&sid).unwrap().clone();
                                    let sockobj = locksock.read().unwrap();

                                    let sockoptret = sockobj.setsockopt(SOL_SOCKET, optname, optval);
                                    if sockoptret < 0 {
                                        match Errno::from_discriminant(interface::get_errno()) {
                                            Ok(i) => {return syscall_error(i, "setsockopt", "The libc call to setsockopt failed!");},
                                            Err(()) => panic!("Unknown errno value from setsockopt returned!"),
                                        };
                                    }
                                }

                                sockfdobj.options = newoptions;

                                return 0;
                            }
                            SO_SNDBUF => {
                                sockfdobj.sndbuf = optval;
                                return 0;
                            }
                            SO_RCVBUF => {
                                sockfdobj.rcvbuf = optval;
                                return 0;
                            }
                            //should always be one -- can only handle it being 1
                            SO_OOBINLINE => {
                                if optval != 1 {
                                    return syscall_error(Errno::EOPNOTSUPP, "getsockopt", "does not support OOBINLINE being set to anything but 1");
                                }
                                return 0;
                            }
                            _ => {
                                return syscall_error(Errno::EOPNOTSUPP, "getsockopt", "unknown optname passed into syscall");
                            }
                        }
                    }
                    _ => {
                        return syscall_error(Errno::EOPNOTSUPP, "getsockopt", "unknown level passed into syscall");
                    }
                }
            } else {
                return syscall_error(Errno::ENOTSOCK, "getsockopt", "the provided file descriptor is not a socket");
            }
        } else {
            return syscall_error(Errno::EBADF, "getsockopt", "the provided file descriptor is invalid");
        }
    }

    pub fn getpeername_syscall(&self, fd: i32, ret_addr: &mut interface::GenSockaddr) -> i32 {
        let fdtable = &self.filedescriptortable;

        if let Some(wrappedfd) = fdtable.get(&fd) {
            let filedesc = wrappedfd.read().unwrap();
            if let Socket(sockfdobj) = &*filedesc {
                //if the socket is not connected, then we should return an error
                if sockfdobj.remoteaddr == None {
                    return syscall_error(Errno::ENOTCONN, "getpeername", "the socket is not connected");
                }
                
                //all of the checks that we had have passed if we are here
                *ret_addr = sockfdobj.remoteaddr.unwrap();
                return 0;

            } else {
                return syscall_error(Errno::ENOTSOCK, "getpeername", "the provided file is not a socket");
            }
        } else {
            return syscall_error(Errno::EBADF, "getpeername", "the provided file descriptor is not valid");
        }
    }

    pub fn getsockname_syscall(&self, fd: i32, ret_addr: &mut interface::GenSockaddr) -> i32 {
        let fdtable = &self.filedescriptortable;

        if let Some(wrappedfd) = fdtable.get(&fd) {
            let filedesc = wrappedfd.read().unwrap();
            if let Socket(sockfdobj) = &*filedesc {
                if sockfdobj.localaddr == None {
                    
                    //sets the address to 0.0.0.0 if the address is not initialized yet
                    //setting the family as well based on the domain
                    let addr = match sockfdobj.domain {
                        AF_INET => { interface::GenIpaddr::V4(interface::V4Addr::default()) }
                        AF_INET6 => { interface::GenIpaddr::V6(interface::V6Addr::default()) }
                        _ => { unreachable!() }
                    };
                    ret_addr.set_addr(addr);
                    ret_addr.set_port(0);
                    ret_addr.set_family(sockfdobj.domain as u16);
                    return 0;
                }
 
                //if the socket is not none, then return the socket
                *ret_addr = sockfdobj.localaddr.unwrap();
                return 0;

            } else {
                return syscall_error(Errno::ENOTSOCK, "getsockname", "the provided file is not a socket");
            }
        } else {
            return syscall_error(Errno::EBADF, "getsockname", "the provided file descriptor is not valid");
        }
    }

    //we only return the default host name because we do not allow for the user to change the host name right now
    pub fn gethostname_syscall(&self, address_ptr: *mut u8, length: isize) -> i32 {
        if length < 0 {
            return syscall_error(Errno::EINVAL, "gethostname_syscall", "provided length argument is invalid");
        }

        let mut bytes: Vec<u8> = DEFAULT_HOSTNAME.as_bytes().to_vec();
        bytes.push(0u8); //Adding a null terminator to the end of the string
        let name_length = bytes.len();
        
        let mut len = name_length;
        if (length as usize) < len {
            len = length as usize;
        }

        interface::fill(address_ptr, len, &bytes);

        return 0;
    }

    pub fn poll_syscall(&self, fds: &mut [PollStruct], timeout: Option<interface::RustDuration>) -> i32 { //timeout is supposed to be in milliseconds

        let mut return_code: i32 = 0;
        let start_time = interface::starttimer();

        let end_time = match timeout {
            Some(time) => time,
            None => interface::RustDuration::MAX
        };

        loop {
            for structpoll in &mut *fds {
                let fd = structpoll.fd;
                let events = structpoll.events;

                let mut reads = interface::RustHashSet::<i32>::new();
                let mut writes = interface::RustHashSet::<i32>::new();
                let mut errors = interface::RustHashSet::<i32>::new();

                //read
                if events & POLLIN > 0 {reads.insert(fd);}
                //write
                if events & POLLOUT > 0 {writes.insert(fd);}
                //err
                if events & POLLERR > 0 {errors.insert(fd);}

                let mut mask: i16 = 0;

                //0 essentially sets the timeout to the max value allowed (which is almost always more than enough time)
                if Self::select_syscall(&self, fd, &mut reads, &mut writes, &mut errors, Some(interface::RustDuration::ZERO)) > 0 {
                    mask += if !reads.is_empty() {POLLIN} else {0};
                    mask += if !writes.is_empty() {POLLOUT} else {0};
                    mask += if !errors.is_empty() {POLLERR} else {0};
                    return_code += 1;
                }
                structpoll.revents = mask;
            }

            if return_code != 0 || interface::readtimer(start_time) > end_time {
                break;
            } else {
                interface::sleep(BLOCK_TIME);
            }
        }
        return return_code;
    }

    pub fn _epoll_object_allocator(&self) -> i32 {

        //seems to only be called in functions that don't have a filedesctable lock, so not passing the lock.
        let mut fdtable = &self.filedescriptortable;
        
        //get a file descriptor
        if let Some(newfd) = self.get_next_fd(None, None) {
            //new epoll fd
            let epollobjfd = EpollDesc {
                mode: 0000,
                registered_fds: interface::RustHashMap::<i32, EpollEvent>::new(),
                advlock: interface::RustRfc::new(interface::AdvisoryLock::new()),
                errno: 0,
                flags: 0
            };
            
            //insert into the fdtable and return the fd i32
            let wrappedsock = interface::RustRfc::new(interface::RustLock::new(Epoll(epollobjfd)));
            fdtable.insert(newfd, wrappedsock);
            return newfd;
        } else {
            return syscall_error(Errno::ENFILE, "epoll create", "no available file descriptor number could be found");
        }
    }

    pub fn epoll_create_syscall(&self, size: i32) -> i32 {
        if size <= 0 {
            return syscall_error(Errno::EINVAL, "epoll create", "provided size argument is invalid");
        }
        return Self::_epoll_object_allocator(self);
    }

    //this one can still be optimized
    pub fn epoll_ctl_syscall(&self, epfd: i32, op: i32, fd: i32, event: &EpollEvent) -> i32 {

        let fdtable = &self.filedescriptortable;

        //making sure that the epfd is really an epoll fd
        if let Some(wrappedfd) = fdtable.get(&epfd) {
            let mut filedesc_enum_epollfd = wrappedfd.write().unwrap();
            if let Epoll(epollfdobj) = &mut *filedesc_enum_epollfd {

                //check if the other fd is an epoll or not...
                if let Epoll(_) = &*fdtable.get(&fd).unwrap().read().unwrap() {
                    return syscall_error(Errno::EBADF, "epoll ctl", "provided fd is not a valid file descriptor")
                }

                //now that we know that the types are all good...
                match op {
                    EPOLL_CTL_DEL => {
                        //since remove returns the value at the key and the values will always be EpollEvents, 
                        //I am using this to optimize the code
                        if let EpollEvent{ events: _, fd: _ } = epollfdobj.registered_fds.remove(&fd).unwrap().1 {} else {
                            return syscall_error(Errno::ENOENT, "epoll ctl", "fd is not registered with this epfd");
                        }
                    }
                    EPOLL_CTL_MOD => {
                        //check if the fd that we are modifying exists or not
                        if let Some(_) = epollfdobj.registered_fds.get(&fd) {} else {
                            return syscall_error(Errno::ENOENT, "epoll ctl", "fd is not registered with this epfd");
                        }
                        //if the fd already exists, insert overwrites the prev entry
                        epollfdobj.registered_fds.insert(fd, EpollEvent { events: event.events, fd: event.fd });
                    }
                    EPOLL_CTL_ADD => {
                        if let Some(_) = epollfdobj.registered_fds.get(&fd) {
                            return syscall_error(Errno::EEXIST, "epoll ctl", "fd is already registered");
                        }
                        epollfdobj.registered_fds.insert(fd, EpollEvent { events: event.events, fd: event.fd });
                    }
                    _ => {
                        return syscall_error(Errno::EINVAL, "epoll ctl", "provided op is invalid");
                    }
                }
            } else {
                return syscall_error(Errno::EBADF, "epoll ctl", "provided fd is not a valid file descriptor")
            }
        } else {
            return syscall_error(Errno::EBADF, "epoll ctl", "provided epoll fd is not a valid epoll file descriptor");
        }
        return 0;
    }

    pub fn epoll_wait_syscall(&self, epfd: i32, events: &mut [EpollEvent], maxevents: i32, timeout: Option<interface::RustDuration>) -> i32 {

        let fdtable = &self.filedescriptortable;

        if let Some(wrappedfd) = fdtable.get(&epfd) {
            let filedesc_enum = wrappedfd.write().unwrap();
            if let Epoll(epollfdobj) = &*filedesc_enum {
                if !maxevents > 0 {
                    return syscall_error(Errno::EINVAL, "epoll wait", "max events argument is not a positive number");
                }

                let mut poll_fds_vec: Vec<PollStruct> = vec![];

                for (key, value) in epollfdobj.registered_fds.iter() {

                    let events = value.events;
                    let mut structpoll = PollStruct {
                        fd: key,
                        events: 0,
                        revents: 0
                    };
                    if events & EPOLLIN as u32 > 0 {
                        structpoll.events |= POLLIN;
                    }
                    if events & EPOLLOUT as u32 > 0 {
                        structpoll.events |= POLLOUT;
                    }
                    if events & EPOLLERR as u32 > 0 {
                        structpoll.events |= POLLERR;
                    }
                    poll_fds_vec.push(structpoll);
                }

                let poll_fds_slice = &mut poll_fds_vec[..];
                Self::poll_syscall(&self, poll_fds_slice, timeout);
                let mut count_changed: i32 = 0;

                for (count, result) in poll_fds_slice[..maxevents as usize].iter().enumerate() {
                    let mut event = EpollEvent{ events: 0, fd: epollfdobj.registered_fds.get(&result.fd).unwrap().fd};
                    if result.revents & POLLIN > 0 {
                        event.events |= EPOLLIN as u32;
                    }
                    if result.revents & POLLOUT > 0 {
                        event.events |= EPOLLOUT as u32;
                    }
                    if result.revents & POLLERR > 0 {
                        event.events |= EPOLLERR as u32;
                    }
                    events[count] = event;
                    count_changed += 1;
                }
                return count_changed;
            } else {
                return syscall_error(Errno::EINVAL, "epoll wait", "provided fd is not an epoll file descriptor");
            }
        } else {
            return syscall_error(Errno::EBADF, "epoll wait", "provided fd is not a valid file descriptor");
        }
    }

    // Because socketpair needs to spawn off a helper thread to connect the two ends of the socket pair, and because that helper thread,
    // along with the main thread, need to access the cage to call methods (syscalls) of it, and because rust's threading model states that
    // any reference passed into a thread but not moved into it mut have a static lifetime, we cannot use a standard member function to perform
    // this syscall, and must use an arc wrapped cage instead as a "this" parameter in lieu of self
    pub fn socketpair_syscall(this: interface::RustRfc<Cage>, domain: i32, socktype: i32, protocol: i32, sv: &mut interface::SockPair) -> i32 {
        let newdomain = if domain == AF_UNIX {AF_INET} else {domain};
        let sock1fd = this.socket_syscall(newdomain, socktype, protocol);
        if sock1fd < 0 {return sock1fd;}
        let sock2fd = this.socket_syscall(newdomain, socktype, protocol);
        if sock2fd < 0 {
            this.close_syscall(sock1fd);
            return sock2fd;
        }
    
        let portlessaddr = if newdomain == AF_INET {
            let ipaddr = interface::V4Addr {s_addr: u32::from_ne_bytes([127, 0, 0, 1])};
            let innersockaddr = interface::SockaddrV4{sin_family: newdomain as u16, sin_addr: ipaddr, sin_port: 0, padding: 0};
            interface::GenSockaddr::V4(innersockaddr)
        } else if domain == AF_INET6 {
            let ipaddr = interface::V6Addr {s6_addr: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1]};
            let innersockaddr = interface::SockaddrV6{sin6_family: newdomain as u16, sin6_addr: ipaddr, sin6_port: 0, sin6_flowinfo: 0, sin6_scope_id: 0};
            interface::GenSockaddr::V6(innersockaddr)
        } else {
            unreachable!();
        };
    
        if socktype == SOCK_STREAM {
            let bindret = this.bind_inner(sock1fd, &portlessaddr, false); //len assigned arbitrarily large value
            if bindret != 0 {
                this.close_syscall(sock1fd);
                this.close_syscall(sock2fd);
                return bindret;
            }

            let mut bound_addr = portlessaddr.clone();
            this.getsockname_syscall(sock1fd, &mut bound_addr);
    
            let listenret = this.listen_syscall(sock1fd, 1);
            if listenret != 0 {
                this.close_syscall(sock1fd);
                this.close_syscall(sock2fd);
                return listenret;
            }
    
            let mut garbage_remote = portlessaddr.clone();
            let thishandle2 = this.clone();
            
            let acceptor = interface::helper_thread(move || {
                let accret = thishandle2.accept_syscall(sock1fd, &mut garbage_remote);
                if accret < 0 {
                    let sockerrno = match Errno::from_discriminant(interface::get_errno()) {
                        Ok(i) => i,
                        Err(()) => panic!("Unknown errno value from accept within socketpair returned!"),
                    };
                    return Err(syscall_error(sockerrno, "socketpair", "The libc call to accept within socketpair failed!"));
                }
                thishandle2.close_syscall(sock1fd);
                return Ok(accret);
            });
    
            let connret = this.connect_syscall(sock2fd, &bound_addr);
            if connret < 0 {
                let sockerrno = match Errno::from_discriminant(interface::get_errno()) {
                    Ok(i) => i,
                    Err(()) => panic!("Unknown errno value from connect within socketpair returned!"),
                };
                let _ = acceptor.join().unwrap(); //make sure to synchronize threads, assigned to _ to get rid of unused result warning
                this.close_syscall(sock1fd);
                this.close_syscall(sock2fd);
                return syscall_error(sockerrno, "socketpair", "The libc call to connect within socketpair failed!");
            }
    
            let fullaccres = acceptor.join().unwrap(); //unwrap to assume the thread did not die
            //the error is handled in the parent thread to make sure both threads are synchronized when erroring out (i.e. closes)
            let otherfd = match fullaccres {
                Ok(fd) => fd,
                Err(syserr) => {
                    this.close_syscall(sock1fd);
                    this.close_syscall(sock2fd);
                    return syserr;
                }
            };
            sv.sock1 = sock2fd;
            sv.sock2 = otherfd;
        } else if socktype == SOCK_DGRAM {
            let bind1ret = this.bind_inner(sock1fd, &portlessaddr, false); //arbitrarily large length given
            if bind1ret < 0 {
                this.close_syscall(sock1fd);
                this.close_syscall(sock2fd);
                return bind1ret;
            }
    
            let bind2ret = this.bind_inner(sock2fd, &portlessaddr, false); //arbitrarily large length given
            if bind2ret < 0 {
                this.close_syscall(sock1fd);
                this.close_syscall(sock2fd);
                return bind2ret;
            }

            let mut bound1addr = portlessaddr.clone();
            let mut bound2addr = portlessaddr.clone();
            this.getsockname_syscall(sock1fd, &mut bound1addr);
            this.getsockname_syscall(sock2fd, &mut bound2addr);
    
            let conn1ret = this.connect_syscall(sock1fd, &bound2addr);
            if conn1ret < 0 {
                this.close_syscall(sock1fd);
                this.close_syscall(sock2fd);
                return conn1ret;
            }
    
            let conn2ret = this.connect_syscall(sock2fd, &bound1addr);
            if conn2ret < 0 {
                this.close_syscall(sock1fd);
                this.close_syscall(sock2fd);
                return conn2ret;
            }
            sv.sock1 = sock1fd;
            sv.sock2 = sock2fd;
        } else {
            unreachable!();
        }
        return 0;
    }
}
