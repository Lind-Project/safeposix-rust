#![allow(dead_code)]
// Network related system calls
// outlines and implements all of the networking system calls that are being emulated/faked in Lind

use crate::interface;
use crate::interface::errnos::{Errno, syscall_error};
use super::net_constants::*;
use super::fs_constants::*;
use super::sys_constants::*;
use crate::safeposix::cage::{*, FileDescriptor::*};
use crate::safeposix::filesystem::*;
use crate::safeposix::net::*;

impl Cage {
    fn _socket_initializer(&self, domain: i32, socktype: i32, protocol: i32, blocking: bool, cloexec: bool, conn: ConnState) -> SocketDesc {
        let flags = if blocking {O_NONBLOCK} else {0} | if cloexec {O_CLOEXEC} else {0};


        let sockfd = SocketDesc {
            flags: flags,
            handle: interface::RustRfc::new(interface::RustLock::new(Self::mksockhandle(domain, socktype, protocol, conn, flags))),
            advlock: interface::RustRfc::new(interface::AdvisoryLock::new()),
        }; //currently on failure to create handle we create successfully but it's corrupted, change?

        return sockfd;
    }

    fn _socket_inserter(&self, sockfd: FileDescriptor) -> i32 {
        let (fd, guardopt) = self.get_next_fd(None);
        if fd < 0 { return fd }
        let fdoption = &mut *guardopt.unwrap();
        let _insertval = fdoption.insert(sockfd);
        return fd;
    }

