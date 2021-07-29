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
    pub fn bind_syscall(&self, fd: i32, localaddr: &interface::RustSockAddr, len: i32) -> i32 {
        let fdtable = self.filedescriptortable.read().unwrap();
        if let Some(wrappedfd) = fdtable.get(&fd) {
            let mut filedesc_enum = wrappedfd.write().unwrap();
            match &mut *filedesc_enum {
                Socket(sockobj) => {
                    if sockobj.localaddr.is_some() {
                        return syscall_error(Errno::EINVAL, "bind", "The socket is already bound to an address");
                    }
                    let mut mutmetadata = NET_METADATA.write().unwrap();
                    let mut intent_to_rebind = false;
                    if let Some(fds_on_port) = mutmetadata.porttable.get(&localaddr) {
                        for otherfd_wrapped in fds_on_port {
                            let otherfd_enum = otherfd_wrapped.read().unwrap();
                            if let Socket(othersockobj) = &*otherfd_enum {
                                if othersockobj.domain == sockobj.domain && 
                                   othersockobj.socktype == sockobj.socktype &&
                                   othersockobj.protocol == sockobj.protocol {
                                    if (sockobj.options & othersockobj.options & SO_REUSEPORT) == SO_REUSEPORT {
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
                        let localout = mutmetadata._reserve_localport(localaddr.port(), sockobj.protocol);
                        if let Err(errnum) = localout {return errnum;}
                        localout.unwrap()
                    } else {
                        localaddr.port()
                    };

                    let newsockaddr = interface::RustSockAddr::new(localaddr.ip(), newlocalport);

                    if sockobj.protocol == IPPROTO_UDP {
                        if sockobj.socketobjectid.is_some() {
                            mutmetadata.used_udp_port_set.remove(&newlocalport);
                            return syscall_error(Errno::EOPNOTSUPP, "bind", "We can't close the previous listener when re-binding");
                        }

                        let udpsockobj = if localaddr.ip().is_unspecified() {
                            //loopback stuff
                            if localaddr.is_ipv4() {
                                interface::RustUdpSocket::bind(interface::RustSockAddr::new("127.0.0.1".parse().unwrap(), newlocalport)).unwrap()
                            } else {
                                interface::RustUdpSocket::bind(interface::RustSockAddr::new("::1".parse().unwrap(), newlocalport)).unwrap()
                            }
                        } else {
                            interface::RustUdpSocket::bind(newsockaddr).unwrap()
                        };
                        sockobj.socketobjectid = match mutmetadata.insert_into_socketobjecttable(GeneralizedSocket::Udp(udpsockobj)) {
                            Ok(id) => Some(id),
                            Err(errnum) => {
                                mutmetadata.used_udp_port_set.remove(&newlocalport);
                                return errnum;
                            }
                        }
                    }

                    mutmetadata.porttable.entry(newsockaddr.clone()).or_insert(vec!()).push(wrappedfd.clone());

                    sockobj.localaddr = Some(newsockaddr);
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
}
