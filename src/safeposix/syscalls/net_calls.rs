#![allow(dead_code)]
// Network related system calls
// Authors: Jonathan Singer and Tristan Brigham
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
            advlock: interface::AdvisoryLock::new(),
            pendingconnections: Vec::new(),
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
        let mut fdtable = self.filedescriptortable.write().unwrap(); 
        let wrappedsock = interface::RustRfc::new(interface::RustLock::new(Socket(sockfd)));

        let newfd = if let Some(fd) = self.get_next_fd(None, Some(&fdtable)) {
            fd
        } else {
            return syscall_error(Errno::ENFILE, "socket or sockpair", "no available file descriptor number could be found");
        };
        fdtable.insert(newfd, wrappedsock);
        newfd
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
        let fdtable = self.filedescriptortable.read().unwrap();

        if let Some(wrappedfd) = fdtable.get(&fd) {
            let mut filedesc_enum = wrappedfd.write().unwrap();
            match &mut *filedesc_enum {

                Socket(sockfdobj) => {
                    if localaddr.get_family() != sockfdobj.domain as u16 {
                        return syscall_error(Errno::EINVAL, "bind", "An address with an invalid family for the given domain was specified");
                    }
                    if sockfdobj.localaddr.is_some() {
                        return syscall_error(Errno::EINVAL, "bind", "The socket is already bound to an address");
                    }

                    let mut mutmetadata = NET_METADATA.write().unwrap();
                    let mut intent_to_rebind = false;

                    //check that nobody else is bound to this address, but if they are, attempt to rebind
                    if let Some(fds_on_port) = mutmetadata.porttable.get(&localaddr) {
                        for otherfd_wrapped in fds_on_port {
                            let otherfd_enum = otherfd_wrapped.read().unwrap();
                            if let Socket(othersockfdobj) = &*otherfd_enum {

                                if othersockfdobj.domain == sockfdobj.domain && 
                                   othersockfdobj.socktype == sockfdobj.socktype &&
                                   othersockfdobj.protocol == sockfdobj.protocol {
                                    if (sockfdobj.options & othersockfdobj.options & SO_REUSEPORT) == SO_REUSEPORT {
                                        intent_to_rebind = true;
                                    } else {
                                        return syscall_error(Errno::EADDRINUSE, "bind", "Another socket is already bound to this addr");
                                    }
                                }

                            } else {
                                panic!("For some reason a non-socket fd was in the port table!");
                            }
                        }
                    }

                    //if we're trying to rebind, we should probably figure out just how multiple interfaces work
                    let newlocalport = if !intent_to_rebind {
                        let localout = mutmetadata._reserve_localport(localaddr.addr(), localaddr.port(), sockfdobj.protocol, sockfdobj.domain);
                        if let Err(errnum) = localout {return errnum;}
                        localout.unwrap()
                    } else {
                        localaddr.port()
                    };

                    let mut newsockaddr = localaddr.clone();
                    newsockaddr.set_port(newlocalport);

                    //we don't actually want/need to create the socket object now, that is done in listen or in connect or whatever
                    mutmetadata.porttable.entry(newsockaddr.clone()).or_insert(vec!()).push(wrappedfd.clone());

                    sockfdobj.localaddr = Some(newsockaddr);
                    0
                }

                _ => {
                    return syscall_error(Errno::ENOTSOCK, "bind", "file descriptor refers to something other than a socket");
                }
            }
        } else {
            return syscall_error(Errno::EBADF, "bind", "invalid file descriptor");
        }
    }

    fn assign_new_addr(sockfdobj: &SocketDesc, isv6: bool) -> Result<interface::GenSockaddr, i32> {
        if let Some(addr) = &sockfdobj.localaddr {
            Ok(addr.clone())
        } else {
            let mut mutmetadata = NET_METADATA.write().unwrap();

            //in lieu of getmyip we just always use 0.0.0.0 or the ipv6 equivalent because we have
            //no kernel-respecting way of accessing the actual interface addresses for ipv6 for now
            //(netlink for now is a big no go)
            let retval = if isv6 {
                let mut newremote = interface::GenSockaddr::V6(interface::SockaddrV6::default());
                let addr = interface::GenIpaddr::V6(interface::V6Addr::default());
                let port = mutmetadata._get_available_tcp_port(addr.clone(), sockfdobj.domain);
                if let Err(e) = port {return Err(e);}
                newremote.set_port(port.unwrap());
                newremote.set_addr(addr);
                newremote.set_family(AF_INET6 as u16);
                newremote
            } else {
                let mut newremote = interface::GenSockaddr::V4(interface::SockaddrV4::default());
                let addr = interface::GenIpaddr::V4(interface::V4Addr::default());
                let port = mutmetadata._get_available_tcp_port(addr.clone(), sockfdobj.domain);
                if let Err(e) = port {return Err(e);}
                newremote.set_port(port.unwrap());
                newremote.set_addr(addr);
                newremote.set_family(AF_INET as u16);
                newremote
            };

            Ok(retval)
        }
    }

    pub fn connect_syscall(&self, fd: i32, remoteaddr: &interface::GenSockaddr) -> i32 {
        let fdtable = self.filedescriptortable.read().unwrap();
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
                                let localaddr = match Self::assign_new_addr(sockfdobj, matches!(remoteaddr, interface::GenSockaddr::V6(_))) {
                                    Ok(a) => a,
                                    Err(e) => return e,
                                };

                                //unlock fdtable so that we're fine to call bind_syscall
                                drop(filedesc_enum);
                                drop(fdtable);

                                return self.bind_syscall(fd, &localaddr); //len assigned arbitrarily large value
                            }
                        };
                    } else if sockfdobj.protocol == IPPROTO_TCP {
                        //for TCP, actually create the internal socket object and connect it
                        let localaddr = match Self::assign_new_addr(sockfdobj, matches!(remoteaddr, interface::GenSockaddr::V6(_))) {
                            Ok(a) => a,
                            Err(e) => return e,
                        };

                        let tcpsockobj = interface::Socket::new(sockfdobj.domain, sockfdobj.socktype, sockfdobj.protocol);
                        let connectret = tcpsockobj.connect(remoteaddr);
                        if connectret < 0 {
                            let sockerrno = match Errno::from_discriminant(-connectret) {
                                Ok(i) => i,
                                Err(()) => panic!("Unknown errno value from socket connect returned!"),
                            };

                            //error is always -1
                            return syscall_error(Errno::ECONNREFUSED, "connect", "The libc call to connect failed!");
                        }

                        //openconnection to get newsockobj
                        sockfdobj.socketobjectid = Some(NET_METADATA.write().unwrap().insert_into_socketobjecttable(tcpsockobj).unwrap());
                        sockfdobj.localaddr = Some(localaddr);
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

        let fdtable = self.filedescriptortable.read().unwrap();
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
                            let sid = Self::getsockobjid(&mut *sockfdobj);

                            //bind the udp port as we do not bind them at bind_syscall, and this is
                            //the last possible moment to do so
                            let localaddr = match Self::assign_new_addr(sockfdobj, matches!(dest_addr, interface::GenSockaddr::V6(_))) {
                                Ok(a) => a,
                                Err(e) => return e,
                            };
                            let mut mutmetadata = NET_METADATA.write().unwrap();
                            let sockobj = mutmetadata.socket_object_table.get(&sid).unwrap();

                            let bindret = sockobj.bind(&localaddr);
                            if bindret < 0 {
                                return syscall_error(Errno::ECONNREFUSED, "sendto", "The libc call to bind failed!");
                            }
                            if let None = sockfdobj.localaddr {
                                sockfdobj.localaddr = Some(localaddr);
                            }
                            //we don't mind if this fails for now and we will just get the error
                            //from calling sendto

                            let mut bufleft = buf;
                            let mut buflenleft = buflen;

                            loop {
                                let sockret = sockobj.sendto(buf, buflen, Some(dest_addr)); //all our sockets are nonblocking so we block manually

                                if sockret >= 0 {
                                    //if our socket succeeds in a partial send that means we
                                    //assume it's blocking until it completes the whole send
                                    buflenleft -= sockret as usize;
                                    if buflenleft == 0 {
                                        mutmetadata.writersblock_state.store(false, interface::RustAtomicOrdering::Relaxed);
                                        return sockret;
                                    }

                                    bufleft = bufleft.wrapping_offset(sockret as isize);
                                    mutmetadata.writersblock_state.store(true, interface::RustAtomicOrdering::Relaxed);

                                    //we've only done a partial send, retry
                                    continue;
                                } else {
                                    let sockerrno = match Errno::from_discriminant(-sockret) {
                                        Ok(i) => i,
                                        Err(()) => panic!("Unknown errno value from socket send returned!"),
                                    };

                                    if sockerrno == Errno::EAGAIN {
                                        mutmetadata.writersblock_state.store(true, interface::RustAtomicOrdering::Relaxed);
                                        interface::sleep(BLOCK_TIME);
                                        continue;
                                    };

                                    mutmetadata.writersblock_state.store(false, interface::RustAtomicOrdering::Relaxed);
                                    //if we fail but have already sent stuff to the socket, return that
                                    if buflenleft != buflen {
                                        return (buflen - buflenleft) as i32; //partial write amount
                                    }

                                    return syscall_error(sockerrno, "sendto", "The libc call to sendto failed!");
                                }
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
        let fdtable = self.filedescriptortable.read().unwrap();
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
                            let sockobj = metadata.socket_object_table.get(&sid).unwrap();

                            let mut bufleft = buf;
                            let mut buflenleft = buflen;
                            loop {
                                let retval = sockobj.sendto(buf, buflen, None); //nonblocking, so we manually block

                                if retval < 0 {
                                    let sockerrno = match Errno::from_discriminant(-retval) {
                                        Ok(i) => i,
                                        Err(()) => panic!("Unknown errno value from socket send returned!"),
                                    };

                                    if sockerrno == Errno::EAGAIN {
                                        metadata.writersblock_state.store(true, interface::RustAtomicOrdering::Relaxed);
                                        interface::sleep(BLOCK_TIME);
                                        continue;
                                    }

                                    metadata.writersblock_state.store(false, interface::RustAtomicOrdering::Relaxed);
                                    //if we fail but have already sent stuff to the socket, return that
                                    if buflenleft != buflen {
                                        return (buflen - buflenleft) as i32; //partial write amount
                                    }

                                    return syscall_error(sockerrno, "send", "The libc call to sendto failed!");
                                } else {
                                    buflenleft -= retval as usize;
                                    if buflenleft == 0 {
                                        metadata.writersblock_state.store(false, interface::RustAtomicOrdering::Relaxed);
                                        return retval;
                                    }

                                    //we've only done a partial send, retry
                                    bufleft = bufleft.wrapping_offset(retval as isize);
                                    metadata.writersblock_state.store(true, interface::RustAtomicOrdering::Relaxed);
                                    continue;
                                }
                            }
                        }

                        IPPROTO_UDP => {
                            let remoteaddr = match &sockfdobj.remoteaddr {
                                Some(x) => x.clone(),
                                None => return syscall_error(Errno::ENOTCONN, "send", "The descriptor is not connected"),
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

    fn recv_common(&self, fd: i32, buf: *mut u8, buflen: usize, flags: i32, addr: &mut Option<&mut interface::GenSockaddr>, fdtable: &FdTable) -> i32 {
        if let Some(wrappedfd) = fdtable.get(&fd) {
            let mut filedesc_enum = wrappedfd.write().unwrap();
            match &mut *filedesc_enum {
                Socket(sockfdobj) => {
                    match sockfdobj.protocol {
                        IPPROTO_TCP => {
                            if sockfdobj.state != ConnState::CONNECTED {
                                return syscall_error(Errno::ENOTCONN, "recvfrom", "The descriptor is not connected");
                            }

                            let sid = Self::getsockobjid(&mut *sockfdobj);
                            let metadata = NET_METADATA.read().unwrap();
                            let sockobj = metadata.socket_object_table.get(&sid).unwrap();

                            let mut newbuflen = buflen;
                            let mut newbufptr = buf;

                            //if we have peeked some data before, fill our buffer with that data before moving on
                            if !sockfdobj.last_peek.is_empty() {
                                let bytecount = interface::rust_max(sockfdobj.last_peek.len(), newbuflen);
                                interface::copy_fromrustdeque_sized(buf, bytecount, &sockfdobj.last_peek);
                                newbuflen -= bytecount;
                                newbufptr = newbufptr.wrapping_add(bytecount);

                                //if we're not still peeking data, consume the data we peeked from our peek buffer
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

                            let mut bufleft = newbufptr;
                            let mut buflenleft = newbuflen;
                            loop {
                                let retval = sockobj.recvfrom(bufleft, buflenleft, addr); //nonblocking, block manually

                                if retval < 0 {
                                    let sockerrno = match Errno::from_discriminant(-retval) {
                                        Ok(i) => i,
                                        Err(()) => panic!("Unknown errno value from socket send returned!"),
                                    };

                                    if sockerrno == Errno::EAGAIN  && (flags & O_NONBLOCK == 0) {
                                        interface::sleep(BLOCK_TIME);
                                        continue;
                                    }

                                    //if our recvfrom call failed but we're not retrying (it wasn't blocking that was 
                                    //the issue), then continue with the data we've read so far if we read any data from
                                    //peek or a previous iteration, or return the error given
                                    if buflen == buflenleft {
                                        return syscall_error(sockerrno, "recvfrom", "Internal call to recvfrom failed");
                                    } else {
                                        break;
                                    }
                                }
                                if retval == 0 {break;}

                                buflenleft -= retval as usize;
                                bufleft = bufleft.wrapping_offset(retval as isize);

                                if buflenleft == 0 || (flags & O_NONBLOCK == 0) {break;}
                            }

                            let totalbyteswritten = buflen - buflenleft;

                            if flags & MSG_PEEK != 0 {
                                //extend from the point after we read our previously peeked bytes
                                interface::extend_fromptr_sized(newbufptr, (newbuflen - buflenleft) as usize, &mut sockfdobj.last_peek);
                            }

                            return totalbyteswritten as i32;

                        }
                        IPPROTO_UDP => {
                            if sockfdobj.localaddr.is_none() {
                                return syscall_error(Errno::EOPNOTSUPP, "recvfrom", "BUG / FIXME: Should bind before using UDP to recv/recvfrom");
                            }

                            let sid = Self::getsockobjid(&mut *sockfdobj);
                            let metadata = NET_METADATA.read().unwrap();
                            let sockobj = metadata.socket_object_table.get(&sid).unwrap();

                            let mut bufleft = buf;
                            let mut buflenleft = buflen;
                            loop {
                                //if the remoteaddr is set and addr is not, use remoteaddr
                                let retval = if addr.is_none() && sockfdobj.remoteaddr.is_some() {
                                    sockobj.recvfrom(bufleft, buflenleft, &mut sockfdobj.remoteaddr.as_mut())
                                } else {
                                    sockobj.recvfrom(bufleft, buflenleft, addr)
                                };
                                if retval == 0 {break;}

                                if retval < 0 {
                                    let sockerrno = match Errno::from_discriminant(1 as i32) {
                                        Ok(i) => i,
                                        Err(()) => panic!("Unknown errno value from socket send returned!"),
                                    };

                                    if sockerrno == Errno::EAGAIN {
                                        interface::sleep(BLOCK_TIME);
                                        continue;
                                    }
                                    println!("ERRNO: {}", -(unsafe{*libc::__errno_location()} as i32));

                                    //if our recvfrom call failed but we're not retrying (it wasn't blocking that was 
                                    //the issue), then continue with the data we've read so far if we read any data from 
                                    //a previous iteration, or return the error given
                                    if buflen == buflenleft {
                                        return syscall_error(sockerrno, "recvfrom", "Internal call to recvfrom failed");
                                    } else {
                                        break;
                                    }
                                }

                                buflenleft -= retval as usize;
                                bufleft = bufleft.wrapping_offset(retval as isize);

                                if buflenleft == 0 {break;}
                            }
                            return (buflen - buflenleft) as i32;
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
        let fdtable = self.filedescriptortable.read().unwrap();
        return self.recv_common(fd, buf, buflen, flags, addr, &*fdtable);
    }

    pub fn recv_syscall(&self, fd: i32, buf: *mut u8, buflen: usize, flags: i32) -> i32 {
        let fdtable = self.filedescriptortable.read().unwrap();
        return self.recv_common(fd, buf, buflen, flags, &mut None, &*fdtable);
    }

    //we currently ignore backlog
    pub fn listen_syscall(&self, fd: i32, _backlog: i32) -> i32 {
        let fdtable = self.filedescriptortable.read().unwrap();
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
                            let mut mutmetadata = NET_METADATA.write().unwrap();
                            let mut ladr;
                            let mut porttuple;
                            match sockfdobj.localaddr {
                                Some(sla) => {
                                    ladr = sla.clone();
                                    porttuple = mux_port(ladr.addr().clone(), ladr.port(), sockfdobj.domain, TCPPORT);

                                    if mutmetadata.listening_port_set.contains(&porttuple) {
                                        match mutmetadata._get_available_tcp_port(ladr.addr().clone(), sockfdobj.domain) {
                                            Ok(port) => ladr.set_port(port),
                                            Err(i) => return i,
                                        }
                                        porttuple = mux_port(ladr.addr().clone(), ladr.port(), sockfdobj.domain, TCPPORT);
                                    }
                                }
                                None => {
                                    ladr = match Self::assign_new_addr(sockfdobj, sockfdobj.domain == AF_INET6) {
                                        Ok(a) => a,
                                        Err(e) => return e,
                                    };
                                    porttuple = mux_port(ladr.addr().clone(), ladr.port(), sockfdobj.domain, TCPPORT);
                                }
                            }
                            mutmetadata.listening_port_set.insert(porttuple);

                            sockfdobj.state = ConnState::LISTEN;

                            //create the socket and bind it before listening
                            let sockobj = interface::Socket::new(sockfdobj.domain, sockfdobj.socktype, sockfdobj.protocol);
                            let bindret = sockobj.bind(&ladr);
                            if bindret < 0 {
                                panic!("Unexpected failure in binding socket");
                            }
                            let listenret = sockobj.listen(5); //default backlog in repy for whatever reason, we replicate it
                            if listenret < 0 {
                                panic!("Unexpected failure in binding socket");
                            }

                            sockfdobj.socketobjectid = match mutmetadata.insert_into_socketobjecttable(sockobj) {
                                Ok(id) => Some(id),
                                Err(errnum) => {
                                    mutmetadata.listening_port_set.remove(&mux_port(ladr.addr().clone(), ladr.port(), sockfdobj.domain, TCPPORT));
                                    sockfdobj.state = ConnState::CONNECTED;
                                    return errnum;
                                }
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
        let mut fdtable = self.filedescriptortable.write().unwrap();
        match how {
            SHUT_RD => {
                return syscall_error(Errno::EOPNOTSUPP, "netshutdown", "partial shutdown read is not implemented");
            }
            SHUT_WR => {
                return Self::_cleanup_socket(self, fd, true, &mut fdtable);
            }
            SHUT_RDWR => {
                //BUG:: need to check for duplicate entries
                return Self::_cleanup_socket(self, fd, false, &mut fdtable);
            }
            _ => {
                //See http://linux.die.net/man/2/shutdown for nuance to this error
                return syscall_error(Errno::EINVAL, "netshutdown", "the shutdown how argument passed is not supported");
            }
        }
    }

    pub fn _cleanup_socket(&self, fd: i32, partial: bool, fdtable: &mut FdTable) -> i32 {

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
    fn _accept_helper(sockfdobj: &mut SocketDesc, mutmetadata: &mut NetMetadata) -> (Result<interface::Socket, i32>, interface::GenSockaddr) {
        let sid = Self::getsockobjid(&mut *sockfdobj);
        let sockobj = mutmetadata.socket_object_table.get(&sid).unwrap();

        match sockfdobj.domain {
            PF_INET => sockobj.accept(true),
            PF_INET6 => sockobj.accept(false),
            _ => panic!("Unknown domain in accepting socket"),
        }
    }
    
    pub fn accept_syscall(&self, fd: i32, addr: &mut interface::GenSockaddr) -> i32 {

        loop { //we must block manually
            let mut fdtable = self.filedescriptortable.write().unwrap();
            if let Some(wrappedfd) = fdtable.get(&fd) {
                let mut filedesc_enum = wrappedfd.write().unwrap();
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

                                let mut mutmetadata = NET_METADATA.write().unwrap();
                                let (acceptedresult, remote_addr) = if let Some(tup) = sockfdobj.pendingconnections.pop() {
                                    //if we got a pending connection in select/poll/whatever, return that here instead
                                    tup
                                } else {
                                    Self::_accept_helper(sockfdobj, &mut *mutmetadata)
                                };

                                if let Err(errval) = acceptedresult {
                                    let accerrno = match Errno::from_discriminant(-errval) {
                                        Ok(i) => i,
                                        Err(()) => panic!("Unknown errno value from socket send returned!"),
                                    };

                                    if accerrno == Errno::EAGAIN {
                                        interface::sleep(BLOCK_TIME);
                                        continue;
                                    }
                                    return errval;
                                }

                                let acceptedsock = acceptedresult.unwrap();

                                //create new connected socket
                                let mut newsockobj = self._socket_initializer(sockfdobj.domain, sockfdobj.socktype, sockfdobj.protocol, false, false);
                                newsockobj.state = ConnState::CONNECTED;

                                let mut newaddr = sockfdobj.localaddr.clone().unwrap();
                                let mut newport = match mutmetadata._reserve_localport(newaddr.addr(), 0, sockfdobj.protocol, sockfdobj.domain) {
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

                                *addr = remote_addr; //populate addr with what address it connected to
                                let domain = sockfdobj.domain;

                                //THIS DEADLOCKS IF WE DON'T DROP THE FDTABLE HERE...
                                //we need to figure out a way to make sure that the loop doesn't break when these are dropped, though
                                //if we drop everything before getting into the loop, we need to make sure that the fd is a sock everytime
                                //and probably that it hasn't changed in the time that it took to drop the lock to the table and regain it...

                                let id = newsockobj.socketobjectid.clone();
                                drop(sockfdobj);
                                drop(wrappedfd);
                                drop(filedesc_enum);

                                //socket inserter code
                                let socket_result = {
                                    let wrappedsock = interface::RustRfc::new(interface::RustLock::new(Socket(newsockobj)));

                                    let newfd = if let Some(fd) = self.get_next_fd(None, Some(&fdtable)) {
                                        fd
                                    } else {
                                        return syscall_error(Errno::ENFILE, "accept", "no available file descriptor number could be found");
                                    };

                                    fdtable.insert(newfd, wrappedsock);
                                    newfd
                                };
                                
                                match socket_result {
                                    x if x < 0 => {
                                        let mut mutmetadata = NET_METADATA.write().unwrap();
                                        mutmetadata.listening_port_set.remove(&mux_port(newipaddr, newport, domain, TCPPORT));
                                        mutmetadata.socket_object_table.remove(&id.unwrap());
                                        return x;
                                    },
                                    y => {
                                        return y;
                                    },
                                }
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
        let mut new_readfds = interface::RustHashSet::<i32>::new();
        let mut new_writefds = interface::RustHashSet::<i32>::new();
        //let mut new_exceptfds = interface::RustHashSet::<i32>::new(); we don't support exceptfds for now
    
        if nfds < STARTINGFD || nfds >= MAXFD {
            return syscall_error(Errno::EINVAL, "select", "Number of FDs is wrong");
        }
    
        let start_time = interface::starttimer();
    
        let end_time = match timeout {
            Some(time) => time,
            None => interface::RustDuration::MAX
        };
    
        let mut retval = 0;
        if !exceptfds.is_empty() {
            return syscall_error(Errno::EOPNOTSUPP, "select", "We don't support exceptfds in select currently");
        }
    
        loop { //we must block manually
            let fdtable = self.filedescriptortable.write().unwrap();
            for fd in readfds.iter() {
                if let Some(wrappedfd) = fdtable.get(&fd) {
                    let mut filedesc_enum = wrappedfd.write().unwrap();

                    match &mut *filedesc_enum {
                        Socket(ref mut sockfdobj) => {
                            if sockfdobj.state == ConnState::LISTEN {
                                let mut mutmetadata = NET_METADATA.write().unwrap();

                                if sockfdobj.pendingconnections.is_empty() {
                                    let listeningsocket = Self::_accept_helper(sockfdobj, &mut *mutmetadata);
                                    if let Ok(_) = listeningsocket.0 {
                                        //save the pending connection for accept to do something with it
                                        sockfdobj.pendingconnections.push(listeningsocket);
                                    } else {
                                        //if it returned an error, then don't insert it into new_readfds
                                        continue;
                                    }
                                }

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
                                    if self._nonblock_peek_read(*fd, &*fdtable) {
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

            for fd in writefds.iter() {
                if let Some(wrappedfd) = fdtable.get(&fd) {
                    let mut filedesc_enum = wrappedfd.write().unwrap();
                    match &mut *filedesc_enum {
                        //we always say sockets are writable?
                        Socket(_) => {
                            let metadata = NET_METADATA.read().unwrap();
                            if !metadata.writersblock_state.load(interface::RustAtomicOrdering::Relaxed) {
                                new_writefds.insert(*fd);
                                retval += 1;
                            }
                        }

                        //we always say streams are writable?
                        Stream(_) => {
                            new_writefds.insert(*fd);
                            retval += 1;
                        }

                        //not supported yet
                        Pipe(_) => {
                            new_readfds.insert(*fd);
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

            //we'd do exceptfds here if we supported them

            if retval != 0 || interface::readtimer(start_time) > end_time {
                break;
            } else {
                interface::sleep(BLOCK_TIME);
            }
        }
        *readfds = new_readfds;
        *writefds = new_writefds;
        return retval; //package out fd_set?
    }

    pub fn getsockopt_syscall(&self, fd: i32, level: i32, optname: i32) -> i32 {
        let fdtable = self.filedescriptortable.read().unwrap();
        
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
                        match optname {
                            //indicate whether we are accepting connections or not in the moment
                            SO_ACCEPTCONN => {
                                if sockfdobj.state == ConnState::LISTEN {
                                    return 1;
                                }
                                return 0;
                            }
                            //if the option is a stored binary option, just return it...
                            SO_LINGER | SO_KEEPALIVE | SO_SNDLOWAT | SO_RCVLOWAT | SO_REUSEPORT | SO_REUSEADDR => {
                                if sockfdobj.options & optname == optname {
                                    return 1;
                                }
                                return 0;
                            }
                            //handling the ignored buffer settings:
                            SO_SNDBUF => {
                                return sockfdobj.sndbuf;
                            }
                            SO_RCVBUF => {
                                return sockfdobj.rcvbuf;
                            }
                            //returning the type if asked
                            SO_TYPE => {
                                return sockfdobj.socktype;
                            }
                            //should always be true
                            SO_OOBINLINE => {
                                return 1;
                            }
                            SO_ERROR => {
                                let tmp = sockfdobj.errno;
                                sockfdobj.errno = 0;
                                return tmp;
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

    //int setsockopt(int sockfd, int level, int optname, const void *optval, socklen_t optlen);
    pub fn setsockopt_syscall(&self, fd: i32, level: i32, optname: i32, optval: i32) -> i32 {
        let fdtable = self.filedescriptortable.read().unwrap();
        
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
                        return 0; //temp for Apache
                        //return syscall_error(Errno::EOPNOTSUPP, "getsockopt", "TCP options not remembered by getsockopt");
                    }
                    SOL_SOCKET => {
                        match optname {
                            SO_ACCEPTCONN | SO_TYPE | SO_SNDLOWAT | SO_RCVLOWAT => {
                                let error_string = format!("Cannot set option using setsockopt. {}", optname);
                                return syscall_error(Errno::ENOPROTOOPT, "setsockopt", &error_string);
                            }
                            //if the option is a stored binary option, just return it...
                            SO_LINGER | SO_KEEPALIVE | SO_REUSEPORT | SO_REUSEADDR => {
                                let mut newoptions = sockfdobj.options;
                                if newoptions & optname == optname {
                                    newoptions = newoptions - optname;
                                    sockfdobj.options = newoptions;
                                    return 1;
                                }
                                
                                //now let's set this if we were told to
                                if optval != 0 {
                                    //optval should always be 1 or 0.
                                    newoptions = newoptions | optname;
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
                                assert_eq!(optval, 1);
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
        let fdtable = self.filedescriptortable.read().unwrap();

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
        let fdtable = self.filedescriptortable.read().unwrap();

        if let Some(wrappedfd) = fdtable.get(&fd) {
            let filedesc = wrappedfd.read().unwrap();
            if let Socket(sockfdobj) = &*filedesc {
                if sockfdobj.localaddr == None {
                    
                    //sets the address to 0.0.0.0 if the address is not initialized yet
                    //setting the family as well based on the domain
                    let addr = match sockfdobj.domain {
                        AF_INET => { interface::GenIpaddr::V4(interface::V4Addr::default()) }
                        AF_INET6 => { interface::GenIpaddr::V6(interface::V6Addr::default()) }
                        _ => { panic!("Unknown domain set"); }
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
    pub fn gethostname_syscall(&self, address_ptr: &mut [u8], length: usize) -> i32 {
        let name_length: usize = DEFAULT_HOSTNAME.chars().count();
        if name_length > length {
            address_ptr[..length].copy_from_slice(&DEFAULT_HOSTNAME[..length].as_bytes());
        } else {
            address_ptr[..name_length].copy_from_slice(&DEFAULT_HOSTNAME.as_bytes());
        }
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
                if events & POLLOUT > 0 {reads.insert(fd);}
                //err
                if events & POLLERR > 0 {reads.insert(fd);}

                let mut mask: u32 = 0;

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
        let mut fdtable = self.filedescriptortable.write().unwrap();
        
        //get a file descriptor
        if let Some(newfd) = self.get_next_fd(None, None) {
            //new epoll fd
            let epollobjfd = EpollDesc {
                mode: 0000,
                registered_fds: interface::RustHashMap::<i32, EpollEvent>::new(),
                advlock: interface::AdvisoryLock::new(),
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

        let mut fdtable = self.filedescriptortable.write().unwrap();

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
                        if let Some(EpollEvent{ events, fd }) = epollfdobj.registered_fds.remove(&fd) {} else {
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

        let fdtable = self.filedescriptortable.read().unwrap();

        if let Some(wrappedfd) = fdtable.get(&epfd) {
            let filedesc_enum = wrappedfd.read().unwrap();
            if let Epoll(epollfdobj) = &*filedesc_enum {
                if !maxevents > 0 {
                    return syscall_error(Errno::EINVAL, "epoll wait", "max events argument is not a positive number");
                }

                //now we know that all of the arguments are valid...
                let mut reads = interface::RustHashSet::<i32>::new();
                let mut writes = interface::RustHashSet::<i32>::new();
                let mut errors = interface::RustHashSet::<i32>::new();

                let mut poll_fds_vec: Vec<PollStruct> = vec![];

                for (&key, &value) in &epollfdobj.registered_fds {

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

                let mut poll_fds_slice = &mut poll_fds_vec[..];
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
            let ipaddr = interface::V4Addr {s_addr: u32::from_ne_bytes([127, 0, 0, 1]).to_be()};
            let innersockaddr = interface::SockaddrV4{sin_family: newdomain as u16, sin_addr: ipaddr, sin_port: 0, padding: 0};
            interface::GenSockaddr::V4(innersockaddr)
        } else if domain == AF_INET6 {
            let ipaddr = interface::V6Addr {s6_addr: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1]};
            let innersockaddr = interface::SockaddrV6{sin6_family: newdomain as u16, sin6_addr: ipaddr, sin6_port: 0, sin6_flowinfo: 0, sin6_scope_id: 0};
            interface::GenSockaddr::V6(innersockaddr)
        } else {
            panic!("Unknown domain set");
        };
    
        let mut mutmetadata = NET_METADATA.write().unwrap();
        if socktype == SOCK_STREAM {
            let port = mutmetadata._get_available_tcp_port(portlessaddr.addr(), newdomain);
    
            if let Err(e) = port {
                this.close_syscall(sock1fd);
                this.close_syscall(sock2fd);
                return e;
            }
    
            let mut addr = portlessaddr;
            addr.set_port(port.unwrap());
    
            let bindret = this.bind_syscall(sock1fd, &addr); //len assigned arbitrarily large value
            if bindret != 0 {
                this.close_syscall(sock1fd);
                this.close_syscall(sock2fd);
                return bindret;
            }
    
            let listenret = this.listen_syscall(sock1fd, 1);
            if listenret != 0 {
                this.close_syscall(sock1fd);
                this.close_syscall(sock2fd);
                return listenret;
            }
    
            let mut garbage_remote = addr.clone();
            let thishandle2 = this.clone();
            let acceptor = interface::helper_thread(move || {
                let accret = thishandle2.accept_syscall(sock1fd, &mut garbage_remote);
                if accret < 0 {
                    panic!("Accept syscall failed unexpectedly in socketpair");
                }
                thishandle2.close_syscall(sock1fd);
                return accret;
            });
    
            let connret = this.connect_syscall(sock2fd, &addr);
            if connret < 0 {
                panic!("Accept syscall failed unexpectedly in socketpair");
            }
    
            let otherfd = acceptor.join().unwrap();
            sv.sock1 = sock2fd;
            sv.sock2 = otherfd;
            return 0;
        } else if socktype == SOCK_DGRAM {
            let port1 = mutmetadata._get_available_udp_port(portlessaddr.addr(), newdomain);
    
            if let Err(e) = port1 {
                this.close_syscall(sock1fd);
                this.close_syscall(sock2fd);
                return e;
            }
    
            let port2 = mutmetadata._get_available_udp_port(portlessaddr.addr(), newdomain);
    
            if let Err(e) = port2 {
                this.close_syscall(sock1fd);
                this.close_syscall(sock2fd);
                return e;
            }
    
            let mut addr1 = portlessaddr.clone();
            let mut addr2 = portlessaddr;
            addr1.set_port(port1.unwrap());
            addr2.set_port(port2.unwrap());
    
            let bind1ret = this.bind_syscall(sock1fd, &addr1); //arbitrarily large length given
            if bind1ret < 0 {
                this.close_syscall(sock1fd);
                this.close_syscall(sock2fd);
                return bind1ret;
            }
    
            let bind2ret = this.bind_syscall(sock1fd, &addr2); //arbitrarily large length given
            if bind2ret < 0 {
                this.close_syscall(sock1fd);
                this.close_syscall(sock2fd);
                return bind2ret;
            }
    
            let conn1ret = this.connect_syscall(sock1fd, &addr2);
            if conn1ret < 0 {
                this.close_syscall(sock1fd);
                this.close_syscall(sock2fd);
                return conn1ret;
            }
    
            let conn2ret = this.connect_syscall(sock1fd, &addr1);
            if conn2ret < 0 {
                this.close_syscall(sock1fd);
                this.close_syscall(sock2fd);
                return conn2ret;
            }
        } else {
            panic!("Unkown socktype set");
        }
        return 0;
    }
}