    fn _implicit_bind(&self, sockhandle: &mut SocketHandle, domain: i32) -> i32 {
        if sockhandle.localaddr.is_none() {
            let localaddr = match Self::assign_new_addr(sockhandle, domain, sockhandle.protocol & (1 << SO_REUSEPORT) != 0) {
                Ok(a) => a,
                Err(e) => return e,
            };

            let bindret = self.bind_inner_socket(sockhandle, &localaddr, true);

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

        match real_socktype {

            SOCK_STREAM => {
                //SOCK_STREAM defaults to TCP for protocol, otherwise protocol is unsupported
                let newprotocol = if protocol == 0 {IPPROTO_TCP} else {protocol};

                if newprotocol != IPPROTO_TCP {
                    return syscall_error(Errno::EOPNOTSUPP, "socket", "The only SOCK_STREAM implemented is TCP. Unknown protocol input.");
                }
                match domain {
                    PF_INET | PF_UNIX => {
                        let sockfdobj = self._socket_initializer(domain, socktype, newprotocol, nonblocking, cloexec, ConnState::NOTCONNECTED);
                        return self._socket_inserter(Socket(sockfdobj));
                    }
                    _ => {
                        return syscall_error(Errno::EOPNOTSUPP, "socket", "trying to use an unimplemented domain");
                    }
                }
            }

            SOCK_DGRAM => {
                //SOCK_DGRAM defaults to UDP for protocol, otherwise protocol is unsuported
                let newprotocol = if protocol == 0 {IPPROTO_UDP} else {protocol};

                if newprotocol != IPPROTO_UDP {
                    return syscall_error(Errno::EOPNOTSUPP, "socket", "The only SOCK_DGRAM implemented is UDP. Unknown protocol input.");
                }
                match domain {
                    PF_INET | PF_UNIX => {
                        let sockfdobj = self._socket_initializer(domain, socktype, newprotocol, nonblocking, cloexec, ConnState::NOTCONNECTED);
                        return self._socket_inserter(Socket(sockfdobj));
                    }
                    _ => {
                        return syscall_error(Errno::EOPNOTSUPP, "socket", "trying to use an unimplemented domain");
                    }
                }
            }

            _ => {
                return syscall_error(Errno::EOPNOTSUPP, "socket", "trying to use an unimplemented socket type");
            }

        }
    }

    //creates a sockhandle if none exists, otherwise this is a no-op
    pub fn force_innersocket(sockhandle: &mut SocketHandle) {
        if let None = sockhandle.innersocket {
            let thissock = interface::Socket::new(sockhandle.domain, sockhandle.socktype, sockhandle.protocol);

            for reuse in [SO_REUSEPORT, SO_REUSEADDR] {
                if sockhandle.options & (1 << reuse) == 0 {continue;}
                let sockret = thissock.setsockopt(SOL_SOCKET, reuse, 1);
                if sockret < 0 {
                    panic!("Cannot handle failure in setsockopt on socket creation");
                }
            }

            sockhandle.innersocket = Some(thissock);
        };
    }

    //we assume we've converted into a RustSockAddr in the dispatcher
    pub fn bind_syscall(&self, fd: i32, localaddr: &interface::GenSockaddr) -> i32 {
        self.bind_inner(fd, localaddr, false)
    }

    fn bind_inner_socket(&self, sockhandle: &mut SocketHandle, localaddr: &interface::GenSockaddr, prereserved: bool) -> i32 {
        if localaddr.get_family() != sockhandle.domain as u16 {
            return syscall_error(Errno::EINVAL, "bind", "An address with an invalid family for the given domain was specified");
        }
    
        if sockhandle.localaddr.is_some() {
            return syscall_error(Errno::EINVAL, "bind", "The socket is already bound to an address");
        }
    
        let mut newsockaddr = localaddr.clone();
    
        let res = match sockhandle.domain {
            AF_UNIX => self.bind_inner_socket_unix(sockhandle, &mut newsockaddr),
            AF_INET | AF_INET6 => self.bind_inner_socket_inet(sockhandle, &mut newsockaddr, prereserved),
            _ => {return syscall_error(Errno::EINVAL, "bind", "Unsupported domain provided");}
        };

        if res != 0 {
            return res; // some error occured
        }

        sockhandle.localaddr = Some(newsockaddr);
    
        0
    }
    
    fn bind_inner_socket_unix(&self, sockhandle: &mut SocketHandle, newsockaddr: &mut interface::GenSockaddr) -> i32 {
        // Unix Sockets
        let path = newsockaddr.path();
        //Check that path is not empty
        if path.len() == 0 {return syscall_error(Errno::ENOENT, "bind", "given path was null");}
        let truepath = normpath(convpath(path), self);
    
        match metawalkandparent(truepath.as_path()) {
            //If neither the file nor parent exists
            (None, None) => {return syscall_error(Errno::ENOENT, "bind", "a directory component in pathname does not exist or is a dangling symbolic link"); }
            //If the file doesn't exist but the parent does
            (None, Some(pardirinode)) => {
                let filename = truepath.file_name().unwrap().to_str().unwrap().to_string(); //for now we assume this is sane, but maybe this should be checked later
    
                //this may end up skipping an inode number in the case of ENOTDIR, but that's not catastrophic
                let newinodenum = FS_METADATA.nextinode.fetch_add(1, interface::RustAtomicOrdering::Relaxed); //fetch_add returns the previous value, which is the inode number we want
                let newinode;
    
                if let Inode::Dir(ref mut dir) = *(FS_METADATA.inodetable.get_mut(&pardirinode).unwrap()) {
                    let mode = (dir.mode | S_FILETYPEFLAGS as u32) & S_IRWXA;
                    let effective_mode = S_IFSOCK as u32 | mode;
    
                    let time = interface::timestamp(); //We do a real timestamp now
                    newinode = Inode::Socket(SocketInode {
                        size: 0, uid: DEFAULT_UID, gid: DEFAULT_GID,
                        mode: effective_mode, linkcount: 1, refcount: 1,
                        atime: time, ctime: time, mtime: time,
                    });
    
                    dir.filename_to_inode_dict.insert(filename.clone(), newinodenum);
                    dir.linkcount += 1;
                } else {
                    return syscall_error(Errno::ENOTDIR, "bind", "unix domain socket path made socket address child of non-directory file");
                }
                sockhandle.unix_info = Some(UnixSocketInfo {mode: S_IFSOCK | 0666, sendpipe: None, path: truepath.clone(), receivepipe: None, inode: newinodenum});  
                NET_METADATA.domsock_paths.insert(truepath);
                FS_METADATA.inodetable.insert(newinodenum, newinode);
            }
            (Some(_inodenum), ..) => { return syscall_error(Errno::EADDRINUSE, "bind", "Address already in use"); }
        }
    
        0
    }
    
    fn bind_inner_socket_inet(&self, sockhandle: &mut SocketHandle, newsockaddr: &mut interface::GenSockaddr, prereserved: bool) -> i32 {
        // INET Sockets
        let intent_to_rebind = sockhandle.options & (1 << SO_REUSEPORT) != 0;
        Self::force_innersocket(sockhandle);
    
        let newlocalport = if prereserved {
            newsockaddr.port()
        } else {
            let localout = NET_METADATA._reserve_localport(newsockaddr.addr(), newsockaddr.port(), sockhandle.protocol, sockhandle.domain, intent_to_rebind);
            if let Err(errnum) = localout {return errnum;}
            localout.unwrap()
        };
    
        newsockaddr.set_port(newlocalport);
        let bindret = sockhandle.innersocket.as_ref().unwrap().bind(&newsockaddr);
    
        if bindret < 0 {
            match Errno::from_discriminant(interface::get_errno()) {
                Ok(i) => {return syscall_error(i, "bind", "The libc call to bind failed!");},
                Err(()) => panic!("Unknown errno value from socket bind returned!"),
            };
        }
    
        0
    }
    

    pub fn bind_inner(&self, fd: i32, localaddr: &interface::GenSockaddr, prereserved: bool) -> i32 {
        let checkedfd = self.get_filedescriptor(fd).unwrap();
        let mut unlocked_fd = checkedfd.write();
        if let Some(filedesc_enum) = &mut *unlocked_fd {
            match filedesc_enum {
                Socket(ref mut sockfdobj) => {
                    let sock_tmp = sockfdobj.handle.clone();
                    let mut sockhandle = sock_tmp.write();
                    self.bind_inner_socket(&mut *sockhandle, localaddr, prereserved)
                }
                _ => {
                    syscall_error(Errno::ENOTSOCK, "bind", "file descriptor refers to something other than a socket")
                }
            }
        } else {
            syscall_error(Errno::EBADF, "bind", "invalid file descriptor")
        }
    }

    fn assign_new_addr_unix(sockhandle: &SocketHandle) -> interface::GenSockaddr {
        if let Some(addr) = sockhandle.localaddr.clone() {
            addr
        } else {
            let path = interface::gen_ud_path();
            let newremote = interface::GenSockaddr::Unix(interface::new_sockaddr_unix(AF_UNIX as u16, path.as_bytes()));
            newremote
        } 
    }

    fn assign_new_addr(sockhandle: &SocketHandle, domain: i32, rebindability: bool) -> Result<interface::GenSockaddr, i32> {
        if let Some(addr) = &sockhandle.localaddr {
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
                    newremote.set_port(match NET_METADATA._reserve_localport(addr.clone(), 0, sockhandle.protocol, sockhandle.domain, rebindability) {
                        Ok(portnum) => portnum,
                        Err(errnum) => return Err(errnum),
                    });
                }
                AF_INET6 => { 
                    newremote = interface::GenSockaddr::V6(interface::SockaddrV6::default());
                    let addr = interface::GenIpaddr::V6(interface::V6Addr::default());
                    newremote.set_addr(addr);
                    newremote.set_family(AF_INET6 as u16);
                    newremote.set_port(match NET_METADATA._reserve_localport(addr.clone(), 0, sockhandle.protocol, sockhandle.domain, rebindability) {
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
        let checkedfd = self.get_filedescriptor(fd).unwrap();
        let mut unlocked_fd = checkedfd.write();
        if let Some(filedesc_enum) = &mut *unlocked_fd {
            match filedesc_enum {
                Socket(ref mut sockfdobj) => {
                    let sock_tmp = sockfdobj.handle.clone();
                    let mut sockhandle = sock_tmp.write();
                    if remoteaddr.get_family() != sockhandle.domain as u16 {
                        return syscall_error(Errno::EINVAL, "connect", "An address with an invalid family for the given domain was specified");
                    }
                    match sockhandle.protocol {
                        IPPROTO_UDP => return self.connect_udp(&mut *sockhandle, remoteaddr),
                        IPPROTO_TCP => return self.connect_tcp(&mut *sockhandle, sockfdobj, remoteaddr),
                        _ => return syscall_error(Errno::EOPNOTSUPP, "connect", "Unknown protocol in connect"),
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
    
    fn connect_udp(&self, sockhandle: &mut SocketHandle, remoteaddr: &interface::GenSockaddr) -> i32 {
        //for UDP, just set the addresses and return
        //we don't need to check connection state for UDP, it's connectionless!
        sockhandle.remoteaddr = Some(remoteaddr.clone());
        match sockhandle.localaddr {
            Some(_) => return 0,
            None => {
                let localaddr = match Self::assign_new_addr(&*sockhandle, sockhandle.domain, sockhandle.protocol & (1 << SO_REUSEPORT) != 0) {
                    Ok(a) => a,
                    Err(e) => return e,
                };
    
                return self.bind_inner_socket(&mut *sockhandle, &localaddr, true);
            }
        };
    }
    
    fn connect_tcp(&self, sockhandle: &mut SocketHandle, sockfdobj: &mut SocketDesc, remoteaddr: &interface::GenSockaddr) -> i32 {
        // TCP connection logic
        if sockhandle.state != ConnState::NOTCONNECTED {
            return syscall_error(Errno::EISCONN, "connect", "The descriptor is already connected");
        }
    
        match sockhandle.domain {
            AF_UNIX => self.connect_tcp_unix(&mut *sockhandle, sockfdobj, remoteaddr),
            AF_INET | AF_INET6 => self.connect_tcp_inet(&mut *sockhandle, remoteaddr),
            _ => {return syscall_error(Errno::EINVAL, "connect", "Unsupported domain provided")},
        }
    }
    
    fn connect_tcp_unix(&self, sockhandle: &mut SocketHandle, sockfdobj: &mut SocketDesc, remoteaddr: &interface::GenSockaddr) -> i32 {
        // TCP domain socket logic
        if let None = sockhandle.localaddr {
            let localaddr = Self::assign_new_addr_unix(&sockhandle);
            self.bind_inner_socket(&mut *sockhandle, &localaddr, false); 
        }
        let remotepathbuf = normpath(convpath(remoteaddr.path().clone()), self);

        // try to get and hold reference to the key-value pair, so other process can't alter it
        let path_ref = NET_METADATA.domsock_paths.get(&remotepathbuf);
        // if the entry doesn't exist, return an error.
        if path_ref.is_none() {
            return syscall_error(Errno::ENOENT, "connect", "not valid unix domain path");
        }
    
        let (pipe1, pipe2) = create_unix_sockpipes();
    
        sockhandle.remoteaddr = Some(remoteaddr.clone());
        sockhandle.unix_info.as_mut().unwrap().sendpipe = Some(pipe1.clone());
        sockhandle.unix_info.as_mut().unwrap().receivepipe = Some(pipe2.clone());
    
        let connvar = if sockfdobj.flags & O_NONBLOCK != 0 { 
            Some(interface::RustRfc::new(ConnCondVar::new()))
        } else { None };
    
        // receive_pipe and send_pipe need to be swapped here
        // because the receive_pipe and send_pipe are opposites between the 
        // sender and receiver. Swapping here also means we do not need to swap in
        // accept.
        let entry = DomsockTableEntry {
            sockaddr: sockhandle.localaddr.unwrap().clone(),
            receive_pipe: Some(pipe1.clone()).unwrap(),
            send_pipe: Some(pipe2.clone()).unwrap(),
            cond_var: connvar.clone(),
        };
        NET_METADATA.domsock_accept_table.insert(remotepathbuf, entry);
        sockhandle.state = ConnState::CONNECTED;
        if sockfdobj.flags & O_NONBLOCK != 0 { connvar.unwrap().wait(); }
        return 0; 
    }
    
    fn connect_tcp_inet(&self, sockhandle: &mut SocketHandle, remoteaddr: &interface::GenSockaddr) -> i32 {
        // TCP inet domain logic
        //for TCP, actually create the internal socket object and connect it
        let remoteclone = remoteaddr.clone();
    
        if sockhandle.state != ConnState::NOTCONNECTED {
            return syscall_error(Errno::EISCONN, "connect", "The descriptor is already connected");
        }
    
    
        if let None = sockhandle.localaddr {
            Self::force_innersocket(sockhandle);
    
            let localaddr = match Self::assign_new_addr(&*sockhandle, sockhandle.domain, sockhandle.protocol & (1 << SO_REUSEPORT) != 0) {
                Ok(a) => a,
                Err(e) => return e,
            };
            let bindret = sockhandle.innersocket.as_ref().unwrap().bind(&localaddr);
            if bindret < 0 {
                sockhandle.localaddr = Some(localaddr);
                match Errno::from_discriminant(interface::get_errno()) {
                    Ok(i) => {return syscall_error(i, "connect", "The libc call to bind within connect failed");},
                    Err(()) => panic!("Unknown errno value from socket bind within connect returned!"),
                };
            }
        } 
    
        let mut inprogress = false;
        let connectret = sockhandle.innersocket.as_ref().unwrap().connect(&remoteclone);
        if connectret < 0 {
            match Errno::from_discriminant(interface::get_errno()) {
                Ok(i) => {
                    if i == Errno::EINPROGRESS { inprogress = true; }
                    else { return syscall_error(i, "connect", "The libc call to connect failed!") };
                },
                Err(()) => panic!("Unknown errno value from socket connect returned!"),
            };
        }
    
        sockhandle.state = ConnState::CONNECTED;
        sockhandle.remoteaddr = Some(remoteaddr.clone());
        sockhandle.errno = 0;
        if inprogress {
            sockhandle.state = ConnState::INPROGRESS;
            return syscall_error(Errno::EINPROGRESS, "connect", "The libc call to connect is in progress.");
        }
        else {
            return 0;
        }
    }    

    fn mksockhandle(domain: i32, socktype: i32, protocol: i32, conn: ConnState, options: i32) -> SocketHandle {

        SocketHandle {
            innersocket: None,
            options: options,
            state: conn,
            protocol: protocol,
            
            domain: domain,
            last_peek: interface::RustDeque::new(),
            localaddr: None,
            remoteaddr: None,

            unix_info: None,
            socktype: socktype,
            sndbuf: 131070, //buffersize, which is only used by getsockopt
            rcvbuf: 262140, //buffersize, which is only used by getsockopt
            errno: 0,
        }
    }

    pub fn sendto_syscall(&self, fd: i32, buf: *const u8, buflen: usize, flags: i32, dest_addr: &interface::GenSockaddr) -> i32 {
        //if ip and port are not specified, shunt off to send
        if dest_addr.port() == 0 && dest_addr.addr().is_unspecified() {
            return self.send_syscall(fd, buf, buflen, flags);
        }

        let checkedfd = self.get_filedescriptor(fd).unwrap();
        let unlocked_fd = checkedfd.write();
        if let Some(filedesc_enum) = &*unlocked_fd {
            match filedesc_enum {
                Socket(sockfdobj) => {
                    let sock_tmp = sockfdobj.handle.clone();
                    let mut sockhandle = sock_tmp.write();

                    // check if this is a domain socket
                    if sockhandle.domain == AF_UNIX {
                        return syscall_error(Errno::EISCONN, "sendto", "The descriptor is connection-oriented");
                    } 

                    if dest_addr.get_family() != sockhandle.domain as u16 {
                        return syscall_error(Errno::EINVAL, "sendto", "An address with an invalid family for the given domain was specified");
                    }
                    if (flags & !MSG_NOSIGNAL) != 0 {
                        return syscall_error(Errno::EOPNOTSUPP, "sendto", "The flags are not understood!");
                    }

                    if sockhandle.state != ConnState::NOTCONNECTED {
                        return syscall_error(Errno::EISCONN, "sendto", "The descriptor is connected");
                    }

                    match sockhandle.protocol {
                        //Sendto doesn't make sense for the TCP protocol, it's connection oriented
                        IPPROTO_TCP => {
                            return syscall_error(Errno::EISCONN, "sendto", "The descriptor is connection-oriented");
                        }

                        IPPROTO_UDP => {
                            let tmpdest = *dest_addr;
                            let ibindret = self._implicit_bind(&mut *sockhandle, tmpdest.get_family() as i32);
                            if ibindret < 0 {
                                return ibindret;
                            }

                            //unwrap ok because we implicit_bind_right before
                            let sockret = sockhandle.innersocket.as_ref().unwrap().sendto(buf, buflen, Some(dest_addr));

                            //we don't mind if this fails for now and we will just get the error
                            //from calling sendto
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
        let checkedfd = self.get_filedescriptor(fd).unwrap();
        let mut unlocked_fd = checkedfd.write();
        if let Some(filedesc_enum) = &mut *unlocked_fd {
            match filedesc_enum {
                Socket(ref mut sockfdobj) => {

                    let sock_tmp = sockfdobj.handle.clone();
                    let sockhandle = sock_tmp.write();

                    if (flags & !MSG_NOSIGNAL) != 0 {
                        return syscall_error(Errno::EOPNOTSUPP, "send", "The flags are not understood!");
                    }

                    // check if this is a domain socket
                    let socket_type = sockhandle.domain;
                    match socket_type {
                        AF_UNIX => match sockhandle.protocol {
                            IPPROTO_TCP => {

                                if sockhandle.state != ConnState::CONNECTED {
                                    return syscall_error(Errno::ENOTCONN, "send", "The descriptor is not connected");
                                }

                                // get the socket pipe, write to it, and return bytes written
                                if let Some(sockinfo) = &sockhandle.unix_info {
                                    let mut nonblocking = false;
                                    if sockfdobj.flags & O_NONBLOCK != 0 { nonblocking = true;}
                                    let retval = match sockinfo.sendpipe.as_ref() {
                                        Some(sendpipe) => sendpipe.write_to_pipe(buf, buflen, nonblocking) as i32,
                                        None => {
                                            return syscall_error(Errno::EAGAIN, "write", "there is no data available right now, try again later");
                                        }
                                    };
                                    if retval < 0 { return syscall_error(Errno::EAGAIN, "write", "there is no data available right now, try again later") }
                                    else {
                                        return retval;
                                    }
                                }

                                return syscall_error(Errno::EINPROGRESS, "connect", "The libc call to connect failed!");
                            }
                            _ => {
                                return syscall_error(Errno::EOPNOTSUPP, "send", "Unkown protocol in send");
                            }
                        },
                        // for inet
                        AF_INET | AF_INET6 => match sockhandle.protocol {
                            IPPROTO_TCP => {
                                if (sockhandle.state != ConnState::CONNECTED) && (sockhandle.state != ConnState::CONNWRONLY) {
                                    return syscall_error(Errno::ENOTCONN, "send", "The descriptor is not connected");
                                }
    
                                //because socket must be connected it must have an inner socket
                                let retval = sockhandle.innersocket.as_ref().unwrap().sendto(buf, buflen, None);
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
                                let remoteaddr = match &sockhandle.remoteaddr {
                                    Some(x) => x.clone(),
                                    None => {return syscall_error(Errno::ENOTCONN, "send", "The descriptor is not connected");},
                                };
                                drop(unlocked_fd);
                                drop(sockhandle);
                                //send from a udp socket is just shunted off to sendto with the remote address set
                                return self.sendto_syscall(fd, buf, buflen, flags, &remoteaddr);
                            }
    
                            _ => {
                                return syscall_error(Errno::EOPNOTSUPP, "send", "Unkown protocol in send");
                            }
                        },
                        _ => {return syscall_error(Errno::EINVAL, "connect", "Unsupported domain provided")},
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
                let sock_tmp = sockfdobj.handle.clone();
                    let mut sockhandle = sock_tmp.write();
                    match sockhandle.protocol {
                        IPPROTO_TCP => return self.recv_common_inner_tcp(&mut sockhandle, sockfdobj, buf, buflen, flags, addr),
                        IPPROTO_UDP => return self.recv_common_inner_udp(&mut sockhandle, sockfdobj, buf, buflen, addr),

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

    fn recv_common_inner_tcp(&self, sockhandle: &mut interface::RustLockWriteGuard<SocketHandle>, sockfdobj: &mut SocketDesc, buf: *mut u8, buflen: usize, flags: i32, addr: &mut Option<&mut interface::GenSockaddr>) -> i32 {

        if (sockhandle.state != ConnState::CONNECTED) && (sockhandle.state != ConnState::CONNRDONLY) {
            return syscall_error(Errno::ENOTCONN, "recvfrom", "The descriptor is not connected");
        }

        let mut newbuflen = buflen;
        let mut newbufptr = buf;

        //if we have peeked some data before, fill our buffer with that data before moving on
        if !sockhandle.last_peek.is_empty() {
            let bytecount = interface::rust_min(sockhandle.last_peek.len(), newbuflen);
            interface::copy_fromrustdeque_sized(buf, bytecount, &sockhandle.last_peek);
            newbuflen -= bytecount;
            newbufptr = newbufptr.wrapping_add(bytecount);

            //if we're not still peeking data, consume the data we peeked from our peek buffer
            //and if the bytecount is more than the length of the peeked data, then we remove the entire
            //buffer
            if flags & MSG_PEEK == 0 {
                let len = sockhandle.last_peek.len();
                sockhandle.last_peek.drain(..(
                    if bytecount > len {len} else {bytecount}
                ));
            }

            if newbuflen == 0 {
                //if we've filled all of the buffer with peeked data, return
                return bytecount as i32;
            }
        }

        let bufleft = newbufptr;
        let buflenleft = newbuflen;
        let mut retval;

        if sockhandle.domain == AF_UNIX {
             // get the remote socket pipe, read from it, and return bytes read
            let mut nonblocking = false;
            if sockfdobj.flags & O_NONBLOCK != 0 { nonblocking = true;}  
            loop {
                let sockinfo = &sockhandle.unix_info.as_ref().unwrap();
                let receivepipe = sockinfo.receivepipe.as_ref().unwrap();
                retval = receivepipe.read_from_pipe(bufleft, buflenleft, nonblocking) as i32;   
                if retval < 0 {
                    //If we have already read from a peek but have failed to read more, exit!
                    if buflen != buflenleft { return (buflen - buflenleft) as i32; }
                    if sockfdobj.flags & O_NONBLOCK == 0 && retval == -(Errno::EAGAIN as i32) {
                        if self.cancelstatus.load(interface::RustAtomicOrdering::Relaxed) {
                            // if the cancel status is set in the cage, we trap around a cancel point
                            // until the individual thread is signaled to cancel itself
                            loop{interface::cancelpoint(self.cageid)};
                        }
                        // in order to prevent deadlock
                        interface::RustLockWriteGuard::<SocketHandle>::bump(sockhandle);
                        continue;
                    }
                    else {
                        //if not EAGAIN, return the error
                        return retval;
                    }
                }
                break;
            }
        }
        else {
            loop { // we loop here so we can cancel blocking recvs
                //socket must be connected so unwrap ok
                if sockfdobj.flags & O_NONBLOCK != 0 {
                    retval = sockhandle.innersocket.as_ref().unwrap().recvfrom_nonblocking(bufleft, buflenleft, addr);
                } else {
                    retval = sockhandle.innersocket.as_ref().unwrap().recvfrom(bufleft, buflenleft, addr);
                }

                if retval < 0 {
                    //If we have already read from a peek but have failed to read more, exit!
                    if buflen != buflenleft {
                        return (buflen - buflenleft) as i32;
                    }

                    match Errno::from_discriminant(interface::get_errno()) {
                        Ok(i) => {
                            //We have the recieve timeout set to every one second, so
                            //if our blocking socket ever returns EAGAIN, it must be
                            //the case that this recv timeout was exceeded, and we
                            //should thus not treat this as a failure in our emulated
                            //socket; see comment in Socket::new in interface/comm.rs
                            if sockfdobj.flags & O_NONBLOCK == 0 && i == Errno::EAGAIN {
                                if self.cancelstatus.load(interface::RustAtomicOrdering::Relaxed) {
                                    // if the cancel status is set in the cage, we trap around a cancel point
                                    // until the individual thread is signaled to cancel itself
                                    loop { interface::cancelpoint(self.cageid); }
                                }
                                interface::RustLockWriteGuard::<SocketHandle>::bump(sockhandle);
                                continue; // EAGAIN, try again
                            }

                            return syscall_error(i, "recvfrom", "Internal call to recvfrom failed");
                        },
                        Err(()) => panic!("Unknown errno value from socket recvfrom returned!"),
                    };
                }
                break; // we're okay to move on
            }
        }
        let totalbyteswritten = (buflen - buflenleft) as i32 + retval;

        if flags & MSG_PEEK != 0 {
            //extend from the point after we read our previously peeked bytes
            interface::extend_fromptr_sized(newbufptr, retval as usize, &mut sockhandle.last_peek);
        }

        return totalbyteswritten;
    }

    fn recv_common_inner_udp(&self, sockhandle: &mut interface::RustLockWriteGuard<SocketHandle>, sockfdobj: &mut SocketDesc, buf: *mut u8, buflen: usize, addr: &mut Option<&mut interface::GenSockaddr>) -> i32 {
        let binddomain = if let Some(baddr) = addr {
            baddr.get_family() as i32
       } else { AF_INET };
       
       let ibindret = self._implicit_bind(&mut *sockhandle, binddomain);
       if ibindret < 0 {
           return ibindret;
       }

       loop { // loop for blocking sockets
           //if the remoteaddr is set and addr is not, use remoteaddr
           //unwrap is ok because of implicit bind
           let retval = if let (None, Some(ref mut remoteaddr)) =  (&addr, sockhandle.remoteaddr) {
               sockhandle.innersocket.as_ref().unwrap().recvfrom(buf, buflen, &mut Some(remoteaddr))
           } else {
               sockhandle.innersocket.as_ref().unwrap().recvfrom(buf, buflen, addr)
           };

           if retval < 0 {
               match Errno::from_discriminant(interface::get_errno()) {
                   Ok(i) => {
                       if sockfdobj.flags & O_NONBLOCK == 0 && i == Errno::EAGAIN {
                           if self.cancelstatus.load(interface::RustAtomicOrdering::Relaxed) {
                               // if the cancel status is set in the cage, we trap around a cancel point
                               // until the individual thread is signaled to cancel itself
                               loop { interface::cancelpoint(self.cageid); }
                           }
                           interface::RustLockWriteGuard::<SocketHandle>::bump(sockhandle);
                           continue; //received EAGAIN on blocking socket, try again
                       }
                       return syscall_error(i, "recvfrom", "Internal call to recvfrom failed");
                   },
                   Err(()) => panic!("Unknown errno value from socket recvfrom returned!"),
               };
               
           } else {
               return retval; // we can proceed
           }
       }
    }

    pub fn recv_common(&self, fd: i32, buf: *mut u8, buflen: usize, flags: i32, addr: &mut Option<&mut interface::GenSockaddr>) -> i32 {
        let checkedfd = self.get_filedescriptor(fd).unwrap();
        let mut unlocked_fd = checkedfd.write();
        if let Some(ref mut filedesc_enum) = &mut *unlocked_fd {
            return self.recv_common_inner(filedesc_enum, buf, buflen, flags, addr);
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
        let checkedfd = self.get_filedescriptor(fd).unwrap();
        let mut unlocked_fd = checkedfd.write();
        if let Some(filedesc_enum) = &mut *unlocked_fd {
            match filedesc_enum {
                Socket(ref mut sockfdobj) => {
                    //get or create the socket and bind it before listening
                    let sock_tmp = sockfdobj.handle.clone();
                    let mut sockhandle = sock_tmp.write();

                    match sockhandle.state {
                        ConnState::LISTEN => {
                            return 0; //Already done!
                        }

                        ConnState::CONNECTED | ConnState::CONNRDONLY | ConnState::CONNWRONLY | ConnState::INPROGRESS => {
                            return syscall_error(Errno::EOPNOTSUPP, "listen", "We don't support closing a prior socket connection on listen");
                        }

                        ConnState::NOTCONNECTED => {
                            if sockhandle.protocol != IPPROTO_TCP {
                                return syscall_error(Errno::EOPNOTSUPP, "listen", "This protocol doesn't support listening");
                            }

                            // simple if it's a domain socket
                            if sockhandle.domain == AF_UNIX {
                                sockhandle.state = ConnState::LISTEN;
                                return 0;
                            }

                            let mut ladr;
                            match sockhandle.localaddr {
                                Some(sla) => {
                                    ladr = sla.clone();

                                    if NET_METADATA.listening_port_set.contains(&mux_port(ladr.addr().clone(), ladr.port(), sockhandle.domain, TCPPORT)) {
                                        match NET_METADATA._get_available_tcp_port(ladr.addr().clone(), sockhandle.domain, sockhandle.options & (1 << SO_REUSEPORT) != 0) {
                                            Ok(port) => ladr.set_port(port),
                                            Err(i) => return i,
                                        }
                                    }
                                }
                                None => {
                                    let shd = sockhandle.domain as i32;
                                    let ibindret = self._implicit_bind(&mut *sockhandle, shd);
                                    if ibindret < 0 {
                                        match Errno::from_discriminant(interface::get_errno()) {
                                            Ok(i) => {return syscall_error(i, "listen", "The libc call to bind within listen failed");},
                                            Err(()) => panic!("Unknown errno value from socket bind within listen returned!"),
                                        };
                                    }
                                    
                                    ladr = sockhandle.localaddr.unwrap().clone(); //must have been populated by implicit bind
                                }
                            }
                                
                            let porttuple = mux_port(ladr.addr().clone(), ladr.port(), sockhandle.domain, TCPPORT);

                            NET_METADATA.listening_port_set.insert(porttuple);
                            sockhandle.state = ConnState::LISTEN;

                            let listenret = sockhandle.innersocket.as_ref().unwrap().listen(5); //default backlog in repy for whatever reason, we replicate it
                            if listenret < 0 {
                                let lr = match Errno::from_discriminant(interface::get_errno()) {
                                    Ok(i) => syscall_error(i, "listen", "The libc call to listen failed!"),
                                    Err(()) => panic!("Unknown errno value from socket listen returned!"),
                                };
                                NET_METADATA.listening_port_set.remove(&mux_port(ladr.addr().clone(), ladr.port(), sockhandle.domain, TCPPORT));
                                sockhandle.state = ConnState::NOTCONNECTED;
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
            SHUT_RDWR | SHUT_RD | SHUT_WR => {
                return Self::_cleanup_socket(self, fd, how);
            }
            _ => {
                //See http://linux.die.net/man/2/shutdown for nuance to this error
                return syscall_error(Errno::EINVAL, "netshutdown", "the shutdown how argument passed is not supported");
            }
        }
    }

    pub fn _cleanup_socket_inner_helper(sockhandle: &mut SocketHandle, how: i32, shutdown: bool) -> i32 {
        // we need to do a bunch of actual socket cleanup for INET sockets
        if sockhandle.domain != AF_UNIX { 
            let mut releaseflag = false;
            if let Some(ref sobj) = sockhandle.innersocket {
                if shutdown {
                    let shutresult = sobj.shutdown(how);

                    if shutresult < 0 {
                        match Errno::from_discriminant(interface::get_errno()) {
                            Ok(i) => {return syscall_error(i, "shutdown", "The libc call to setsockopt failed!");},
                            Err(()) => panic!("Unknown errno value from setsockopt returned!"),
                        };
                    }

                    match how {
                        SHUT_RD => {
                            if sockhandle.state == ConnState::CONNRDONLY { releaseflag = true; }
                        }
                        SHUT_WR => {
                            if sockhandle.state == ConnState::CONNWRONLY { releaseflag = true; }
                        }
                        SHUT_RDWR => {
                            releaseflag = true;
                        }
                        _ => {
                            //See http://linux.die.net/man/2/shutdown for nuance to this error
                            return syscall_error(Errno::EINVAL, "netshutdown", "the shutdown how argument passed is not supported");
                        }
                    }
                } else {
                    //Reaching this means that the socket is closed. Removing the sockobj
                    //indicates that the sockobj will drop, and therefore close
                    releaseflag = true;
                    sockhandle.innersocket = None;
                }
            }

            if releaseflag {
                if let Some(localaddr) = sockhandle.localaddr.as_ref().clone() {
                    //move to end
                    let release_ret_val = NET_METADATA._release_localport(localaddr.addr(), localaddr.port(), sockhandle.protocol, sockhandle.domain);
                    sockhandle.localaddr = None;
                    if let Err(e) = release_ret_val {return e;}
                    }
                }
            }     

            // now change the connections for all socket types
            match how {
                SHUT_RD => {
                    sockhandle.state = ConnState::CONNWRONLY;
                }
                SHUT_WR => {
                    sockhandle.state = ConnState::CONNRDONLY;
                }
                SHUT_RDWR => {
                    sockhandle.state = ConnState::NOTCONNECTED;
                }
                _ => {
                    //See http://linux.die.net/man/2/shutdown for nuance to this error
                    return syscall_error(Errno::EINVAL, "netshutdown", "the shutdown how argument passed is not supported");
                }
            }      

            return 0;
    }

    pub fn _cleanup_socket_inner(&self, filedesc: &mut FileDescriptor, how: i32, shutdown: bool) -> i32 {
        if let Socket(sockfdobj) = filedesc {
            let sock_tmp = sockfdobj.handle.clone();
            let mut sockhandle = sock_tmp.write();

            Self::_cleanup_socket_inner_helper(&mut *sockhandle, how, shutdown)
        } else {
            syscall_error(Errno::ENOTSOCK, "cleanup socket", "file descriptor is not a socket")
        }
    }

    pub fn _cleanup_socket(&self, fd: i32, how: i32) -> i32 {
        let checkedfd = self.get_filedescriptor(fd).unwrap();
        let mut unlocked_fd = checkedfd.write();
        if let Some(ref mut filedesc_enum) = &mut *unlocked_fd {
            let inner_result = self._cleanup_socket_inner(filedesc_enum, how, true);
            if inner_result < 0 {
                return inner_result;
            }

            if how == SHUT_RDWR {
                let _discarded_fd = unlocked_fd.take();
            }
        } else {
            return syscall_error(Errno::EBADF, "cleanup socket", "invalid file descriptor");
        }

        return 0;
    }

    //calls accept on the socket object with value depending on ipv4 or ipv6
    //There may be a bug with nonblocking accept with fds not being removed on error
    // NEED REFACTOR
    pub fn accept_syscall(&self, fd: i32, addr: &mut interface::GenSockaddr) -> i32 {
        let checkedfd = self.get_filedescriptor(fd).unwrap();
        let mut unlocked_fd = checkedfd.write();
        if let Some(filedesc_enum) = &mut *unlocked_fd {

            //we need to reserve this fd early to make sure that we don't need to
            //error out later so we perform get_next_fd, and populate it at the end

            let (newfd, guardopt) = self.get_next_fd(None);
            if newfd < 0 { return fd }
            let newfdoption: &mut Option<FileDescriptor> = &mut *guardopt.unwrap();

            match filedesc_enum {
                Socket(ref mut sockfdobj) => {
                    let sock_tmp = sockfdobj.handle.clone();
                    let mut sockhandle = sock_tmp.read();

                    // check if domain socket
                    match sockhandle.domain {
                        AF_UNIX => return self.accept_unix(&mut sockhandle, sockfdobj, newfd, newfdoption, addr),
                        AF_INET | AF_INET6 => return self.accept_inet(&mut sockhandle, sockfdobj, newfd, newfdoption, addr),
                        _ => {return syscall_error(Errno::EINVAL, "accept", "Unsupported domain provided")}
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

    fn accept_unix(&self, sockhandle: &mut interface::RustLockReadGuard<SocketHandle>, sockfdobj: &mut SocketDesc, newfd: i32, newfdoption: &mut Option<FileDescriptor>, addr: &mut interface::GenSockaddr) -> i32 {
        match sockhandle.protocol {
            IPPROTO_UDP => {
                return syscall_error(Errno::EOPNOTSUPP, "accept", "Protocol does not support listening");
            }
            IPPROTO_TCP => {

                if sockhandle.state != ConnState::LISTEN {
                    return syscall_error(Errno::EINVAL, "accept", "Socket must be listening before accept is called");
                }
                let newsockfd = self._socket_initializer(sockhandle.domain, sockhandle.socktype, sockhandle.protocol, sockfdobj.flags & O_NONBLOCK != 0, sockfdobj.flags & O_CLOEXEC != 0, ConnState::CONNECTED);

                let remote_addr : interface::GenSockaddr;
                let sendpipenumber;
                let receivepipenumber;

                loop {
                    let localpathbuf = normpath(convpath(sockhandle.localaddr.unwrap().path().clone()), self);
                    let dsconnobj = NET_METADATA.domsock_accept_table.get(&localpathbuf);

                    if let Some(ds) = dsconnobj {
                        // we loop here to accept the connection
                        // if we get a connection object from the accept table, we complete the connection and set up the address and pipes
                        // if theres no object, we retry, except in the case of non-blocking accept where we return EAGAIN
                        if let Some(connvar) = ds.get_cond_var() {
                            if !connvar.broadcast() { 
                                drop(ds);
                                continue;
                            }
                        }
                        let addr = ds.get_sockaddr().clone();
                        remote_addr = addr.clone();
                        receivepipenumber = ds.get_receive_pipe().clone();
                        sendpipenumber = ds.get_send_pipe().clone();
                        drop(ds);
                        NET_METADATA.domsock_accept_table.remove(&localpathbuf);
                        break;
                    } else {
                        if 0 != (sockfdobj.flags & O_NONBLOCK) {
                            // if non block return EAGAIN
                            return syscall_error(Errno::EAGAIN, "accept", "host system accept call failed");
                        }
                    }
                }

                let newsock_tmp = newsockfd.handle.clone();
                let mut newsockhandle = newsock_tmp.write();

                let pathclone = normpath(convpath(remote_addr.path().clone()), self);
                if let Some(inodenum) = metawalk(pathclone.as_path()) {                   
                    newsockhandle.unix_info = Some(UnixSocketInfo {
                        path: pathclone,
                        inode: inodenum.clone(),
                        mode: sockhandle.unix_info.as_ref().unwrap().mode,
                        sendpipe: Some(sendpipenumber.clone()),
                        receivepipe: Some(receivepipenumber.clone()),
                    });
                    if let Inode::Socket(ref mut sock) = *(FS_METADATA.inodetable.get_mut(&inodenum).unwrap()) { 
                        sock.refcount += 1; 
                    } 
                }; 
                
                newsockhandle.localaddr = Some(sockhandle.localaddr.unwrap().clone());
                newsockhandle.remoteaddr = Some(remote_addr.clone());
                newsockhandle.state = ConnState::CONNECTED;
                let _insertval = newfdoption.insert(Socket(newsockfd));
                *addr = remote_addr; //populate addr with what address it connected to
                

                
                return newfd;
            }
            _ => {
                return syscall_error(Errno::EOPNOTSUPP, "accept", "Unkown protocol in accept");
            }
        }
    }

    fn accept_inet(&self, sockhandle: &mut interface::RustLockReadGuard<SocketHandle>, sockfdobj: &mut SocketDesc, newfd: i32, newfdoption: &mut Option<FileDescriptor>, addr: &mut interface::GenSockaddr) -> i32 {
        match sockhandle.protocol {
            IPPROTO_UDP => {
                return syscall_error(Errno::EOPNOTSUPP, "accept", "Protocol does not support listening");
            }
            IPPROTO_TCP => {
                if sockhandle.state != ConnState::LISTEN {
                    return syscall_error(Errno::EINVAL, "accept", "Socket must be listening before accept is called");
                }
                let newsockfd = self._socket_initializer(sockhandle.domain, sockhandle.socktype, sockhandle.protocol, sockfdobj.flags & O_NONBLOCK != 0, sockfdobj.flags & O_CLOEXEC != 0, ConnState::CONNECTED);

                let (acceptedresult, remote_addr) = if let Some(mut vec) = NET_METADATA.pending_conn_table.get_mut(&sockhandle.localaddr.unwrap().port()) {
                    //if we got a pending connection in select/poll/whatever, return that here instead
                    let tup = vec.pop().unwrap(); //pending connection tuple recieved
                    if vec.is_empty() {
                        drop(vec);
                        NET_METADATA.pending_conn_table.remove(&sockhandle.localaddr.unwrap().port()); //remove port from pending conn table if no more pending conns exist for it
                    }
                    tup
                } else {
                    //unwrap ok because listening
                    if 0 == (sockfdobj.flags & O_NONBLOCK) {
                        match sockhandle.domain {
                            PF_INET => sockhandle.innersocket.as_ref().unwrap().accept(true),
                            PF_INET6 => sockhandle.innersocket.as_ref().unwrap().accept(false),
                            _ => panic!("Unknown domain in accepting socket"),
                        }
                    } else {
                        match sockhandle.domain {
                            PF_INET => sockhandle.innersocket.as_ref().unwrap().nonblock_accept(true),
                            PF_INET6 => sockhandle.innersocket.as_ref().unwrap().nonblock_accept(false),
                            _ => panic!("Unknown domain in accepting socket"),
                        }
                    }
                };

                if let Err(_) = acceptedresult {
                    match Errno::from_discriminant(interface::get_errno()) {
                        Ok(e) => {
                            return syscall_error(e, "accept", "host system accept call failed");
                        },
                        Err(()) => panic!("Unknown errno value from socket send returned!"),
                    };
                }

                let acceptedsock = acceptedresult.unwrap();

                let mut newaddr = sockhandle.localaddr.unwrap().clone();
                let newport = match NET_METADATA._reserve_localport(newaddr.addr(), 0, sockhandle.protocol, sockhandle.domain, false) {
                    Ok(portnum) => portnum,
                    Err(errnum) => {
                        return errnum;
                    }
                };
                newaddr.set_port(newport);

                let newsock_tmp = newsockfd.handle.clone();
                let mut newsockhandle = newsock_tmp.write();

                newsockhandle.localaddr = Some(newaddr);
                newsockhandle.remoteaddr = Some(remote_addr.clone());

                //create socket object for new connected socket
                newsockhandle.innersocket = Some(acceptedsock);
                
                let _insertval = newfdoption.insert(Socket(newsockfd));
                *addr = remote_addr; //populate addr with what address it connected to

                return newfd;
            }
            _ => {
                return syscall_error(Errno::EOPNOTSUPP, "accept", "Unkown protocol in accept");
            }
        }
    }

    fn _nonblock_peek_read(&self, fd: i32) -> bool{
        let flags = MSG_PEEK;
        let mut buf = [0u8; 1];
        let bufptr = buf.as_mut_ptr();
        let checkedfd = self.get_filedescriptor(fd).unwrap();
        let mut unlocked_fd = checkedfd.write();
        if let Some(ref mut filedesc_enum) = &mut *unlocked_fd {
            let oldflags;
            if let Socket(ref mut sockfdobj) = filedesc_enum {
                oldflags = sockfdobj.flags;
                sockfdobj.flags |= O_NONBLOCK;
            } else {
                return false;
            }
            let retval = self.recv_common_inner(filedesc_enum, bufptr, 1, flags, &mut None);
            if let Socket(ref mut sockfdobj) = filedesc_enum {
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
    // NEED REFACTOR
    pub fn select_syscall(&self, nfds: i32, readfds: &mut interface::RustHashSet<i32>, writefds: &mut interface::RustHashSet<i32>, exceptfds: &mut interface::RustHashSet<i32>, timeout: Option<interface::RustDuration>) -> i32 {
       //exceptfds and writefds are not really implemented at the current moment.
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
           
            // 1. iterate thru readfds
            let mut res = self.select_readfds(readfds, &mut new_readfds, &mut retval);
            if res != 0 {return res}
            
            // 2. iterate thru writefds
            res = self.select_writefds(writefds, &mut new_writefds, &mut retval);
            if res != 0 {return res}
            
            // 3. iterate thru exceptfds
            for fd in exceptfds.iter() {
                //we say none of them ever have exceptional conditions
                let checkedfd = self.get_filedescriptor(*fd).unwrap();
                let unlocked_fd = checkedfd.read();
                if let None = *unlocked_fd { return syscall_error(Errno::EBADF, "select", "invalid file descriptor"); }
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

    fn select_readfds(&self, readfds: &mut interface::RustHashSet<i32>, new_readfds: &mut interface::RustHashSet<i32>, retval: &mut i32) -> i32 {
        for fd in readfds.iter() {
            let checkedfd = self.get_filedescriptor(*fd).unwrap();
            let mut unlocked_fd = checkedfd.write();
            if let Some(filedesc_enum) = &mut *unlocked_fd {
                match filedesc_enum {
                    Socket(ref mut sockfdobj) => {
                        let sock_tmp = sockfdobj.handle.clone();
                        let mut sockhandle = sock_tmp.write();

                        match sockhandle.domain {
                            AF_UNIX => {
                                if sockhandle.state == ConnState::INPROGRESS {
                                    let remotepathbuf = convpath(sockhandle.remoteaddr.unwrap().path().clone());
                                    let dsconnobj = NET_METADATA.domsock_accept_table.get(&remotepathbuf);
                                    if dsconnobj.is_none() { sockhandle.state = ConnState::CONNECTED; }
                                }

                                if sockhandle.state == ConnState::CONNECTED {
                                    drop(sockhandle);
                                    drop(unlocked_fd);
                                    if self._nonblock_peek_read(*fd) {
                                        new_readfds.insert(*fd);
                                        *retval += 1;
                                    }
                                }
                            }
                            AF_INET | AF_INET6 => {
                                if sockhandle.state == ConnState::LISTEN {
                                    if let interface::RustHashEntry::Vacant(vacant) = NET_METADATA.pending_conn_table.entry(sockhandle.localaddr.unwrap().port().clone()) {

                                        //innersock unwrap ok because sockhandle is listening
                                        let listeningsocket = match sockhandle.domain {
                                            PF_INET => sockhandle.innersocket.as_ref().unwrap().nonblock_accept(true),
                                            PF_INET6 => sockhandle.innersocket.as_ref().unwrap().nonblock_accept(false),
                                            _ => panic!("Unknown domain in accepting socket"),
                                        };
                                        drop(sockhandle);
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
                                    *retval += 1;
                                    //sockhandle innersocket unwrap ok if INPROGRESS
                                } else if sockhandle.state == ConnState::INPROGRESS && sockhandle.innersocket.as_ref().unwrap().check_rawconnection() {
                                        sockhandle.state = ConnState::CONNECTED;
                                        new_readfds.insert(*fd);
                                        *retval += 1;
                                } else {
                                    if sockhandle.protocol == IPPROTO_UDP {
                                        new_readfds.insert(*fd);
                                        *retval += 1;
                                    } else {
                                        drop(sockhandle);
                                        drop(unlocked_fd);
                                        if self._nonblock_peek_read(*fd) {
                                            new_readfds.insert(*fd);
                                            *retval += 1;
                                        }
                                    }
                                }
                            },
                            _ => {return syscall_error(Errno::EINVAL, "select", "Unsupported domain provided")}
                        }
                    }

                    //we don't support selecting streams
                    Stream(_) => {continue;}

                    //not supported yet
                    Pipe(pipefdobj) => {
                        if pipefdobj.pipe.check_select_read() {
                            new_readfds.insert(*fd);
                            *retval += 1;
                        }
                    }

                    //these file reads never block
                    _ => {
                        new_readfds.insert(*fd);
                        *retval += 1;
                    }
                }
            } else {
                return syscall_error(Errno::EBADF, "select", "invalid file descriptor");
            }
        }
        return 0;
    }

    fn select_writefds(&self, writefds: &mut interface::RustHashSet<i32>, new_writefds: &mut interface::RustHashSet<i32>, retval: &mut i32) -> i32 {
        for fd in writefds.iter() {
            let checkedfd = self.get_filedescriptor(*fd).unwrap();
            let mut unlocked_fd = checkedfd.write();
            if let Some(filedesc_enum) = &mut *unlocked_fd {
                match filedesc_enum {
                    Socket(ref mut sockfdobj) => {
                        // check if we've made an in progress connection first
                        let sock_tmp = sockfdobj.handle.clone();
                        let mut sockhandle = sock_tmp.write();
                        match sockhandle.domain {
                            AF_UNIX => {
                                if sockhandle.state == ConnState::INPROGRESS {
                                    let remotepathbuf = convpath(sockhandle.remoteaddr.unwrap().path().clone());
                                    let dsconnobj = NET_METADATA.domsock_accept_table.get(&remotepathbuf);
                                    if dsconnobj.is_none() { sockhandle.state = ConnState::CONNECTED; }
                                }
                            },
                            AF_INET => {
                                if sockhandle.state == ConnState::INPROGRESS && sockhandle.innersocket.as_ref().unwrap().check_rawconnection() {
                                    sockhandle.state = ConnState::CONNECTED;
                                } 
                            },
                            _ => {return syscall_error(Errno::EINVAL, "select", "Unsupported domain")}
                        }
                        
                        //we always say sockets are writable? Even though this is not true
                        new_writefds.insert(*fd);
                        *retval += 1;
                    }

                    //we always say streams are writable?
                    Stream(_) => {
                        new_writefds.insert(*fd);
                        *retval += 1;
                    }

                    //not supported yet
                    Pipe(pipefdobj) => {
                        if pipefdobj.pipe.check_select_write() {
                            new_writefds.insert(*fd);
                            *retval += 1;
                        }
                    }

                    //these file writes never block
                    _ => {
                        new_writefds.insert(*fd);
                        *retval += 1;
                    }
                }
            } else {
                return syscall_error(Errno::EBADF, "select", "invalid file descriptor");
            }
        }
        return 0;
    }

    pub fn getsockopt_syscall(&self, fd: i32, level: i32, optname: i32, optval: &mut i32) -> i32 {
        let checkedfd = self.get_filedescriptor(fd).unwrap();
        let mut unlocked_fd = checkedfd.write();
        if let Some(filedesc_enum) = &mut *unlocked_fd {
            if let Socket(ref mut sockfdobj) = filedesc_enum {
                //checking that we recieved SOL_SOCKET
                match level {
                    SOL_UDP => {
                        return syscall_error(Errno::EOPNOTSUPP, "getsockopt", "UDP is not supported for getsockopt");
                    }
                    SOL_TCP => {
                        return syscall_error(Errno::EOPNOTSUPP, "getsockopt", "TCP options not remembered by getsockopt");
                    }
                    SOL_SOCKET => {
                        let optbit = 1 << optname;

                        let sock_tmp = sockfdobj.handle.clone();
                        let mut sockhandle = sock_tmp.write();

                        match optname {
                            //indicate whether we are accepting connections or not in the moment
                            SO_ACCEPTCONN => {
                                if sockhandle.state == ConnState::LISTEN {
                                    *optval = 1;
                                } else {
                                    *optval = 0;
                                }
                            }
                            //if the option is a stored binary option, just return it...
                            SO_LINGER | SO_KEEPALIVE | SO_SNDLOWAT | SO_RCVLOWAT | SO_REUSEPORT | SO_REUSEADDR => {
                                if sockhandle.options & optbit == optbit {
                                    *optval = 1;
                                } else {
                                    *optval = 0;
                                }
                            }
                            //handling the ignored buffer settings:
                            SO_SNDBUF => {
                                *optval = sockhandle.sndbuf;
                            }
                            SO_RCVBUF => {
                                *optval = sockhandle.rcvbuf;
                            }
                            //returning the type if asked
                            SO_TYPE => {
                                *optval = sockhandle.socktype;
                            }
                            //should always be true
                            SO_OOBINLINE => {
                                *optval = 1;
                            }
                            SO_ERROR => {
                                let tmp = sockhandle.errno;
                                sockhandle.errno = 0;
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
        let checkedfd = self.get_filedescriptor(fd).unwrap();
        let mut unlocked_fd = checkedfd.write();
        if let Some(filedesc_enum) = &mut *unlocked_fd {
            if let Socket(ref mut sockfdobj) = filedesc_enum {
                //checking that we recieved SOL_SOCKET
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
                        let sock_tmp = sockfdobj.handle.clone();
                        let mut sockhandle = sock_tmp.write();

                        match optname {
                            SO_ACCEPTCONN | SO_TYPE | SO_SNDLOWAT | SO_RCVLOWAT => {
                                let error_string = format!("Cannot set option using setsockopt. {}", optname);
                                return syscall_error(Errno::ENOPROTOOPT, "setsockopt", &error_string);
                            }
                            SO_LINGER | SO_KEEPALIVE => {
                                if optval == 0 {
                                    sockhandle.options &= !optbit;
                                } else {
                                    //optval should always be 1 or 0.
                                    sockhandle.options |= optbit;
                                }


                                return 0;
                            }

                            SO_REUSEPORT | SO_REUSEADDR => {
                                let mut newoptions = sockhandle.options;
                                //now let's set this if we were told to
                                if optval != 0 {
                                    //optval should always be 1 or 0.
                                    newoptions |= optbit;
                                } else {
                                    newoptions &= !optbit;
                                }

                                if newoptions != sockhandle.options {
                                    if let Some(sock) = sockhandle.innersocket.as_ref() {
                                        let sockret = sock.setsockopt(SOL_SOCKET, optname, optval);
                                        if sockret < 0 {
                                            match Errno::from_discriminant(interface::get_errno()) {
                                                Ok(i) => {return syscall_error(i, "setsockopt", "The libc call to setsockopt failed!");},
                                                Err(()) => panic!("Unknown errno value from setsockopt returned!"),
                                            };
                                        }
                                    }
                                }

                                sockhandle.options = newoptions;

                                return 0;
                            }
                            SO_SNDBUF => {
                                sockhandle.sndbuf = optval;
                                return 0;
                            }
                            SO_RCVBUF => {
                                sockhandle.rcvbuf = optval;
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
        let checkedfd = self.get_filedescriptor(fd).unwrap();
        let unlocked_fd = checkedfd.read();
        if let Some(filedesc_enum) = &*unlocked_fd {
            if let Socket(sockfdobj) = filedesc_enum {
                //if the socket is not connected, then we should return an error
                let sock_tmp = sockfdobj.handle.clone();
                let sockhandle = sock_tmp.read();
                if sockhandle.remoteaddr == None {
                    return syscall_error(Errno::ENOTCONN, "getpeername", "the socket is not connected");
                }
                *ret_addr = sockhandle.remoteaddr.unwrap(); 
                return 0;

            } else {
                return syscall_error(Errno::ENOTSOCK, "getpeername", "the provided file is not a socket");
            }
        } else {
            return syscall_error(Errno::EBADF, "getpeername", "the provided file descriptor is not valid");
        }
    }

    pub fn getsockname_syscall(&self, fd: i32, ret_addr: &mut interface::GenSockaddr) -> i32 {
        let checkedfd = self.get_filedescriptor(fd).unwrap();
        let unlocked_fd = checkedfd.read();
        if let Some(filedesc_enum) = &*unlocked_fd {
            if let Socket(sockfdobj) = filedesc_enum {
                let sock_tmp = sockfdobj.handle.clone();
                let sockhandle = sock_tmp.read();
                if sockhandle.domain == AF_UNIX {
                    if sockhandle.localaddr == None {
                        let null_path: &[u8] = &[];
                        *ret_addr = interface::GenSockaddr::Unix(interface::new_sockaddr_unix(sockhandle.domain as u16, null_path));
                        return 0;
                    }
                    //if the socket is not none, then return the socket
                    *ret_addr = sockhandle.localaddr.unwrap();
                    return 0;
                }
                else {
                    if sockhandle.localaddr == None {
                        //sets the address to 0.0.0.0 if the address is not initialized yet
                        //setting the family as well based on the domain
                        let addr = match sockhandle.domain {
                            AF_INET => { interface::GenIpaddr::V4(interface::V4Addr::default()) }
                            AF_INET6 => { interface::GenIpaddr::V6(interface::V6Addr::default()) }
                            _ => { unreachable!() }
                        };
                        ret_addr.set_addr(addr);
                        ret_addr.set_port(0);
                        ret_addr.set_family(sockhandle.domain as u16);
                        return 0;
                    }
                    *ret_addr = sockhandle.localaddr.unwrap();
                    return 0;
                }
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
        let (fd, guardopt) = self.get_next_fd(None);
        if fd < 0 { return fd }
        let fdoption = &mut *guardopt.unwrap();
        let _insertval = fdoption.insert(epollobjfd);
        
        return fd;
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
        let checkedfd = self.get_filedescriptor(epfd).unwrap();
        let mut unlocked_fd = checkedfd.write();
        if let Some(filedesc_enum_epollfd) = &mut *unlocked_fd {
            if let Epoll(epollfdobj) = filedesc_enum_epollfd {

                //check if the other fd is an epoll or not...
                let checkedfd = self.get_filedescriptor(fd).unwrap();
                let unlocked_fd = checkedfd.read();
                if let Some(filedesc_enum) = &*unlocked_fd {
                    if let Epoll(_) = filedesc_enum {
                        return syscall_error(Errno::EBADF, "epoll ctl", "provided fd is not a valid file descriptor")
                    }
                } else { return syscall_error(Errno::EBADF, "epoll ctl", "provided fd is not a valid file descriptor"); }

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
        let checkedfd = self.get_filedescriptor(epfd).unwrap();
        let mut unlocked_fd = checkedfd.write();
        if let Some(filedesc_enum) = &mut *unlocked_fd {
            if let Epoll(epollfdobj) = filedesc_enum {
                if  maxevents <  0 {
                    return syscall_error(Errno::EINVAL, "epoll wait", "max events argument is not a positive number");
                }
                let mut poll_fds_vec: Vec<PollStruct> = vec![];
                let mut num_events: usize = 0;
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
                  num_events += 1;
                } 
                let poll_fds_slice = &mut poll_fds_vec[..];
                Self::poll_syscall(&self, poll_fds_slice, timeout);
                let mut count = 0;
                let end_idx: usize = interface::rust_min(num_events, maxevents as usize);
                for result in poll_fds_slice[..end_idx].iter() {
                    let mut poll_event = false;
                    let mut event = EpollEvent{ events: 0, fd: epollfdobj.registered_fds.get(&result.fd).unwrap().fd};
                    if result.revents & POLLIN > 0 {
                        event.events |= EPOLLIN as u32;
                        poll_event = true;
                    }
                    if result.revents & POLLOUT > 0 {
                        event.events |= EPOLLOUT as u32;
                        poll_event = true;
                    }
                    if result.revents & POLLERR > 0 {
                        event.events |= EPOLLERR as u32;
                        poll_event = true;
                    }

                    if poll_event {
                        events[count] = event;
                        count += 1;
                    }
                }
                return count as i32;
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
        if domain == AF_UNIX && socktype != SOCK_STREAM && protocol != IPPROTO_TCP {
            return syscall_error(Errno::EOPNOTSUPP, "socketpair", "Unix domain sockets are only supported for TCP")
        }

        let sock1fd = this.socket_syscall(domain, socktype, protocol);
        if sock1fd < 0 {return sock1fd;}

        // We set SO_REUSEADDR for socketpair because otherwise, after the sockets are closed, they
        // can get into a TIME_WAIT state which makes the sockets unusable for 60 seconds--however
        // because socketpair is local we don't need such a timeout for the peer to tell the socket
        // has closed, so we set SO_REUSEADDR to allow rebinding--this prevents the case where if
        // lind closes and is restarted within 60 seconds, it fails to bind a socket created via
        // socketpair.
        let setsockopt_res1 = this.setsockopt_syscall(sock1fd, SOL_SOCKET, SO_REUSEADDR, 1);
        if setsockopt_res1 != 0 { //this should really only happen with ENOMEM/ENOBUFS
            this.close_syscall(sock1fd);
            return setsockopt_res1;
        }
        let sock2fd = this.socket_syscall(domain, socktype, protocol);
        if sock2fd < 0 {
            this.close_syscall(sock1fd);
            return sock2fd;
        }
        let setsockopt_res2 = this.setsockopt_syscall(sock1fd, SOL_SOCKET, SO_REUSEADDR, 1);
        if setsockopt_res2 != 0 { //this should really only happen with ENOMEM/ENOBUFS
            this.close_syscall(sock2fd);
            this.close_syscall(sock1fd);
            return setsockopt_res2;
        }
    
        let portlessaddr = if domain == AF_INET {
            let ipaddr = interface::V4Addr {s_addr: u32::from_ne_bytes([127, 0, 0, 1])};
            let innersockaddr = interface::SockaddrV4{sin_family: domain as u16, sin_addr: ipaddr, sin_port: 0, padding: 0};
            interface::GenSockaddr::V4(innersockaddr)
        } else if domain == AF_INET6 {
            let ipaddr = interface::V6Addr {s6_addr: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1]};
            let innersockaddr = interface::SockaddrV6{sin6_family: domain as u16, sin6_addr: ipaddr, sin6_port: 0, sin6_flowinfo: 0, sin6_scope_id: 0};
            interface::GenSockaddr::V6(innersockaddr)
        } else if domain == AF_UNIX {
            let path = interface::gen_ud_path();
            interface::GenSockaddr::Unix(interface::new_sockaddr_unix(AF_UNIX as u16, path.as_bytes()))
        } else {
            unreachable!();
        };
    
        match socktype {
            SOCK_STREAM => {
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
            }
            SOCK_DGRAM => {
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
            }
            _ => {
                unreachable!();
            }
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
