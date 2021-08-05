// Network related system calls

use crate::interface;

use super::net_constants::*;
use super::fs_constants::*;
use crate::safeposix::cage::{CAGE_TABLE, Cage, FileDescriptor::*, SocketDesc, FdTable};
use crate::safeposix::filesystem::*;
use crate::safeposix::net::*;
use super::errnos::*;

impl Cage {
    fn _socket_initializer(&self, domain: i32, socktype: i32, protocol: i32, blocking: bool, cloexec: bool) -> i32 {
        let flags = if blocking {O_NONBLOCK} else {0} | if cloexec {O_CLOEXEC} else {0};

        let sockfd = Socket( SocketDesc {
            mode: S_IFSOCK | 0666, //rw-rw-rw- perms, which POSIX does too
            domain: domain,
            socktype: socktype,
            protocol: protocol,
            options: 0, //start with no options set
            sndbuf: 131070, //buffersize, which is only used by getsockopt
            rcvbuf: 262140, //buffersize, which is only used by getsockopt
            state: ConnState::NOTCONNECTED, //we start without a connection
            advlock: interface::AdvisoryLock::new(),
            flags: flags,
            errno: 0,
            localaddr: None,
            remoteaddr: None,
            last_peek: None,
            socketobjectid: None
        });
        let wrappedsock = interface::RustRfc::new(interface::RustLock::new(sockfd));

        let mut fdtable = self.filedescriptortable.write().unwrap();
        let newfd = if let Some(fd) = self.get_next_fd(None, Some(&fdtable)) {
            fd
        } else {
            return syscall_error(Errno::ENFILE, "socket or sockpair", "no available file descriptor number could be found");
        };
        fdtable.insert(newfd, wrappedsock);
        newfd
    }

