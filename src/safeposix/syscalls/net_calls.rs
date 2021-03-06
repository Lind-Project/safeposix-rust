#![allow(dead_code)]
// Network related system calls
// outlines and implements all of the networking system calls that are being emulated/faked in Lind

use crate::interface;
use crate::interface::errnos::{Errno, syscall_error};

use super::net_constants::*;
use super::fs_constants::*;
use super::sys_constants::*;
use crate::safeposix::cage::{CAGE_TABLE, Cage, FileDescriptor::*, SocketDesc, EpollDesc, EpollEvent, FdTable, PollStruct, FileDescriptor};
use crate::safeposix::filesystem::*;
use crate::safeposix::net::*;

impl Cage {
    fn _socket_initializer(&self, domain: i32, socktype: i32, protocol: i32, blocking: bool, cloexec: bool) -> SocketDesc {
        let flags = if blocking {O_NONBLOCK} else {0} | if cloexec {O_CLOEXEC} else {0};

        let mut fakedomain = domain;
        if domain == PF_UNIX {
            fakedomain = PF_INET;
        }

        let sockfd = SocketDesc {
            mode: S_IFSOCK | 0666, //rw-rw-rw- perms, which POSIX does too
            domain: fakedomain,
            realdomain: domain,
            reallocalpath: None,
            optinode: None,
            socktype: socktype,
            protocol: protocol,
            options: 0, //start with no options set
            sndbuf: 131070, //buffersize, which is only used by getsockopt
            rcvbuf: 262140, //buffersize, which is only used by getsockopt
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
        return self.get_next_fd(None, Socket(sockfd));
    }

    fn _implicit_bind(&self, sockfdobj: &mut SocketDesc, domain: i32, sockobj: &(interface::Socket, ConnState)) -> i32 {
        if sockfdobj.localaddr.is_none() {
            let localaddr = match Self::assign_new_addr(sockfdobj, domain, sockfdobj.protocol & (1 << SO_REUSEPORT) != 0) {
                Ok(a) => a,
                Err(e) => return e,
            };

            let bindret = self.bind_inner_socket(sockfdobj, &localaddr, true, sockobj);

            if bindret < 0 {
                match Errno::from_discriminant(interface::get_errno()) {
                    Ok(i) => {return syscall_error(i, "recvfrom", "syscall error from attempting to bind within recvfrom");},
                    Err(()) => panic!("Unknown errno value from socket recvfrom returned!"),
                };
            }
        }
        0
    }

    pub fn swap_unixaddr(remoteaddr: &interface::GenSockaddr) -> interface::GenSockaddr {
        let mut swapaddr = remoteaddr.clone();
        if let Some(addr) = NET_METADATA.revds_table.get(&remoteaddr.clone()) { swapaddr = *addr; }
        swapaddr
    }

    pub fn socket_syscall(&self, domain: i32, socktype: i32, protocol: i32) -> i32 {
        let real_socktype = socktype & 0x7; //get the type without the extra flags, it's stored in the last 3 bits
        let nonblocking = (socktype & SOCK_NONBLOCK) != 0;
        let cloexec = (socktype & SOCK_CLOEXEC) != 0;

        if nonblocking {
            return syscall_error(Errno::EOPNOTSUPP, "socket", "trying to create a non-blocking socket, which we don't yet support");
        }

        match domain {
            PF_UNIX | PF_INET => {
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

    fn bind_inner_socket(&self, sockfdobj: &mut SocketDesc, localaddr: &interface::GenSockaddr, prereserved: bool, sockobj: &(interface::Socket, ConnState)) -> i32 {
        if localaddr.get_family() != sockfdobj.realdomain as u16 {
            return syscall_error(Errno::EINVAL, "bind", "An address with an invalid family for the given domain was specified");
        }

        if sockfdobj.localaddr.is_some() {
            return syscall_error(Errno::EINVAL, "bind", "The socket is already bound to an address");
        }

        let intent_to_rebind = sockfdobj.options & (1 << SO_REUSEPORT) != 0;
        let mut newsockaddr = localaddr.clone();

        if sockfdobj.realdomain == AF_UNIX {
            // create fake IPV4 addr
            let ipaddr = interface::V4Addr {s_addr: u32::from_ne_bytes([127, 0, 0, 1])};
            let innersockaddr = interface::SockaddrV4{sin_family: AF_INET as u16, sin_addr: ipaddr, sin_port: 0, padding: 0};
            newsockaddr = interface::GenSockaddr::V4(innersockaddr);
        }

        let newlocalport = if prereserved {
            localaddr.port()
        } else {
            let localout = NET_METADATA._reserve_localport(newsockaddr.addr(), newsockaddr.port(), sockfdobj.protocol, sockfdobj.domain, intent_to_rebind);
            if let Err(errnum) = localout {return errnum;}
            localout.unwrap()
        };
        newsockaddr.set_port(newlocalport);

        if let Some(id) = sockfdobj.socketobjectid {
            id
        } else {
            let sock = interface::Socket::new(sockfdobj.domain, sockfdobj.socktype, sockfdobj.protocol);
            let id = NET_METADATA.insert_into_socketobjecttable(sock, ConnState::NOTCONNECTED).unwrap();
            sockfdobj.socketobjectid = Some(id);
            id
        };
        let bindret = sockobj.0.bind(&newsockaddr);

        if bindret < 0 {
            match Errno::from_discriminant(interface::get_errno()) {
                Ok(i) => {return syscall_error(i, "bind", "The libc call to bind failed!");},
                Err(()) => panic!("Unknown errno value from socket bind returned!"),
            };
        }

        sockfdobj.localaddr = Some(newsockaddr);
        if sockfdobj.realdomain == AF_UNIX {
            let path = localaddr.path();
            //Check that path is not empty
            if path.len() == 0 {return syscall_error(Errno::ENOENT, "open", "given path was null");}
            let truepath = normpath(convpath(path), self);

            match metawalkandparent(truepath.as_path()) {
                //If neither the file nor parent exists
                (None, None) => {return syscall_error(Errno::ENOENT, "bind", "a directory component in pathname does not exist or is a dangling symbolic link"); }
                //If the file doesn't exist but the parent does
                (None, Some(pardirinode)) => {
                    let filename = truepath.file_name().unwrap().to_str().unwrap().to_string(); //for now we assume this is sane, but maybe this should be checked later

                    let mode;
                    if let Inode::Dir(ref mut dir) = *(FS_METADATA.inodetable.get_mut(&pardirinode).unwrap()) {
                        mode = (dir.mode | S_FILETYPEFLAGS as u32) & S_IRWXA;
                    } else { unreachable!() }
                    let effective_mode = S_IFSOCK as u32 | mode;
    
                    let time = interface::timestamp(); //We do a real timestamp now
                    let newinode = Inode::Socket(SocketInode {
                        size: 0, uid: DEFAULT_UID, gid: DEFAULT_GID,
                        mode: effective_mode, linkcount: 1, refcount: 1,
                        atime: time, ctime: time, mtime: time,
                    });
    
                    let newinodenum = FS_METADATA.nextinode.fetch_add(1, interface::RustAtomicOrdering::Relaxed); //fetch_add returns the previous value, which is the inode number we want
                    if let Inode::Dir(ref mut ind) = *(FS_METADATA.inodetable.get_mut(&pardirinode).unwrap()) {
                        ind.filename_to_inode_dict.insert(filename, newinodenum);
                        ind.linkcount += 1;
                    } //insert a reference to the file in the parent directory
                    sockfdobj.optinode = Some(newinodenum.clone());
                    FS_METADATA.inodetable.insert(newinodenum, newinode);
                    NET_METADATA.domain_socket_table.insert(truepath.clone(), newsockaddr.clone());
                    NET_METADATA.revds_table.insert(newsockaddr, localaddr.clone());
                    sockfdobj.reallocalpath = Some(truepath);  
                }
                (Some(_inodenum), ..) => { return syscall_error(Errno::EADDRINUSE, "bind", "Address already in use"); }
            }
        }

        0
    }

    pub fn bind_inner(&self, fd: i32, localaddr: &interface::GenSockaddr, prereserved: bool) -> i32 {
        if let Some(wrappedfd) = self.filedescriptortable.get(&fd) {
            let wrappedclone = wrappedfd.clone();
            drop(wrappedfd);
            let mut filedesc_enum = wrappedclone.write();
            match &mut *filedesc_enum {
                Socket(sockfdobj) => {
                    let sid = Self::getsockobjid(&mut *sockfdobj);
                    let locksock = NET_METADATA.socket_object_table.get(&sid).unwrap().clone();
                    let sockobj = locksock.read();

                    self.bind_inner_socket(sockfdobj, localaddr, prereserved, &*sockobj)
                }
                _ => {
                    syscall_error(Errno::ENOTSOCK, "bind", "file descriptor refers to something other than a socket")
                }
            }
        } else {
            syscall_error(Errno::EBADF, "bind", "invalid file descriptor")
        }
    }

    fn assign_new_addr(sockfdobj: &SocketDesc, domain: i32, rebindability: bool) -> Result<interface::GenSockaddr, i32> {
        if let Some(addr) = &sockfdobj.localaddr {
            Ok(addr.clone())
        } else {
            let mut newremote: interface::GenSockaddr;
            //This is the specified behavior for the berkeley sockets API
            match domain {
                AF_UNIX => {
                    let path = interface::gen_ud_path();
                    newremote = interface::GenSockaddr::Unix(interface::new_sockaddr_unix(AF_UNIX as u16, path.as_bytes()));
                }
                AF_INET => {
                    newremote = interface::GenSockaddr::V4(interface::SockaddrV4::default());
                    let addr = interface::GenIpaddr::V4(interface::V4Addr::default());
                    newremote.set_addr(addr);
                    newremote.set_family(AF_INET as u16);
                    newremote.set_port(match NET_METADATA._reserve_localport(addr.clone(), 0, sockfdobj.protocol, sockfdobj.domain, rebindability) {
                        Ok(portnum) => portnum,
                        Err(errnum) => return Err(errnum),
                    });
                }
                AF_INET6 => { 
                    newremote = interface::GenSockaddr::V6(interface::SockaddrV6::default());
                    let addr = interface::GenIpaddr::V6(interface::V6Addr::default());
                    newremote.set_addr(addr);
                    newremote.set_family(AF_INET6 as u16);
                    newremote.set_port(match NET_METADATA._reserve_localport(addr.clone(), 0, sockfdobj.protocol, sockfdobj.domain, rebindability) {
                        Ok(portnum) => portnum,
                        Err(errnum) => return Err(errnum),
                    });
                }
                _ => { return Err( syscall_error(Errno::EOPNOTSUPP, "assign", "Unkown protocol when assigning") ); }
            };
            Ok(newremote)
        }
    }

    pub fn connect_syscall(&self, fd: i32, remoteaddr: &interface::GenSockaddr) -> i32 {
        if let Some(wrappedfd) = self.filedescriptortable.get(&fd) {
            let wrappedclone = wrappedfd.clone();
            drop(wrappedfd);
            let mut filedesc_enum = wrappedclone.write();
            match &mut *filedesc_enum {
                Socket(sockfdobj) => {
                    if remoteaddr.get_family() != sockfdobj.realdomain as u16 {
                        return syscall_error(Errno::EINVAL, "connect", "An address with an invalid family for the given domain was specified");
                    }

                    //for UDP, just set the addresses and return
                    if sockfdobj.protocol == IPPROTO_UDP {
                        //we don't need to check connection state for UDP, it's connectionless!
                        sockfdobj.remoteaddr = Some(remoteaddr.clone());
                        match sockfdobj.localaddr {
                            Some(_) => return 0,
                            None => {
                                let localaddr = match Self::assign_new_addr(sockfdobj, sockfdobj.realdomain, sockfdobj.protocol & (1 << SO_REUSEPORT) != 0) {
                                    Ok(a) => a,
                                    Err(e) => return e,
                                };

                                let sid = Self::getsockobjid(&mut *sockfdobj);
                                let locksock = NET_METADATA.socket_object_table.get(&sid).unwrap().clone();
                                let sockobj = locksock.read();

                                return self.bind_inner_socket(sockfdobj, &localaddr, true, &*sockobj);
                            }
                        };
                    } else if sockfdobj.protocol == IPPROTO_TCP {
                        //for TCP, actually create the internal socket object and connect it
                        let mut remoteclone = remoteaddr.clone();
                        let sid = Self::getsockobjid(&mut *sockfdobj);
                        let locksock = NET_METADATA.socket_object_table.get(&sid).unwrap().clone();
                        let mut sockobj = locksock.write();

                        if sockobj.1 != ConnState::NOTCONNECTED {
                            return syscall_error(Errno::EISCONN, "connect", "The descriptor is already connected");
                        }

                        if let None = sockfdobj.localaddr {
                            let localaddr = match Self::assign_new_addr(sockfdobj, sockfdobj.realdomain, sockfdobj.protocol & (1 << SO_REUSEPORT) != 0) {
                                Ok(a) => a,
                                Err(e) => return e,
                            };

                            if let interface::GenSockaddr::Unix(_) = localaddr {
                                self.bind_inner_socket(sockfdobj, &localaddr, false, &*sockobj);
                            } else {
                                let bindret = sockobj.0.bind(&localaddr);
                                if bindret < 0 {
                                    match Errno::from_discriminant(interface::get_errno()) {
                                        Ok(i) => {return syscall_error(i, "connect", "The libc call to bind within connect failed");},
                                        Err(()) => panic!("Unknown errno value from socket bind within connect returned!"),
                                    };
                                }
                                sockfdobj.localaddr = Some(localaddr);
                            }
                        } 
                        
                        if let interface::GenSockaddr::Unix(_) = remoteaddr {
                            let path = remoteaddr.path().clone();
                            let truepath = normpath(convpath(path), self);
                            if !NET_METADATA.domain_socket_table.contains_key(&truepath) {return syscall_error(Errno::ECONNREFUSED, "connect", "The libc call to connect failed!");}
                            remoteclone = NET_METADATA.domain_socket_table.get(&truepath).unwrap().clone();
                            sockobj.0.set_blocking(); // unix domain sockets block on connect evne if nb, for now we fake them so set blocking and then unset after
                        };

                        let mut inprogress = false;
                        let connectret = sockobj.0.connect(&remoteclone);
                        if connectret < 0 {
                            match Errno::from_discriminant(interface::get_errno()) {
                                Ok(i) => {
                                    if i == Errno::EINPROGRESS { inprogress = true; }
                                    else { return syscall_error(i, "connect", "The libc call to connect failed!") };
                                },
                                Err(()) => panic!("Unknown errno value from socket connect returned!"),
                            };

                        }

                        if let interface::GenSockaddr::Unix(_) = remoteaddr { sockobj.0.set_blocking(); };

                        sockobj.1 = ConnState::CONNECTED;
                        sockfdobj.remoteaddr = Some(remoteaddr.clone());
                        sockfdobj.errno = 0;
                        if inprogress {
                            sockobj.1 = ConnState::INPROGRESS;
                            return syscall_error(Errno::EINPROGRESS, "connect", "The libc call to connect is in progress.");
                        }
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
            sockfdobj.socketobjectid = Some(NET_METADATA.insert_into_socketobjecttable(sock, ConnState::NOTCONNECTED).unwrap());
        } 
        sockfdobj.socketobjectid.unwrap()
    }

    pub fn sendto_syscall(&self, fd: i32, buf: *const u8, buflen: usize, flags: i32, dest_addr: &interface::GenSockaddr) -> i32 {
        //if ip and port are not specified, shunt off to send
        if dest_addr.port() == 0 && dest_addr.addr().is_unspecified() {
            return self.send_syscall(fd, buf, buflen, flags);
        }

        if let Some(wrappedfd) = self.filedescriptortable.get(&fd) {
            let wrappedclone = wrappedfd.clone();
            drop(wrappedfd);
            let mut filedesc_enum = wrappedclone.write();
            match &mut *filedesc_enum {
                Socket(sockfdobj) => {
                    if dest_addr.get_family() != sockfdobj.realdomain as u16 {
                        return syscall_error(Errno::EINVAL, "sendto", "An address with an invalid family for the given domain was specified");
                    }
                    if (flags & !MSG_NOSIGNAL) != 0 {
                        return syscall_error(Errno::EOPNOTSUPP, "sendto", "The flags are not understood!");
                    }

                    let sid = Self::getsockobjid(&mut *sockfdobj);

                    let sockobjwrapper = NET_METADATA.socket_object_table.get(&sid).unwrap();
                    let sockobj = sockobjwrapper.read();


                    if sockobj.1 != ConnState::NOTCONNECTED {
                        return syscall_error(Errno::EISCONN, "sendto", "The descriptor is connected");
                    }

                    match sockfdobj.protocol {
                        //Sendto doesn't make sense for the TCP protocol, it's connection oriented
                        IPPROTO_TCP => {
                            return syscall_error(Errno::EISCONN, "sendto", "The descriptor is connection-oriented");
                        }

                        IPPROTO_UDP => {
                            let tmpdest = *dest_addr;
                            let ibindret = self._implicit_bind(&mut *sockfdobj, tmpdest.get_family() as i32, &*sockobj);
                            if ibindret < 0 {
                                return ibindret;
                            }

                            //we don't mind if this fails for now and we will just get the error
                            //from calling sendto

                            let sockret = sockobj.0.sendto(buf, buflen, Some(dest_addr));

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
        if let Some(wrappedfd) = self.filedescriptortable.get(&fd) {
            let wrappedclone = wrappedfd.clone();
            drop(wrappedfd);
            let mut filedesc_enum = wrappedclone.write();
            match &mut *filedesc_enum {
                Socket(sockfdobj) => {
                    if (flags & !MSG_NOSIGNAL) != 0 {
                        return syscall_error(Errno::EOPNOTSUPP, "send", "The flags are not understood!");
                    }

                    match sockfdobj.protocol {
                        IPPROTO_TCP => {
                            let sid = Self::getsockobjid(&mut *sockfdobj);
                            let sockobjwrapper = NET_METADATA.socket_object_table.get(&sid).unwrap();
                            let sockobj = &*sockobjwrapper.read();

                            if sockobj.1 != ConnState::CONNECTED {
                                return syscall_error(Errno::ENOTCONN, "send", "The descriptor is not connected");
                            }

                            let retval = sockobj.0.sendto(buf, buflen, None);
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

                            drop(filedesc_enum);

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

    fn recv_common_inner(&self, filedesc_enum: &mut FileDescriptor, buf: *mut u8, buflen: usize, flags: i32, addr: &mut Option<&mut interface::GenSockaddr>) -> i32 {
       match &mut *filedesc_enum {
           Socket(ref mut sockfdobj) => {
               match sockfdobj.protocol {
                   IPPROTO_TCP => {
                       let sid = Self::getsockobjid(&mut *sockfdobj);
                       let locksock = NET_METADATA.socket_object_table.get(&sid).unwrap().clone();
                       let sockobj = locksock.read();

                       if sockobj.1 != ConnState::CONNECTED {
                           return syscall_error(Errno::ENOTCONN, "recvfrom", "The descriptor is not connected");
                       }

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

                       let retval;
                       if sockfdobj.flags & O_NONBLOCK != 0 {
                           retval = sockobj.0.recvfrom_nonblocking(bufleft, buflenleft, addr);
                       } else {
                           retval = sockobj.0.recvfrom(bufleft, buflenleft, addr);
                       }

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
                       let sid = Self::getsockobjid(&mut *sockfdobj);
                       let locksock = NET_METADATA.socket_object_table.get(&sid).unwrap().clone();
                       let sockobj = locksock.read();

                       let binddomain : i32;
                       if let Some(baddr) = addr {
                            binddomain = baddr.get_family() as i32;
                       } else { binddomain = AF_INET }
                       let ibindret = self._implicit_bind(&mut *sockfdobj, binddomain, &sockobj);
                       if ibindret < 0 {
                           return ibindret;
                       }

                       //if the remoteaddr is set and addr is not, use remoteaddr
                       let retval = if addr.is_none() && sockfdobj.remoteaddr.is_some() {
                           sockobj.0.recvfrom(buf, buflen, &mut sockfdobj.remoteaddr.as_mut())
                       } else {
                           sockobj.0.recvfrom(buf, buflen, addr)
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
    }

    pub fn recv_common(&self, fd: i32, buf: *mut u8, buflen: usize, flags: i32, addr: &mut Option<&mut interface::GenSockaddr>) -> i32 {
        if let Some(wrappedfd) = self.filedescriptortable.get(&fd) {
            let clonedfd = wrappedfd.clone();
            drop(wrappedfd);
            let mut filedesc_enum = clonedfd.write();
            return self.recv_common_inner(&mut *filedesc_enum, buf, buflen, flags, addr);
        } else {
            return syscall_error(Errno::EBADF, "recvfrom", "invalid file descriptor");
        }
    }

    pub fn recvfrom_syscall(&self, fd: i32, buf: *mut u8, buflen: usize, flags: i32, addr: &mut Option<&mut interface::GenSockaddr>) -> i32 {
        return self.recv_common(fd, buf, buflen, flags, addr);
    }

    pub fn recv_syscall(&self, fd: i32, buf: *mut u8, buflen: usize, flags: i32) -> i32 {
        return self.recv_common(fd, buf, buflen, flags, &mut None);
    }

    //we currently ignore backlog
    pub fn listen_syscall(&self, fd: i32, _backlog: i32) -> i32 {
        if let Some(wrappedfd) = self.filedescriptortable.get(&fd) {
            let clonedfd = wrappedfd.clone();
            drop(wrappedfd);
            let mut filedesc_enum = clonedfd.write();

            match &mut *filedesc_enum {
                Socket(sockfdobj) => {
                    //get or create the socket and bind it before listening
                    let sid = Self::getsockobjid(sockfdobj);
                    let locksock = NET_METADATA.socket_object_table.get(&sid).unwrap().clone();
                    let mut sockobj = locksock.write();
                    match sockobj.1 {
                        ConnState::LISTEN => {
                            return 0; //Already done!
                        }

                        ConnState::CONNECTED | ConnState::INPROGRESS => {
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
                                    ladr = sla.clone();
                                    porttuple = mux_port(ladr.addr().clone(), ladr.port(), sockfdobj.domain, TCPPORT);

                                    if NET_METADATA.listening_port_set.contains(&porttuple) {
                                        match NET_METADATA._get_available_tcp_port(ladr.addr().clone(), sockfdobj.domain, sockfdobj.options & (1 << SO_REUSEPORT) != 0) {
                                            Ok(port) => ladr.set_port(port),
                                            Err(i) => return i,
                                        }
                                        porttuple = mux_port(ladr.addr().clone(), ladr.port(), sockfdobj.domain, TCPPORT);
                                    }
                                }
                                None => {
                                    ladr = match Self::assign_new_addr(sockfdobj, sockfdobj.realdomain, sockfdobj.protocol & (1 << SO_REUSEPORT) != 0) {
                                        Ok(a) => a,
                                        Err(e) => return e,
                                    };
                                    porttuple = mux_port(ladr.addr().clone(), ladr.port(), sockfdobj.domain, TCPPORT);
                                }
                            }

                            NET_METADATA.listening_port_set.insert(porttuple);
                            sockobj.1 = ConnState::LISTEN;

                            if let None = sockfdobj.localaddr {
                                let bindret = sockobj.0.bind(&ladr);
                                if bindret < 0 {
                                    match Errno::from_discriminant(interface::get_errno()) {
                                        Ok(i) => {return syscall_error(i, "listen", "The libc call to bind within listen failed");},
                                        Err(()) => panic!("Unknown errno value from socket bind within listen returned!"),
                                    };
                                }
                            }
                            let listenret = sockobj.0.listen(5); //default backlog in repy for whatever reason, we replicate it
                            if listenret < 0 {
                                let lr = match Errno::from_discriminant(interface::get_errno()) {
                                    Ok(i) => syscall_error(i, "listen", "The libc call to listen failed!"),
                                    Err(()) => panic!("Unknown errno value from socket listen returned!"),
                                };
                                NET_METADATA.listening_port_set.remove(&mux_port(ladr.addr().clone(), ladr.port(), sockfdobj.domain, TCPPORT));
                                sockobj.1 = ConnState::CONNECTED;
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
        match how {
            SHUT_RD => {
                return syscall_error(Errno::EOPNOTSUPP, "netshutdown", "partial shutdown read is not implemented");
            }
            SHUT_WR => {
                return Self::_cleanup_socket(self, fd, true);
            }
            SHUT_RDWR => {
                return Self::_cleanup_socket(self, fd, false);
            }
            _ => {
                //See http://linux.die.net/man/2/shutdown for nuance to this error
                return syscall_error(Errno::EINVAL, "netshutdown", "the shutdown how argument passed is not supported");
            }
        }
    }

    pub fn _cleanup_socket_inner(&self, filedesc: &mut FileDescriptor, partial: bool, shutdown: bool) -> i32 {
        if let Socket(sockfdobj) = filedesc {
            if let Some(localaddr) = sockfdobj.localaddr.as_ref().clone() {
                let release_ret_val = NET_METADATA._release_localport(localaddr.addr(), localaddr.port(), sockfdobj.protocol, sockfdobj.domain);
                sockfdobj.localaddr = None;
                if let Err(e) = release_ret_val {return e;}
                if !partial {
                    if let Some(soid) = sockfdobj.socketobjectid {
                        if shutdown {
                            //we need to close the socket in order to send an EOF down it, but we
                            //also need to have a valid socket object present and pointed to
                            //otherwise table state gets corrupted/wonky
                            let sockobjtherelock = NET_METADATA.socket_object_table.get(&soid).unwrap().clone();
                            let mut sockobjthere = sockobjtherelock.write();

                            //dropping the old socket closes it
                            sockobjthere.0 = interface::Socket::new(sockfdobj.domain, sockfdobj.socktype, sockfdobj.protocol);
                            sockobjthere.1 = ConnState::NOTCONNECTED;

                            if let Some(localaddr) = sockfdobj.localaddr {
                                if sockobjthere.0.bind(&localaddr) != 0 {
                                    panic!("Bind on known ok address failed within shutdown!");
                                }
                            }

                            //check reuseaddr/port
                            for optname in [sockfdobj.options & SO_REUSEPORT, sockfdobj.options & SO_REUSEADDR] {
                                if optname != 0 {
                                    if sockobjthere.0.setsockopt(SOL_SOCKET, optname, 1) < 0 {
                                        panic!("Setsockopt within known ok conditions failed within shutdown!");
                                    }
                                }
                            }

                            //check nonblock
                            if sockfdobj.flags & O_NONBLOCK != 0 {
                                if sockobjthere.0.set_nonblocking() < 0 {
                                    panic!("Setting nonblock using fcntl on known ok fd failed within shutdown!");
                                }
                            }

                            //now we have completely recreated the socket but unconnected
                        } else {
                            //Reaching this means that the socket is closed. Removing the sockobj
                            //indicates that the sockobj will drop, and therefore close
                            NET_METADATA.socket_object_table.remove(&soid).unwrap();
                        }
                    }
                }
            }
        } else {
            return syscall_error(Errno::ENOTSOCK, "cleanup socket", "file descriptor is not a socket");
        }
        return 0;
    }

    pub fn _cleanup_socket(&self, fd: i32, partial: bool) -> i32 {

        //The FdTable must always be passed.

        if let interface::RustHashEntry::Occupied(mut occval) = self.filedescriptortable.entry(fd) {
            let inner_result = self._cleanup_socket_inner(&mut *occval.get_mut().write(), partial, true);
            if inner_result < 0 {
                return inner_result;
            }

            if !partial {
                occval.remove();
            }
        } else {
            return syscall_error(Errno::EBADF, "cleanup socket", "invalid file descriptor");
        }

        return 0;
    }


    
    //calls accept on the socket object with value depending on ipv4 or ipv6
    pub fn accept_syscall(&self, fd: i32, addr: &mut interface::GenSockaddr) -> i32 {

        if let Some(wrappedfd) = self.filedescriptortable.get(&fd) {
            let unwrapclone = wrappedfd.clone();
            drop(wrappedfd);

            //we need to reserve this fd early to make sure that we don't need to
            //error out later so we perform get_next_fd manually, and populate it
            //at the end
            let mut vacantentry = None;
            for possfd in 0..MAXFD{
                match self.filedescriptortable.entry(possfd) {
                    interface::RustHashEntry::Occupied(_) => {}
                    interface::RustHashEntry::Vacant(vacant) => {
                        vacantentry = Some(vacant);
                        break;
                    }
                };
            };

            if let None = vacantentry {
                return syscall_error(Errno::ENFILE, "open_syscall", "no available file descriptor number could be found");
            }

            let entry = vacantentry.unwrap();
            let key = *entry.key();

            let mut filedesc_enum = unwrapclone.write();
            match &mut *filedesc_enum {
                Socket(ref mut sockfdobj) => {
                    match sockfdobj.protocol {
                        IPPROTO_UDP => {
                            return syscall_error(Errno::EOPNOTSUPP, "accept", "Protocol does not support listening");
                        }
                        IPPROTO_TCP => {
                            let sid = Self::getsockobjid(&mut *sockfdobj);
                            let locksock = NET_METADATA.socket_object_table.get(&sid).unwrap().clone();
                            let sockobj = locksock.read();

                            if sockobj.1 != ConnState::LISTEN {
                                return syscall_error(Errno::EINVAL, "accept", "Socket must be listening before accept is called");
                            }

                            //we need to lock this socket for the duration so that nothing else can
                            //access it before we're done populating with sane data, but we make
                            //sure that we're not locking up the entire fdtable, insert takes
                            //ownership of the entry so we can connect in another thread using the
                            //fdtable
                            let newsockobj = self._socket_initializer(sockfdobj.domain, sockfdobj.socktype, sockfdobj.protocol, sockfdobj.flags & O_NONBLOCK != 0, sockfdobj.flags & O_CLOEXEC != 0);
                            let arclocksock = interface::RustRfc::new(interface::RustLock::new(Socket(newsockobj)));
                            let mut sockref = arclocksock.write();
                            let mut newsockwithin = if let Socket(s) = &mut *sockref {s} else {unreachable!()};
                            entry.insert(arclocksock.clone());

                            let (acceptedresult, remote_addr) = if let Some(mut vec) = NET_METADATA.pending_conn_table.get_mut(&sockfdobj.localaddr.unwrap().port()) {
                                //if we got a pending connection in select/poll/whatever, return that here instead
                                let tup = vec.pop().unwrap(); //pending connection tuple recieved
                                if vec.is_empty() {
                                    drop(vec);
                                    NET_METADATA.pending_conn_table.remove(&sockfdobj.localaddr.unwrap().port()); //remove port from pending conn table if no more pending conns exist for it
                                }
                                tup
                            } else {
                                if 0 == (sockfdobj.flags & O_NONBLOCK) {
                                    match sockfdobj.domain {
                                        PF_INET => sockobj.0.accept(true),
                                        PF_INET6 => sockobj.0.accept(false),
                                        _ => panic!("Unknown domain in accepting socket"),
                                    }
                                } else {
                                    match sockfdobj.domain {
                                        PF_INET => sockobj.0.nonblock_accept(true),
                                        PF_INET6 => sockobj.0.nonblock_accept(false),
                                        _ => panic!("Unknown domain in accepting socket"),
                                    }
                                }
                            };

                            if let Err(_) = acceptedresult {
                                match Errno::from_discriminant(interface::get_errno()) {
                                    Ok(e) => {
                                        self.filedescriptortable.remove(&key);
                                        return syscall_error(e, "accept", "host system accept call failed");
                                    },
                                    Err(()) => panic!("Unknown errno value from socket send returned!"),
                                };
                            }

                            let acceptedsock = acceptedresult.unwrap();

                            let mut newaddr = sockfdobj.localaddr.clone().unwrap();
                            let newport = match NET_METADATA._reserve_localport(newaddr.addr(), 0, sockfdobj.protocol, sockfdobj.domain, false) {
                                Ok(portnum) => portnum,
                                Err(errnum) => {
                                    self.filedescriptortable.remove(&key);
                                    return errnum;
                                }
                            };
                            newaddr.set_port(newport);

                            let newipaddr = newaddr.addr().clone();
                            newsockwithin.localaddr = Some(newaddr);
                            newsockwithin.remoteaddr = Some(remote_addr.clone());

                            //create socket object for new connected socket
                            drop(sockobj);
                            newsockwithin.socketobjectid = match NET_METADATA.insert_into_socketobjecttable(acceptedsock, ConnState::CONNECTED) {
                                Ok(id) => Some(id),
                                Err(errnum) => {
                                    NET_METADATA.listening_port_set.remove(&mux_port(newipaddr.clone(), newport, sockfdobj.domain, TCPPORT));
                                    self.filedescriptortable.remove(&key);
                                    return errnum;
                                }
                            };
                            let possibleunixaddr = Self::swap_unixaddr(&remote_addr.clone());
                            if let interface::GenSockaddr::Unix(_) = possibleunixaddr {
                                let pathclone = normpath(convpath(possibleunixaddr.path().clone()), self);
                                if let Some(inodenum) = metawalk(pathclone.as_path()) {                
                                    newsockwithin.realdomain = AF_UNIX;
                                    newsockwithin.reallocalpath = Some(pathclone);   
                                    newsockwithin.optinode = Some(inodenum.clone());   
                                    if let Inode::Socket(ref mut sock) = *(FS_METADATA.inodetable.get_mut(&inodenum).unwrap()) { 
                                        sock.refcount += 1; 
                                    } 
                                };
                            };
                            

                            *addr = remote_addr; //populate addr with what address it connected to

                            return key;
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

    fn _nonblock_peek_read(&self, fd: i32) -> bool{
        let flags = MSG_PEEK;
        let mut buf = [0u8; 1];
        let bufptr = buf.as_mut_ptr();
        if let Some(wrappedfd) = self.filedescriptortable.get(&fd) {
            let clonedfd = wrappedfd.clone();
            drop(wrappedfd);
            let mut filedesc_enum = clonedfd.write();
            let oldflags;
            if let Socket(ref mut sockfdobj) = &mut *filedesc_enum {
                oldflags = sockfdobj.flags;
                sockfdobj.flags |= O_NONBLOCK;
            } else {
                return false;
            }
            let retval = self.recv_common_inner(&mut *filedesc_enum, bufptr, 1, flags, &mut None);
            if let Socket(ref mut sockfdobj) = &mut *filedesc_enum {
                sockfdobj.flags = oldflags;
            } else {
                unreachable!();
            }
            return retval >= 0; //it it's less than 0, it failed, it it's 0 peer is dead, 1 it succeeded, in the latter 2 it's true
        } else {
            return false;
        }
    }

    //TODO: handle pipes
    pub fn select_syscall(&self, nfds: i32, readfds: &mut interface::RustHashSet<i32>, writefds: &mut interface::RustHashSet<i32>, exceptfds: &mut interface::RustHashSet<i32>, timeout: Option<interface::RustDuration>) -> i32 {
        //exceptfds and writefds are not really implemented at the current moment.
        //They both always return success. However we have some intention of making
        //writefds work at some point for pipes? We have no such intention for exceptfds
        let new_readfds = interface::RustHashSet::<i32>::new();
        let new_writefds = interface::RustHashSet::<i32>::new();
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
                if let Some(wrappedfd) = self.filedescriptortable.get(&fd) {
                    let wrappedclone = wrappedfd.clone();
                    drop(wrappedfd);
                    let mut filedesc_enum = wrappedclone.write();

                    match &mut *filedesc_enum {
                        Socket(ref mut sockfdobj) => {
                            let sid = Self::getsockobjid(&mut *sockfdobj);
                            let locksock = NET_METADATA.socket_object_table.get(&sid).unwrap().clone();
                            let mut sockobj = locksock.write();

                            if sockobj.1 == ConnState::LISTEN {
                                if let interface::RustHashEntry::Vacant(vacant) = NET_METADATA.pending_conn_table.entry(sockfdobj.localaddr.unwrap().port().clone()) {

                                    let listeningsocket = match sockfdobj.domain {
                                        PF_INET => sockobj.0.nonblock_accept(true),
                                        PF_INET6 => sockobj.0.nonblock_accept(false),
                                        _ => panic!("Unknown domain in accepting socket"),
                                    };
                                    drop(sockobj);
                                    if let Ok(_) = listeningsocket.0 {
                                        //save the pending connection for accept to do something with it
                                        vacant.insert(vec!(listeningsocket));
                                    } else {
                                        //if it returned an error, then don't insert it into new_readfds
                                        continue;
                                    }
                                } //if it's already got a pending connection, add it!

                                //if we reach here there is a pending connection
                                new_readfds.insert(*fd);
                                retval += 1;
                            } else if sockobj.1 == ConnState::INPROGRESS && sockobj.0.check_rawconnection() {
                                    sockobj.1 = ConnState::CONNECTED;
                                    new_readfds.insert(*fd);
                                    retval += 1;
                            } else {
                                if sockfdobj.protocol == IPPROTO_UDP {
                                    new_readfds.insert(*fd);
                                    retval += 1;
                                } else {
                                    drop(sockfdobj);
                                    drop(filedesc_enum);
                                    drop(sockobj);
                                    if self._nonblock_peek_read(*fd) {
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
                if let Some(wrappedfd) = self.filedescriptortable.get(&fd) {
                    let wrappedclone = wrappedfd.clone();
                    drop(wrappedfd);
                    let mut filedesc_enum = wrappedclone.write();
                    match &mut *filedesc_enum {
                        Socket(ref mut sockfdobj) => {
                            // check if we've made an in progress connection first
                            let sid = Self::getsockobjid(&mut *sockfdobj);
                            let locksock = NET_METADATA.socket_object_table.get(&sid).unwrap().clone();
                            let mut sockobj = locksock.write();
                            if sockobj.1 == ConnState::INPROGRESS && sockobj.0.check_rawconnection() {
                                sockobj.1 = ConnState::CONNECTED;
                            } 
                            
                            //we always say sockets are writable? Even though this is not true
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
                if !self.filedescriptortable.contains_key(&fd) {
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
        
        if let Some(wrappedfd) = self.filedescriptortable.get(&fd) {
            let wrappedclone = wrappedfd.clone();
            drop(wrappedfd);
            let mut filedesc = wrappedclone.write();
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
                                if let Some(sid) = sockfdobj.socketobjectid {
                                    let locksock = NET_METADATA.socket_object_table.get(&sid).unwrap().clone();
                                    let sockobj = locksock.read();
                                    if sockobj.1 == ConnState::LISTEN {
                                        *optval = 1;
                                    } else {
                                        *optval = 0;
                                    }
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
        
        if let Some(wrappedfd) = self.filedescriptortable.get(&fd) {
            let wrappedclone = wrappedfd.clone();
            drop(wrappedfd);
            let mut filedesc = wrappedclone.write();
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
                                    let locksock = NET_METADATA.socket_object_table.get(&sid).unwrap().clone();
                                    let sockobj = locksock.read();

                                    let sockoptret = sockobj.0.setsockopt(SOL_SOCKET, optname, optval);
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
        if let Some(wrappedfd) = self.filedescriptortable.get(&fd) {
            let wrappedclone = wrappedfd.clone();
            drop(wrappedfd);
            let filedesc = wrappedclone.read();
            if let Socket(sockfdobj) = &*filedesc {
                //if the socket is not connected, then we should return an error
                if sockfdobj.remoteaddr == None {
                    return syscall_error(Errno::ENOTCONN, "getpeername", "the socket is not connected");
                }
                // will swap if unix
                let remoteaddr = Self::swap_unixaddr(&sockfdobj.remoteaddr.unwrap().clone());
                //all of the checks that we had have passed if we are here
                *ret_addr = remoteaddr;
                return 0;

            } else {
                return syscall_error(Errno::ENOTSOCK, "getpeername", "the provided file is not a socket");
            }
        } else {
            return syscall_error(Errno::EBADF, "getpeername", "the provided file descriptor is not valid");
        }
    }

    pub fn getsockname_syscall(&self, fd: i32, ret_addr: &mut interface::GenSockaddr) -> i32 {
        if let Some(wrappedfd) = self.filedescriptortable.get(&fd) {
            let wrappedclone = wrappedfd.clone();
            drop(wrappedfd);
            let filedesc = wrappedclone.read();
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
        
        let epollobjfd = Epoll(EpollDesc {
            mode: 0000,
            registered_fds: interface::RustHashMap::<i32, EpollEvent>::new(),
            advlock: interface::RustRfc::new(interface::AdvisoryLock::new()),
            errno: 0,
            flags: 0
        });
        //get a file descriptor
        return self.get_next_fd(None, epollobjfd);
    }

    pub fn epoll_create_syscall(&self, size: i32) -> i32 {
        if size <= 0 {
            return syscall_error(Errno::EINVAL, "epoll create", "provided size argument is invalid");
        }
        return Self::_epoll_object_allocator(self);
    }

    //this one can still be optimized
    pub fn epoll_ctl_syscall(&self, epfd: i32, op: i32, fd: i32, event: &EpollEvent) -> i32 {

        //making sure that the epfd is really an epoll fd
        if let Some(wrappedfd) = self.filedescriptortable.get(&epfd) {
            let wrappedclone = wrappedfd.clone();
            drop(wrappedfd);
            let mut filedesc_enum_epollfd = wrappedclone.write();
            if let Epoll(epollfdobj) = &mut *filedesc_enum_epollfd {

                //check if the other fd is an epoll or not...
                if let Epoll(_) = &*self.filedescriptortable.get(&fd).unwrap().read() {
                    return syscall_error(Errno::EBADF, "epoll ctl", "provided fd is not a valid file descriptor")
                }

                //now that we know that the types are all good...
                match op {
                    EPOLL_CTL_DEL => {
                        //since remove returns the value at the key and the values will always be EpollEvents, 
                        //I am using this to optimize the code
                        epollfdobj.registered_fds.remove(&fd).unwrap().1;
                    }
                    EPOLL_CTL_MOD => {
                        //check if the fd that we are modifying exists or not
                        if !epollfdobj.registered_fds.contains_key(&fd) {
                            return syscall_error(Errno::ENOENT, "epoll ctl", "fd is not registered with this epfd");
                        }
                        //if the fd already exists, insert overwrites the prev entry
                        epollfdobj.registered_fds.insert(fd, EpollEvent { events: event.events, fd: event.fd });
                    }
                    EPOLL_CTL_ADD => {
                        if epollfdobj.registered_fds.contains_key(&fd) {
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

        if let Some(wrappedfd) = self.filedescriptortable.get(&epfd) {
            let wrappedclone = wrappedfd.clone();
            drop(wrappedfd);
            let filedesc_enum = wrappedclone.write();
            if let Epoll(epollfdobj) = &*filedesc_enum {
                if !maxevents > 0 {
                    return syscall_error(Errno::EINVAL, "epoll wait", "max events argument is not a positive number");
                }

                let mut poll_fds_vec: Vec<PollStruct> = vec![];

                for set in epollfdobj.registered_fds.iter() {
                    let (&key, &value) = set.pair();

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


            let connret = this.connect_syscall(sock2fd, &bound_addr);
            if connret < 0 {
                let sockerrno = match Errno::from_discriminant(interface::get_errno()) {
                    Ok(i) => i,
                    Err(()) => panic!("Unknown errno value from connect within socketpair returned!"),
                };
                this.close_syscall(sock1fd);
                this.close_syscall(sock2fd);
                return syscall_error(sockerrno, "socketpair", "The libc call to connect within socketpair failed!");
            }
    
            let mut garbage_addr = portlessaddr.clone();
            let accret = this.accept_syscall(sock1fd, &mut garbage_addr);
            if accret < 0 {
                let sockerrno = match Errno::from_discriminant(interface::get_errno()) {
                    Ok(i) => i,
                    Err(()) => panic!("Unknown errno value from accept within socketpair returned!"),
                };
                this.close_syscall(sock1fd);
                this.close_syscall(sock2fd);
                return syscall_error(sockerrno, "socketpair", "The libc call to accept within socketpair failed!");
            }
            this.close_syscall(sock1fd);
    
            sv.sock1 = sock2fd;
            sv.sock2 = accret;
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

    // all this does is send the net_devs data in a string to libc, where we will later parse and 
    // alloc into getifaddrs structs
    pub fn getifaddrs_syscall(&self, buf: *mut u8, count: usize) -> i32 {
        if NET_IFADDRS_STR.len() < count {
            interface::fill(buf, NET_IFADDRS_STR.len(), &NET_IFADDRS_STR.as_bytes().to_vec());
            0 // return success
        }
        else {
            return syscall_error(Errno::EOPNOTSUPP, "getifaddrs", "invalid ifaddrs length");
        }
    }
}