    pub fn socket_syscall(&self, domain: i32, socktype: i32, protocol: i32) -> i32 {
        let real_socktype = socktype & 0x7; //get the type without the extra flags
        let nonblocking = (socktype & SOCK_NONBLOCK) != 0;
        let cloexec = (socktype & SOCK_CLOEXEC) != 0;

        if nonblocking {
            return syscall_error(Errno::EOPNOTSUPP, "socket", "trying to create a non-blocking socket, which we don't yet support");
        }

        match domain {
            PF_INET => {
                match real_socktype {
                    SOCK_STREAM => {
                        let newprotocol = if protocol == 0 {IPPROTO_TCP} else {protocol};
                        if newprotocol != IPPROTO_TCP {
                            return syscall_error(Errno::EOPNOTSUPP, "socket", "The only SOCK_STREAM implemented is TCP. Unknown protocol input.");
                        }
                        return self._socket_initializer(domain, socktype, newprotocol, nonblocking, cloexec);
                    }
                    SOCK_DGRAM => {
                        let newprotocol = if protocol == 0 {IPPROTO_UDP} else {protocol};
                        if newprotocol != IPPROTO_UDP {
                            return syscall_error(Errno::EOPNOTSUPP, "socket", "The only SOCK_DGRAM implemented is UDP. Unknown protocol input.");
                        }
                        return self._socket_initializer(domain, socktype, newprotocol, false, false); //last two are not passed??
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

    pub fn socketpair_syscall(&self, domain: i32, socktype: i32, protocol: i32, sv: &mut SockPair) -> i32 {
        let newdomain = if domain == AF_UNIX {AF_INET} else {domain};
        let sock1fd = self.socket_syscall(newdomain, socktype, protocol);

        let mut fdtable = self.filedescriptortable.write().unwrap();
        let sock2fd = if let Some(fd) = self.get_next_fd(None, Some(&fdtable)) {
            let clonedfdobj = fdtable.get(&sock1fd).unwrap().clone();//we clone the arc so we can reinsert it
            fdtable.insert(fd, clonedfdobj);
            fd
        } else {
            fdtable.remove(&sock1fd);
            return syscall_error(Errno::ENFILE, "socket or sockpair", "no available file descriptor number could be found");
        };
        sv.sock1 = sock1fd;
        sv.sock2 = sock2fd;
        //bind to localhost if PF_LOCAL?

        return 0;
    }

    //we assume we've converted into a RustSockAddr in the dispatcher
    pub fn bind_syscall(&self, fd: i32, localaddr: &interface::GenSockaddr, len: i32) -> i32 {
        let fdtable = self.filedescriptortable.read().unwrap();
        if let Some(wrappedfd) = fdtable.get(&fd) {
            let mut filedesc_enum = wrappedfd.write().unwrap();
            match &mut *filedesc_enum {
                Socket(sockfdobj) => {
                    if sockfdobj.localaddr.is_some() {
                        return syscall_error(Errno::EINVAL, "bind", "The socket is already bound to an address");
                    }
                    let mut mutmetadata = NET_METADATA.write().unwrap();
                    let mut intent_to_rebind = false;
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

                    let newlocalport = if !intent_to_rebind {
                        let localout = mutmetadata._reserve_localport(localaddr.port(), sockfdobj.protocol);
                        if let Err(errnum) = localout {return errnum;}
                        localout.unwrap()
                    } else {
                        localaddr.port()
                    };

                    let mut newsockaddr = localaddr.clone();
                    newsockaddr.set_port(newlocalport);

                    if sockfdobj.protocol == IPPROTO_UDP {
                        if sockfdobj.socketobjectid.is_some() {
                            mutmetadata.used_udp_port_set.remove(&newlocalport);
                            return syscall_error(Errno::EOPNOTSUPP, "bind", "We can't close the previous listener when re-binding");
                        }

                        let udpsockobj = interface::Socket::new(sockfdobj.domain, sockfdobj.socktype, sockfdobj.protocol);
                        let bindret = udpsockobj.bind(&newsockaddr);
                        if bindret < 0 {
                            panic!("Unexpected failure in binding socket");
                        }

                        sockfdobj.socketobjectid = match mutmetadata.insert_into_socketobjecttable(udpsockobj) {
                            Ok(id) => Some(id),
                            Err(errnum) => {
                                mutmetadata.used_udp_port_set.remove(&newlocalport);
                                return errnum;
                            }
                        }
                    }

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

    fn assign_new_addr(sockfdobj: &SocketDesc, remoteaddr: &interface::GenSockaddr) -> Result<interface::GenSockaddr, i32> {
        if let Some(addr) = &sockfdobj.localaddr {
            Ok(addr.clone())
        } else {
            let mut mutmetadata = NET_METADATA.write().unwrap();
            let mut newremote = remoteaddr.clone();
            let port = mutmetadata._get_available_tcp_port();
            if let Err(e) = port {return Err(e);}
            newremote.set_port(port.unwrap());
            match remoteaddr {
                interface::GenSockaddr::V4(_) => newremote.set_addr(interface::GenIpaddr::V4(interface::V4Addr{s_addr: 0})),
                interface::GenSockaddr::V6(_) => newremote.set_addr(interface::GenIpaddr::V6(interface::V6Addr{s6_addr: [0; 16]})),
            }; //in lieu of getmyip
            Ok(newremote)
        }
    }

    pub fn connect_syscall(&self, fd: i32, remoteaddr: &interface::GenSockaddr) -> i32 {
        let fdtable = self.filedescriptortable.read().unwrap();
        if let Some(wrappedfd) = fdtable.get(&fd) {
            let mut filedesc_enum = wrappedfd.write().unwrap();
            match &mut *filedesc_enum {
                Socket(sockfdobj) => {
                    if sockfdobj.state != ConnState::NOTCONNECTED {
                        return syscall_error(Errno::EISCONN, "connect", "The descriptor is already connected");
                    }

                    if sockfdobj.protocol == IPPROTO_UDP {
                        match sockfdobj.localaddr {
                            Some(_) => return 0,
                            None => {
                                let localaddr = match Self::assign_new_addr(sockfdobj, remoteaddr) {
                                    Ok(a) => a,
                                    Err(e) => return e,
                                };

                                //unlock fdtable so that we're fine to call bind_syscall
                                drop(filedesc_enum);
                                drop(fdtable);

                                return self.bind_syscall(fd, &localaddr, 4096); //len assigned arbitrarily large value
                            }
                        };
                    } else if sockfdobj.protocol == IPPROTO_TCP {
                        let localaddr = match Self::assign_new_addr(sockfdobj, remoteaddr) {
                            Ok(a) => a,
                            Err(e) => return e,
                        };

                        let tcpsockobj = interface::Socket::new(sockfdobj.domain, sockfdobj.socktype, sockfdobj.protocol);

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

    pub fn sendto_syscall(&self, fd: i32, buf: *mut u8, buflen: usize, flags: i32, dest_addr: &interface::GenSockaddr) -> i32 {
        if dest_addr.port() == 0 && dest_addr.addr().is_unspecified() {
            return self.send_syscall(fd, buf, buflen, flags);
        }

        let fdtable = self.filedescriptortable.read().unwrap();
        if let Some(wrappedfd) = fdtable.get(&fd) {
            let mut filedesc_enum = wrappedfd.write().unwrap();
            match &mut *filedesc_enum {
                Socket(sockfdobj) => {
                    if (flags & !MSG_NOSIGNAL) != 0 {
                        return syscall_error(Errno::EOPNOTSUPP, "sendto", "The flags are not understood!");
                    }
                    if sockfdobj.state == ConnState::CONNECTED || sockfdobj.state == ConnState::LISTEN {
                        return syscall_error(Errno::EISCONN, "sendto", "The descriptor is connected");
                    }
                    match sockfdobj.protocol {
                        IPPROTO_TCP => {
                            return syscall_error(Errno::EISCONN, "sendto", "The descriptor is connection-oriented");
                        }
                        IPPROTO_UDP => {
                            let sid = Self::getsockobjid(&mut *sockfdobj);
                            let metadata = NET_METADATA.read().unwrap();
                            let sockobj = metadata.socket_object_table.get(&sid).unwrap();
                            let sockret = sockobj.sendto(buf, buflen, Some(dest_addr));
                            if sockret >= 0 {return sockret;}
                            if sockret == Errno::ENETUNREACH as i32 {
                                return syscall_error(Errno::ENETUNREACH, "sendto", "Network was unreachable due to inability to access local port / IP");
                            } else if sockret == Errno::ENETUNREACH as i32 {
                                return syscall_error(Errno::EADDRINUSE, "sendto", "Network address in use");
                            } else {
                                panic!("Unexpected error recieved from sendto syscall");
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
    pub fn send_syscall(&self, fd: i32, buf: *mut u8, buflen: usize, flags: i32) -> i32 {
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

                            loop {
                                let retval = sockobj.sendto(buf, buflen, None); //nonblocking, so we manually block
                                if -retval == Errno::EAGAIN as i32 {
                                    interface::sleep(interface::RustDuration::MILLISECOND);
                                    continue;
                                }
                                return retval;
                            }
                        }
                        IPPROTO_UDP => {
                            let remoteaddr = match &sockfdobj.remoteaddr {
                                Some(x) => x.clone(),
                                None => return syscall_error(Errno::ENOTCONN, "send", "The descriptor is not connected"),
                            };

                            //drop fdtable lock so as not to deadlock
                            drop(filedesc_enum);
                            drop(fdtable);
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

    pub fn recvfrom_syscall(&self, fd: i32, buf: *mut u8, buflen: usize, flags: i32, addr: &mut Option<&mut interface::GenSockaddr>) -> i32 {
        let fdtable = self.filedescriptortable.read().unwrap();
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

                            let peek = &mut sockfdobj.last_peek;
                            let remoteaddr = &sockfdobj.remoteaddr;

                            let mut newbuflen = buflen;
                            let mut newbufptr = buf;

                            if let Some(ref mut peekvec) = peek {
                                let bytecount = interface::rust_max(peekvec.len(), newbuflen);
                                interface::copy_fromvec_sized(buf, bytecount, peekvec);
                                newbuflen -= bytecount;
                                newbufptr = newbufptr.wrapping_add(bytecount);
                                if newbuflen == 0 {
                                    if flags & MSG_PEEK == 0 {
                                        sockfdobj.last_peek = None;
                                    } else {
                                        peekvec.truncate(bytecount); //vec.truncate is a no-op if vec is already shorter
                                    }
                                    return bytecount as i32;
                                }
                            }

                            let mut retval;
                            loop {
                                retval = sockobj.recvfrom(newbufptr, newbuflen, addr);

                                if -retval == Errno::EAGAIN as i32 {
                                    interface::sleep(interface::RustDuration::MILLISECOND);
                                    continue;
                                }

                                if retval < 0 {
                                    let bytes = buflen - newbuflen;
                                    if bytes == 0 {return retval;}
                                    else {return bytes as i32;}
                                }

                                break;
                            }

                            let totalbyteswritten = buflen - newbuflen + retval as usize;

                            if flags & MSG_PEEK != 0 {
                                if sockfdobj.last_peek.is_none() {
                                    sockfdobj.last_peek = Some(vec!());
                                }
                                //extend from the point after we read our previously peeked bytes
                                interface::extend_fromptr_sized(newbufptr, retval as usize, sockfdobj.last_peek.as_mut().unwrap());
                            } else {
                                sockfdobj.last_peek = None;
                            }

                            return totalbyteswritten as i32;

                        }
                        IPPROTO_UDP => {
                            if sockfdobj.localaddr.is_none() {
                                return syscall_error(Errno::EOPNOTSUPP, "recvfrom", "BUG / FIXME: Should bind before using UDP to recv/recvfrom");
                            }
                            if sockfdobj.remoteaddr.is_none() && addr.is_none() {
                            }
                            let sid = Self::getsockobjid(&mut *sockfdobj);
                            let metadata = NET_METADATA.read().unwrap();
                            let sockobj = metadata.socket_object_table.get(&sid).unwrap();

                            loop {
                                let retval = sockobj.recvfrom(buf, buflen, addr);

                                if -retval == Errno::EAGAIN as i32 {
                                    interface::sleep(interface::RustDuration::MILLISECOND);
                                    continue;
                                }
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

    pub fn recv_syscall(&self, fd: i32, buf: *mut u8, buflen: usize, flags: i32) -> i32 {
        self.recvfrom_syscall(fd, buf, buflen, flags, &mut None)
    }
}
