//! This module contains all networking-related system calls.
//!
//! ## Notes:
//!
//! - These calls are implementations of the [`Cage`] struct in the
//!   [`safeposix`](crate::safeposix) crate. See the
//!   [`safeposix`](crate::safeposix) crate for more information.
//! They have been structed as different modules for better maintainability and
//! related functions. since they are tied to the `Cage` struct This module's
//! rustdoc may turn up empty, thus they have been explicitly listed below for
//! documentation purposes.
//!
//!
//! ## Networking System Calls
//!
//! This module contains all networking system calls that are being
//! emulated/faked in Lind.
//!
//! - [socket_syscall](crate::safeposix::cage::Cage::socket_syscall)
//! - [force_innersocket](crate::safeposix::cage::Cage::force_innersocket)
//! - [bind_syscall](crate::safeposix::cage::Cage::bind_syscall)
//! - [bind_inner](crate::safeposix::cage::Cage::bind_inner)
//! - [connect_syscall](crate::safeposix::cage::Cage::connect_syscall)
//! - [sendto_syscall](crate::safeposix::cage::Cage::sendto_syscall)
//! - [send_syscall](crate::safeposix::cage::Cage::send_syscall)
//! - [recv_common](crate::safeposix::cage::Cage::recv_common)
//! - [recvfrom_syscall](crate::safeposix::cage::Cage::recvfrom_syscall)
//! - [recv_syscall](crate::safeposix::cage::Cage::recv_syscall)
//! - [listen_syscall](crate::safeposix::cage::Cage::listen_syscall)
//! - [netshutdown_syscall](crate::safeposix::cage::Cage::netshutdown_syscall)
//! - [_cleanup_socket_inner_helper](crate::safeposix::cage::Cage::_cleanup_socket_inner_helper)
//! - [_cleanup_socket_inner](crate::safeposix::cage::Cage::_cleanup_socket_inner)
//! - [_cleanup_socket](crate::safeposix::cage::Cage::_cleanup_socket)
//! - [accept_syscall](crate::safeposix::cage::Cage::accept_syscall)
//! - [select_syscall](crate::safeposix::cage::Cage::select_syscall)
//! - [getsockopt_syscall](crate::safeposix::cage::Cage::getsockopt_syscall)
//! - [setsockopt_syscall](crate::safeposix::cage::Cage::setsockopt_syscall)
//! - [getpeername_syscall](crate::safeposix::cage::Cage::getpeername_syscall)
//! - [getsockname_syscall](crate::safeposix::cage::Cage::getsockname_syscall)
//! - [gethostname_syscall](crate::safeposix::cage::Cage::gethostname_syscall)
//! - [poll_syscall](crate::safeposix::cage::Cage::poll_syscall)
//! - [_epoll_object_allocator](crate::safeposix::cage::Cage::_epoll_object_allocator)
//! - [epoll_create_syscall](crate::safeposix::cage::Cage::epoll_create_syscall)
//! - [epoll_ctl_syscall](crate::safeposix::cage::Cage::epoll_ctl_syscall)
//! - [epoll_wait_syscall](crate::safeposix::cage::Cage::epoll_wait_syscall)
//! - [socketpair_syscall](crate::safeposix::cage::Cage::socketpair_syscall)
//! - [getifaddrs_syscall](crate::safeposix::cage::Cage::getifaddrs_syscall)

#![allow(dead_code)]
// Network related system calls
// outlines and implements all of the networking system calls that are being
// emulated/faked in Lind

use super::fs_constants::*;
use super::net_constants::*;
use super::sys_constants::*;
use crate::interface;
use crate::interface::errnos::{syscall_error, Errno};
use crate::safeposix::cage::{FileDescriptor::*, *};
use crate::safeposix::filesystem::*;
use crate::safeposix::net::*;

impl Cage {
    //Initializes a socket file descriptor and sets the necessary flags
    fn _socket_initializer(
        &self,
        domain: i32,
        socktype: i32,
        protocol: i32,
        nonblocking: bool,
        cloexec: bool,
        conn: ConnState,
    ) -> SocketDesc {
        //For blocking sockets, operations wait until completed.
        //For non-blocking sockets, operations return immediately, even if the
        // requested operation is not completed. To further understand blocking, refer to https://www.scottklement.com/rpg/socktut/nonblocking.html
        //
        //O_CLOEXEC flag closes the fd pointing to the socket on the execution of a new
        // program This flag is neccessary upon setting the fd to avoid race
        // condition described in https://man7.org/linux/man-pages/man2/open.2.html under the O_CLOEXEC section
        let flags = if nonblocking { O_NONBLOCK } else { 0 } | if cloexec { O_CLOEXEC } else { 0 };

        let sockfd = SocketDesc {
            flags: flags,
            domain: domain,
            rawfd: -1, // RawFD set in bind for inet, or stays at -1 for others
            handle: interface::RustRfc::new(interface::RustLock::new(Self::mksockhandle(
                domain, socktype, protocol, conn, flags,
            ))),
            advlock: interface::RustRfc::new(interface::AdvisoryLock::new()),
        }; //currently on failure to create handle we create successfully but it's
           // corrupted, change?

        return sockfd;
    }

    //Find an available file descriptor index in the File Descriptor Table
    //If available, insert the socket fd into the File Descriptor Table at the
    //available index
    fn _socket_inserter(&self, sockfd: FileDescriptor) -> i32 {
        let (fd, guardopt) = self.get_next_fd(None);
        //In the case that no fd is available from the call to get_next_fd,
        //fd is set to -ENFILE = -23 and the error is propagated forward
        if fd < 0 {
            return fd;
        }
        let fdoption = &mut *guardopt.unwrap();
        //Insert the sockfd into the FileDescriptor option inside the fd table
        let _insertval = fdoption.insert(sockfd);
        return fd;
    }

    //An implicit bind refers to the automatic binding of a socket to an address
    //and port by the system, without an explicit call to the bind() function by
    // the programmer. This typically happens when the socket is used for
    // client-side operations, such as when it initiates a connection to a
    // server.
    fn _implicit_bind(&self, sockhandle: &mut SocketHandle, domain: i32) -> i32 {
        if sockhandle.localaddr.is_none() {
            //Assign a new local address to the socket handle
            let localaddr = match Self::assign_new_addr(
                sockhandle,
                domain,
                //The SO_RESUEPORT bit placement within the protocol int encodes rebind ability
                //The rebind ability of a socket refers to whether a socket can be
                //re-bound to an address and port that it was previously bound to,
                //especially after it has been closed or if the binding has been reset.
                //To learn more about the importance of SO_REUSEPORT, check out https://lwn.net/Articles/542629/
                sockhandle.protocol & (1 << SO_REUSEPORT) != 0,
            ) {
                Ok(a) => a,
                Err(e) => return e,
            };

            //Bind the address to the socket handle
            let bindret = self.bind_inner_socket(sockhandle, &localaddr, true);

            //If an error occurs during binding,
            if bindret < 0 {
                match Errno::from_discriminant(interface::get_errno()) {
                    Ok(i) => {
                        return syscall_error(
                            i,
                            "recvfrom",
                            "syscall error from attempting to bind within recvfrom",
                        );
                    }
                    Err(()) => panic!("Unknown errno value from socket recvfrom returned!"),
                };
            }
        }
        0
    }

    /// ## `socket_syscall`
    ///
    /// ### Description
    /// This function creates a new socket, ensuring the requested domain,
    /// socket type, and protocol are supported by SafePosix.
    /// It validates the requested communication domain, socket type, and
    /// protocol, permitting only combinations that are known to be safe and
    /// secure.
    ///
    /// ### Function Arguments
    /// * `domain`: The communication domain for the socket. Supported values
    ///   are `PF_INET` (Internet Protocol) and `PF_UNIX` (Unix domain sockets).
    /// * `socktype`: The socket type. Supported values are `SOCK_STREAM`
    ///   (stream sockets) and `SOCK_DGRAM` (datagram sockets).
    /// * `protocol`: The protocol to use for communication. This defaults to
    ///   TCP for stream sockets (`SOCK_STREAM`) and UDP for datagram sockets
    ///   (`SOCK_DGRAM`).
    ///
    /// ### Returns
    /// * The new file descriptor representing the socket on success.
    ///
    /// ### Errors
    /// * `EOPNOTSUPP(95)`: If an unsupported combination of domain, socket
    ///   type, or protocol is requested.
    /// * `EINVAL(22)`: If an invalid combination of flags is provided.
    /// ### Panics
    /// There are no panics in this syscall.
    pub fn socket_syscall(&self, domain: i32, socktype: i32, protocol: i32) -> i32 {
        let real_socktype = socktype & 0x7; //get the type without the extra flags, it's stored in the last 3 bits
        let nonblocking = (socktype & SOCK_NONBLOCK) != 0; // Checks if the socket should be non-blocking.
                                                           //Check blocking status for storage in the file descriptor, we'll need this for
                                                           // calls that don't access the kernel
                                                           // socket, unix sockets, and properly directing kernel calls for recv and accept
        let cloexec = (socktype & SOCK_CLOEXEC) != 0;
        // Checks if the 'close-on-exec' flag is set. This flag ensures the socket is
        // automatically closed if the current process executes another program,
        // preventing unintended inheritance of the socket by the new program.

        // additional flags are not supported
        // filtering out any socktypes with unexpected flags set.
        // This is important as we dont want to pass down any flags that are not
        // supported by SafePOSIX. which may potentially cause issues with the
        // underlying libc call. or the socket creation process. leading to
        // unexpected behavior.
        if socktype & !(SOCK_NONBLOCK | SOCK_CLOEXEC | 0x7) != 0 {
            return syscall_error(Errno::EOPNOTSUPP, "socket", "Invalid combination of flags");
        }
        //SafePOSIX intentionally supports only a restricted subset of socket types .
        // This is to make sure that applications not creating other socket
        // types which may lead to security issues. By using the match
        // statement, SafePOSIX ensures that only these approved socket types are
        // allowed.
        match real_socktype {
            // Handles different socket types SOCK_STREAM or SOCK_DGRAM in this cases
            SOCK_STREAM => {
                //SOCK_STREAM defaults to TCP for protocol, otherwise protocol is unsupported
                let newprotocol = if protocol == 0 { IPPROTO_TCP } else { protocol };

                if newprotocol != IPPROTO_TCP {
                    return syscall_error(
                        Errno::EOPNOTSUPP,
                        "socket",
                        "The only SOCK_STREAM implemented is TCP. Unknown protocol input.",
                    );
                }
                match domain {
                    // Handles different communication domains in this case PF_INET/PF_UNIX
                    PF_INET | PF_INET6 | PF_UNIX => {
                        // Internet Protocol (PF_INET) and Unix Domain Sockets (PF_UNIX)
                        //PR_INET / AF_INET and PF_UNIX / AF_UNIX are the same
                        //https://man7.org/linux/man-pages/man2/socket.2.html
                        let sockfdobj = self._socket_initializer(
                            domain,
                            socktype,
                            newprotocol,
                            nonblocking,
                            cloexec,
                            ConnState::NOTCONNECTED,
                        );
                        // Creates a SafePOSIX socket descriptor using '_socket_initializer', a
                        // helper function that encapsulates the internal
                        // details of socket creation and initialization.
                        return self._socket_inserter(Socket(sockfdobj));
                        // Inserts the newly created socket descriptor into the
                        // cage's file descriptor table,
                        // making it accessible to the application.Returns the
                        // file descriptor representing the socket.
                    }
                    _ => {
                        return syscall_error(
                            Errno::EOPNOTSUPP,
                            "socket",
                            "trying to use an unimplemented domain",
                        ); // Returns an error if an unsupported domain is
                           // requested.
                    }
                }
            }

            SOCK_DGRAM => {
                //SOCK_DGRAM defaults to UDP for protocol, otherwise protocol is unsuported
                let newprotocol = if protocol == 0 { IPPROTO_UDP } else { protocol };

                if newprotocol != IPPROTO_UDP {
                    return syscall_error(
                        Errno::EOPNOTSUPP,
                        "socket",
                        "The only SOCK_DGRAM implemented is UDP. Unknown protocol input.",
                    );
                }
                // SafePOSIX intentionally supports only a restricted subset of socket types .
                // This is to make sure that applications not creating other
                // socket types which may lead to security issues. By using the
                // match statement,  SafePOSIX ensures that only these approved socket types are
                // allowed.
                match domain {
                    // Handles different communication domains in this case PF_INET/PF_UNIX
                    PF_INET | PF_INET6 | PF_UNIX => {
                        // Internet Protocol (PF_INET) and Unix Domain Sockets (PF_UNIX)
                        //PR_INET / AF_INET and PF_UNIX / AF_UNIX are the same
                        //https://man7.org/linux/man-pages/man2/socket.2.html
                        let sockfdobj = self._socket_initializer(
                            domain,
                            socktype,
                            newprotocol,
                            nonblocking,
                            cloexec,
                            ConnState::NOTCONNECTED,
                        );
                        // Creates a SafePOSIX socket descriptor using '_socket_initializer', a
                        // helper function that encapsulates the internal
                        // details of socket creation and initialization.
                        return self._socket_inserter(Socket(sockfdobj));
                        // Inserts the newly created socket descriptor into the
                        // cage's file descriptor table,making it accessible to
                        // the application. Returns the
                        // file descriptor (an integer) representing the socket.
                    }
                    _ => {
                        return syscall_error(
                            Errno::EOPNOTSUPP,
                            "socket",
                            "trying to use an unimplemented domain",
                        );
                    }
                }
            }

            _ => {
                return syscall_error(
                    Errno::EOPNOTSUPP,
                    "socket",
                    "trying to use an unimplemented socket type",
                ); // Returns an error if an unsupported domain is requested.
            }
        }
    }

    //creates a sockhandle if none exists, otherwise this is a no-op
    pub fn force_innersocket(sockhandle: &mut SocketHandle) {
        //The innersocket is an Option wrapped around the raw sys fd
        //Depending on domain, innersocket may or may not be necessary
        //Ex: Unix sockets do not have a kernel fd
        //If innersocket is not available, then create a socket and insert its
        //fd into innersocket
        if let None = sockhandle.innersocket {
            //Create a new socket, as no sockhandle exists
            //Socket creation rarely fails except for invalid parameters or extremely
            // low-resources conditions Upon failure, process will panic
            //Lind handles IPv4, IPv6, and Unix for domains
            let thissock =
                interface::Socket::new(sockhandle.domain, sockhandle.socktype, sockhandle.protocol);

            //Loop through socket options and check which ones are set
            //This is necessary as we can only set one option at a time
            for reuse in [SO_REUSEPORT, SO_REUSEADDR] {
                //If socket option is not set, continue to next socket option
                if sockhandle.socket_options & (1 << reuse) == 0 {
                    continue;
                }

                //Otherwise, set the socket option in the new socket
                //The level argument specifies the protocol level at which the option resides
                //In our case, we are setting options at the socket level
                //To learn more about setsockopt https://man7.org/linux/man-pages/man3/setsockopt.3p.html
                let sockret = thissock.setsockopt(SOL_SOCKET, reuse, 1);
                //Failure occured upon setting a socket option
                //Possible failures can be read at the man page linked above
                //
                //TODO: Possibly add errors instead of having a single panic for all errors
                if sockret < 0 {
                    panic!("Cannot handle failure in setsockopt on socket creation");
                }
            }

            //Insert the socket file descriptor into the innersocket value
            sockhandle.innersocket = Some(thissock);
        };
    }

    //TODO: bind_syscall can be merged with bind_inner to be one function as this
    // is a remnant from a previous refactor

    /// ### Description
    ///
    /// `bind_syscall` - when a socket is created with socket_syscall, it exists
    /// in a name  space (address family) but has no address assigned to it.
    /// bind_syscall  assigns the address specified by localaddr to the
    /// socket referred to  by the file descriptor fd.
    ///
    /// ### Arguments
    ///
    /// it accepts two parameters:
    /// * `fd` - an open file descriptor
    /// * `localaddr` - the address to bind to the socket referred to by fd
    ///
    /// ### Returns
    ///
    /// On success, zero is returned.  On error, -errno is returned, and
    /// errno is set to indicate the error.
    ///
    /// ### Errors
    ///
    /// * EACCES - The address is protected, and the user is not the superuser.
    /// * EADDRINUSE - The given address is already in use.
    /// * EADDRINUSE - (Internet domain sockets) The port number was specified
    ///   as zero in the socket address structure, but, upon attempting to bind
    ///   to an ephemeral port, it was determined that all port numbers in the
    ///   ephemeral port range are currently in use.  See the discussion of
    ///   /proc/sys/net/ipv4/ip_local_port_range ip(7).
    /// * EBADF - sockfd is not a valid file descriptor.
    /// * EINVAL - The socket is already bound to an address.
    /// * EINVAL - addrlen is wrong, or addr is not a valid address for this
    ///   socket's domain.
    /// * ENOTSOCK - The file descriptor sockfd does not refer to a socket.
    ///
    /// The following errors are specific to UNIX domain (AF_UNIX)
    /// sockets:
    ///
    /// * EACCES - Search permission is denied on a component of the path
    ///   prefix.  (See also path_resolution(7).)
    /// * EADDRNOTAVAIL - A nonexistent interface was requested or the requested
    ///   address was not local.
    /// * EFAULT - addr points outside the user's accessible address space.
    /// * ELOOP - Too many symbolic links were encountered in resolving addr.
    /// * ENAMETOOLONG - addr is too long.
    /// * ENOENT - A component in the directory prefix of the socket pathname
    ///   does not exist.
    /// * ENOMEM - Insufficient kernel memory was available.
    /// * ENOTDIR - A component of the path prefix is not a directory.
    /// * EROFS - The socket inode would reside on a read-only filesystem.
    pub fn bind_syscall(&self, fd: i32, localaddr: &interface::GenSockaddr) -> i32 {
        self.bind_inner(fd, localaddr, false)
    }

    //Direct to appropriate helper function based on the domain of the socket
    //to bind socket with local address
    //For INET sockets, create and bind the inner socket which is a kernel socket
    // noted by raw_sys_fd For Unix sockets, setup unix info field and bind to
    // local address On success, zero is returned.  On error, -errno is
    // returned, and errno is set to indicate the error.
    fn bind_inner_socket(
        &self,
        sockhandle: &mut SocketHandle,
        localaddr: &interface::GenSockaddr,
        prereserved: bool,
    ) -> i32 {
        //The family of the local address must match the domain of the socket handle
        if localaddr.get_family() != sockhandle.domain as u16 {
            return syscall_error(
                Errno::EINVAL,
                "bind",
                "An address with an invalid family for the given domain was specified",
            );
        }

        //If the socket is already bound to an address, exit with error
        if sockhandle.localaddr.is_some() {
            return syscall_error(
                Errno::EINVAL,
                "bind",
                "The socket is already bound to an address",
            );
        }

        let mut newsockaddr = localaddr.clone();

        //Bind socket based on domain type
        //The rules used in name binding vary between address families.
        //To learn more, read under the description section at https://man7.org/linux/man-pages/man2/bind.2.html
        let res = match sockhandle.domain {
            AF_UNIX => self.bind_inner_socket_unix(sockhandle, &mut newsockaddr),
            AF_INET | AF_INET6 => {
                self.bind_inner_socket_inet(sockhandle, &mut newsockaddr, prereserved)
            }
            _ => {
                return syscall_error(Errno::EINVAL, "bind", "Unsupported domain provided");
            }
        };

        sockhandle.localaddr = Some(newsockaddr);

        res
    }

    //bind_syscall implementation in the case that the socket's domain is unix
    //More details at https://man7.org/linux/man-pages/man7/unix.7.html
    fn bind_inner_socket_unix(
        &self,
        sockhandle: &mut SocketHandle,
        newsockaddr: &mut interface::GenSockaddr,
    ) -> i32 {
        // Unix Sockets
        let path = newsockaddr.path();
        //Check that path is not empty
        if path.len() == 0 {
            return syscall_error(Errno::ENOENT, "bind", "given path was null");
        }
        //true path is normalized path of the path to a unix socket
        let truepath = normpath(convpath(path), self);

        //returns tuple consisting of inode number of file (if it exists), and
        //inode number of parent (if it exists)
        match metawalkandparent(truepath.as_path()) {
            //If neither the file nor parent exists
            (None, None) => {
                return syscall_error(Errno::ENOENT, "bind", "a directory component in pathname does not exist or is a dangling symbolic link");
            }
            //If the file doesn't exist but the parent does
            (None, Some(pardirinode)) => {
                let filename = truepath.file_name().unwrap().to_str().unwrap().to_string(); //for now we assume this is sane, but maybe this should be checked later

                //this may end up skipping an inode number in the case of ENOTDIR, but that's
                // not catastrophic FS_METADATA contains information about the
                // file system
                let newinodenum = FS_METADATA
                    .nextinode
                    .fetch_add(1, interface::RustAtomicOrdering::Relaxed);
                //fetch_add returns the previous value, which is the inode number we want,
                // while incrementing the nextinode value as well The ordering
                // argument Relaxed guarantees the memory location is atomic, which is all that
                // is neccessary for a counter Read more at https://stackoverflow.com/questions/30407121/which-stdsyncatomicordering-to-use

                let newinode;

                //Pattern match the Directory Inode of the parent
                if let Inode::Dir(ref mut dir) =
                    *(FS_METADATA.inodetable.get_mut(&pardirinode).unwrap())
                {
                    //Add file type flags and user read,write,execute permission flags
                    let mode = (dir.mode | S_FILETYPEFLAGS as u32) & S_IRWXA;
                    //Add file type constant of a socket
                    let effective_mode = S_IFSOCK as u32 | mode;

                    let time = interface::timestamp(); //We do a real timestamp now
                                                       //Create a new inode for the file of the socket
                    newinode = Inode::Socket(SocketInode {
                        size: 0,
                        uid: DEFAULT_UID,
                        gid: DEFAULT_GID,
                        mode: effective_mode,
                        linkcount: 1,
                        refcount: 1,
                        atime: time,
                        ctime: time,
                        mtime: time,
                    });
                    //Find the DashMap that contains the parent directory
                    //Insert the file name and inode num as a key-value pair
                    //This will be used to find the inode num based on the file name
                    //in the file system
                    dir.filename_to_inode_dict
                        .insert(filename.clone(), newinodenum);
                    dir.linkcount += 1;
                } else {
                    //Parent dictory inode does not exist in inode table of file system
                    return syscall_error(
                        Errno::ENOTDIR,
                        "bind",
                        "unix domain socket path made socket address child of non-directory file",
                    );
                }
                //Insert unix info into socket handle
                //S_IFSOCK is the file type constant of a socket
                //0o666 allows read and write file operations within the directory
                //sendpipe and receivepipe are left as None because at the time of binding,
                //no data transfer occurs
                //inode is the newinodenum found above
                sockhandle.unix_info = Some(UnixSocketInfo {
                    mode: S_IFSOCK | 0o666,
                    sendpipe: None,
                    receivepipe: None,
                    inode: newinodenum,
                });

                //Insert path to socket file into a set
                NET_METADATA.domsock_paths.insert(truepath);
                //Insert the file inode num and inode as key-value pair into
                //file system inode table
                FS_METADATA.inodetable.insert(newinodenum, newinode);
            }
            //File already exists, meaning the given address argument to the bind_syscall
            //is not available for the socket
            (Some(_inodenum), ..) => {
                return syscall_error(Errno::EADDRINUSE, "bind", "Address already in use");
            }
        }

        0
    }

    //bind_syscall implementation in the case that the socket's domain is INET
    //More details at https://man7.org/linux/man-pages/man7/ip.7.html
    fn bind_inner_socket_inet(
        &self,
        sockhandle: &mut SocketHandle,
        newsockaddr: &mut interface::GenSockaddr,
        prereserved: bool,
    ) -> i32 {
        //INET Sockets
        //rebind ability is set to true if the SO_REUSEPORT bit is set in
        //the socket handle options
        let intent_to_rebind = sockhandle.socket_options & (1 << SO_REUSEPORT) != 0;
        //Create a socket and insert it into the innersocket in sockhandle
        Self::force_innersocket(sockhandle);

        //If socket address is preserved, set the local port to the reserved port
        //Otherwise, set the local port to a new value
        let newlocalport = if prereserved {
            newsockaddr.port()
        } else {
            //Reserve a new local port num
            let localout = NET_METADATA._reserve_localport(
                newsockaddr.addr(),
                newsockaddr.port(),
                sockhandle.protocol,
                sockhandle.domain,
                intent_to_rebind,
            );

            match localout {
                Err(errnum) => return errnum,
                Ok(local_port) => local_port,
            }
        };

        //Set the port of the socket address
        newsockaddr.set_port(newlocalport);
        //Bind the address to the socket handle
        let bindret = sockhandle.innersocket.as_ref().unwrap().bind(&newsockaddr);

        //If an error occurs during binding,
        if bindret < 0 {
            match Errno::from_discriminant(interface::get_errno()) {
                Ok(i) => {
                    return syscall_error(i, "bind", "The libc call to bind failed!");
                }
                Err(()) => panic!("Unknown errno value from socket bind returned!"),
            };
        }

        0
    }

    //Helper function of bind_syscall
    //Checks if fd refers to a valid socket file descriptor
    //fd: the file descriptor associated with the socket
    //localaddr: reference to the GenSockaddr enum that hold the address
    //prereserved: bool that describes whether the address and port have
    //             been set aside or designated for specific purposes
    pub fn bind_inner(
        &self,
        fd: i32,
        localaddr: &interface::GenSockaddr,
        prereserved: bool,
    ) -> i32 {
        //checkedfd is an atomic reference count of the number of locks on the fd
        let checkedfd = self.get_filedescriptor(fd).unwrap();
        //returns a write lock once no other writers or readers have access to the lock
        let mut unlocked_fd = checkedfd.write();
        if let Some(filedesc_enum) = &mut *unlocked_fd {
            match filedesc_enum {
                //*unlocked_fd is in the format of Option<T>, where T is of type Socket(&mut SocketDesc)
                Socket(ref mut sockfdobj) => {
                    //Clone the socket handle
                    let sock_tmp = sockfdobj.handle.clone();
                    //Obtain write gaurd for socket handle
                    let mut sockhandle = sock_tmp.write();
                    //We would like to pass the socket handle data to the function
                    //without giving it ownership sockfdobj, which may be in use
                    //by other threads accessing other fields
                    self.bind_inner_socket(&mut *sockhandle, localaddr, prereserved)
                }
                //error if file descriptor doesn't refer to a socket
                _ => syscall_error(
                    Errno::ENOTSOCK,
                    "bind",
                    "file descriptor refers to something other than a socket",
                ),
            }
        //error if fd is invalid
        } else {
            syscall_error(Errno::EBADF, "bind", "invalid file descriptor")
        }
    }

    //Assign address in unix domain
    //This helped function is used when we know we are working with unix sockets
    //so we can return the address without possibilites of errors
    //that come from INET sockets
    fn assign_new_addr_unix(sockhandle: &SocketHandle) -> interface::GenSockaddr {
        //If the socket handle has a local address set, return a clone of the addr.
        //This is because we do not want to assign a new address to a socket that is
        // already assigned one
        if let Some(addr) = sockhandle.localaddr.clone() {
            addr
        } else {
            //path will be in the format of /sockID, where ID is of type
            //usize before being converted to a string type
            //The UD_ID_COUNTER begins counting at 0
            let path = interface::gen_ud_path();
            //Unix domains paths can't exceed 108 bytes. If this happens, process will
            // panic Set the newremote address based on the Unix domain and the
            // path Note, Unix domain socket addresses expect a null-terminated
            // string or zero-padded path
            let newremote = interface::GenSockaddr::Unix(interface::new_sockaddr_unix(
                AF_UNIX as u16,
                path.as_bytes(),
            ));
            newremote
        }
    }

    //Return a new address based on the domain of the socket handle
    //If a socket handle contains a local address, return a clone of the local
    // address
    fn assign_new_addr(
        sockhandle: &SocketHandle,
        domain: i32,
        rebindability: bool,
    ) -> Result<interface::GenSockaddr, i32> {
        //The input domain must match the domain of the socket handle
        if domain != sockhandle.domain as i32 {
            return Err(syscall_error(
                Errno::EINVAL,
                "assign_new_addr",
                "An address with an invalid family for the given domain was specified",
            ));
        }
        //If the socket handle has a local address set, return a Result
        //type containing a clone of the addr. This is because we do not
        //want to assign a new address to a socket that already contains one
        if let Some(addr) = &sockhandle.localaddr {
            Ok(addr.clone())
        } else {
            let mut newremote: interface::GenSockaddr;
            //This is the specified behavior for the berkeley sockets API
            //Learn more about BSD at https://web.mit.edu/macdev/Development/MITSupportLib/SocketsLib/Documentation/sockets.html
            match domain {
                AF_UNIX => {
                    //path will be in the format of /sockID, where ID is of type
                    //usize before being converted toa string type
                    //The UD_ID_COUNTER begins counting at 0
                    let path = interface::gen_ud_path();
                    //Unix domains paths can't exceed 108 bytes. If this happens, process will
                    // panic Set the newremote address based on the Unix domain
                    // and the path Note, Unix domain socket addresses expect a
                    // null-terminated string or zero-padded path
                    newremote = interface::GenSockaddr::Unix(interface::new_sockaddr_unix(
                        AF_UNIX as u16,
                        path.as_bytes(),
                    ));
                }
                AF_INET => {
                    //Initialize and assign values to the remote address for a connection
                    //if a process binds to 0.0.0.0, all incoming connection to
                    //this machine are forwarded to this process
                    newremote = interface::GenSockaddr::V4(interface::SockaddrV4::default());
                    let addr = interface::GenIpaddr::V4(interface::V4Addr::default());
                    newremote.set_addr(addr);
                    newremote.set_family(AF_INET as u16);
                    //Arguments being passed in ...
                    //port is set to 0 to use any available port
                    //Possible protocols are IPPROTO_UDP and IPPROTO_TCP
                    //Will panic for unknown protocols
                    newremote.set_port(
                        match NET_METADATA._reserve_localport(
                            addr.clone(),
                            0,
                            sockhandle.protocol,
                            sockhandle.domain,
                            rebindability,
                        ) {
                            Ok(portnum) => portnum,
                            Err(errnum) => return Err(errnum),
                        },
                    );
                }
                AF_INET6 => {
                    //Initialize and assign values to the remote address for a connection
                    //if a process binds to [0; 16] all incoming connection to
                    //this machine are forwarded to this process
                    newremote = interface::GenSockaddr::V6(interface::SockaddrV6::default());
                    let addr = interface::GenIpaddr::V6(interface::V6Addr::default());
                    newremote.set_addr(addr);
                    newremote.set_family(AF_INET6 as u16);
                    //Arguments being passed in ...
                    //port is set to 0 to use any available port
                    //Possible protocols are IPPROTO_UDP and IPPROTO_TCP
                    //Will panic for unknown protocols
                    newremote.set_port(
                        match NET_METADATA._reserve_localport(
                            addr.clone(),
                            0,
                            sockhandle.protocol,
                            sockhandle.domain,
                            rebindability,
                        ) {
                            Ok(portnum) => portnum,
                            Err(errnum) => return Err(errnum),
                        },
                    );
                }
                _ => {
                    return Err(syscall_error(
                        Errno::EOPNOTSUPP,
                        "assign",
                        "Unkown protocol when assigning",
                    ));
                }
            };
            Ok(newremote)
        }
    }

    /// ### Description
    ///
    /// `connect_syscall` connects the socket referred to by the
    /// file descriptor fd to the address specified by remoteaddr.
    ///
    /// ### Arguments
    ///
    /// it accepts two parameters:
    /// * `fd` - an open file descriptor
    /// * `remoteaddr` - the address to request a connection to
    ///
    /// ### Returns
    ///
    /// for a successful call, zero is returned. On
    /// error, -errno is returned, and errno is set to indicate the error.
    ///
    /// ### Errors
    ///
    /// * EADDRNOTAVAIL - The specified address is not available from the local
    ///   machine.
    /// * EAFNOSUPPORT - The specified address is not a valid address for the
    ///   address family of the specified socket.
    /// * EALREADY - A connection request is already in progress for the
    ///   specified socket.
    /// ** EBADF - The socket argument is not a valid file descriptor.
    /// * ECONNREFUSED - The target address was not listening for connections or
    ///   refused the connection request.
    /// ** EINPROGRESS - O_NONBLOCK is set for the file descriptor for the
    ///    socket and the connection cannot be immediately established; the
    ///    connection shall be established asynchronously.
    /// * EINTR - The attempt to establish a connection was interrupted by
    ///   delivery of a signal that was caught; the connection shall be
    ///   established asynchronously.
    /// ** EISCONN - The specified socket is connection-mode and is already
    ///    connected.
    /// * ENETUNREACH - No route to the network is present.
    /// ** ENOTSOCK - The socket argument does not refer to a socket.
    /// * EPROTOTYPE - The specified address has a different type than the
    ///   socket bound to the specified peer address.
    /// * ETIMEDOUT - The attempt to connect timed out before a connection was
    ///   made.
    ///
    /// If the address family of the socket is AF_UNIX, then connect() shall
    /// fail if:
    ///
    /// * EIO - An I/O error occurred while reading from or writing to the file
    ///   system.
    /// * ELOOP - A loop exists in symbolic links encountered during resolution
    ///   of the pathname in address.
    /// * ENAMETOOLONG - A component of a pathname exceeded {NAME_MAX}
    ///   characters, or an entire pathname exceeded {PATH_MAX} characters.
    /// ** ENOENT - A component of the pathname does not name an existing file
    ///    or the pathname is an empty string.
    /// * ENOTDIR - A component of the path prefix of the pathname in address is
    ///   not a directory.
    ///
    /// The connect() function may fail if:
    ///
    /// * EACCES - Search permission is denied for a component of the path
    ///   prefix; or write access to the named socket is denied.
    /// * EADDRINUSE - Attempt to establish a connection that uses addresses
    ///   that are already in use.
    /// * ECONNRESET - Remote host reset the connection request.
    /// * EHOSTUNREACH - The destination host cannot be reached (probably
    ///   because the host is down or a remote router cannot reach it).
    /// ** EINVAL - The address_len argument is not a valid length for the
    ///    address family; or invalid address family in the sockaddr structure.
    /// * ELOOP - More than {SYMLOOP_MAX} symbolic links were encountered during
    ///   resolution of the pathname in address.
    /// * ENAMETOOLONG - Pathname resolution of a symbolic link produced an
    ///   intermediate result whose length exceeds {PATH_MAX}.
    /// * ENETDOWN - The local network interface used to reach the destination
    ///   is down.
    /// * ENOBUFS- No buffer space is available.
    /// ** EOPNOTSUPP - The socket is listening and cannot be connected.
    ///
    /// ** Indicates the error may be returned from RustPOSIX
    ///
    /// ### Panics
    ///
    /// * Unknown errno value from bind libc call, will cause panic
    /// * Unknown errno value from connect libc call, will cause panic.
    ///
    /// for more detailed description of all the commands and return values, see
    /// [connect(3)](https://linux.die.net/man/3/connect)
    pub fn connect_syscall(&self, fd: i32, remoteaddr: &interface::GenSockaddr) -> i32 {
        //If fd is out of range of [0,MAXFD], process will panic
        //Otherwise, we obtain a write gaurd to the Option<FileDescriptor> object
        let checkedfd = self.get_filedescriptor(fd).unwrap();
        let mut unlocked_fd = checkedfd.write();
        //Pattern match such that FileDescriptor object must be the Socket variant
        //Otherwise, return with an err as the fd refers to something other than a
        // socket
        if let Some(filedesc_enum) = &mut *unlocked_fd {
            match filedesc_enum {
                Socket(ref mut sockfdobj) => {
                    //We would like to pass the socket handle data to the function
                    //without giving it ownership sockfdobj, which may be in use
                    //by other threads accessing other fields
                    let sock_tmp = sockfdobj.handle.clone();
                    let mut sockhandle = sock_tmp.write();
                    //Possible address families are Unix, V4, V6
                    //Error occurs if remoteaddr's address family does not match
                    //the domain of the socket pointed to by fd
                    if remoteaddr.get_family() != sockhandle.domain as u16 {
                        return syscall_error(
                            Errno::EINVAL,
                            "connect",
                            "An address with an invalid family for the given domain was specified",
                        );
                    }

                    match sockhandle.protocol {
                        IPPROTO_UDP => {
                            return self.connect_udp(&mut *sockhandle, sockfdobj, remoteaddr)
                        }
                        IPPROTO_TCP => {
                            return self.connect_tcp(&mut *sockhandle, sockfdobj, remoteaddr)
                        }
                        _ => {
                            return syscall_error(
                                Errno::EOPNOTSUPP,
                                "connect",
                                "Unknown protocol in connect",
                            )
                        }
                    };
                }
                _ => {
                    return syscall_error(
                        Errno::ENOTSOCK,
                        "connect",
                        "file descriptor refers to something other than a socket",
                    );
                }
            }
        } else {
            return syscall_error(Errno::EBADF, "connect", "invalid file descriptor");
        }
    }

    //User datagram protocol is a standardized communication protocol that
    // transfers data between computers in a network. However, unlike other
    // protocols such as TCP, UDP simplifies data transfer by sending packets
    // (or, more specifically, datagrams) directly to the receiver without first
    // establishing a two-way connection. Read more at https://spiceworks.com/tech/networking/articles/user-datagram-protocol-udp/
    //
    //The function sets up a connection on a UDP socket
    //Args: sockhandle is a mut reference to the SocketHandle of the local socket
    //      sockfdobj is a mut reference to the Socket Description of the local
    // socket      remoteaddr is a reference to the remote address that the
    // local socket will connect to On success, zero is returned. On error,
    // -errno is returned, and errno is set to indicate the error.
    fn connect_udp(
        &self,
        sockhandle: &mut SocketHandle,
        sockfdobj: &mut SocketDesc,
        remoteaddr: &interface::GenSockaddr,
    ) -> i32 {
        //for UDP, just set the addresses and return
        //we don't need to check connection state for UDP, it's connectionless!
        sockhandle.remoteaddr = Some(remoteaddr.clone());
        match sockhandle.localaddr {
            //If the socket is assigned a local address, then return with success
            Some(_) => return 0,
            //Otherwise, assign a local address to the socket
            None => {
                //Note, assign_new_addr expects a reference to SocketHandle, but sockhandle is
                // a mut reference. This is why we need to dereference and
                // reference the sockhandle pointer An error occurs if the
                // sockhandle domain is not AF_UNIX, AF_INET, AF_INET6
                let localaddr = match Self::assign_new_addr(
                    &*sockhandle,
                    sockhandle.domain,
                    sockhandle.protocol & (1 << SO_REUSEPORT) != 0,
                ) {
                    Ok(a) => a,
                    Err(e) => return e,
                };

                //Set up the connection with the local address
                let bindret = self.bind_inner_socket(&mut *sockhandle, &localaddr, true);
                // Set the rawfd for select_syscall as we cannot implement the select
                // logics for AF_INET socket right now, so we have to call the select
                // syscall from libc, which takes the rawfd as the argument instead of
                // the fake fd used by lind.
                // The raw fd of the socket is the set to be the same as the fd set by the
                // kernel in the libc connect call
                sockfdobj.rawfd = sockhandle.innersocket.as_ref().unwrap().raw_sys_fd;
                return bindret;
            }
        };
    }

    //Transmission Control Protocol (TCP) is a standard protocol on the internet
    //that ensures the reliable transmission of data between devices on a network.
    //Read more at https://www.techtarget.com/searchnetworking/definition/TCP
    //
    //The function sets up a connection on a TCP socket
    //Args: sockhandle is a mut reference to the SocketHandle of the local socket
    //      sockfdobj is a mut reference to the Socket Description of the local
    // socket      remoteaddr is a reference to the remote address that the
    // local socket will connect to On success, zero is returned. On error,
    // -errno is returned, and errno is set to indicate the error.
    fn connect_tcp(
        &self,
        sockhandle: &mut SocketHandle,
        sockfdobj: &mut SocketDesc,
        remoteaddr: &interface::GenSockaddr,
    ) -> i32 {
        //BUG:
        // According to man pages, it may be possible dissolve the
        // association by connecting to an address with the sa_family member
        // of sockaddr set to AF_UNSPEC; thereafter, the socket can be
        // connected to another address.
        //
        //If socket is already connected, we can not reconnect with the same socket
        if sockhandle.state != ConnState::NOTCONNECTED {
            return syscall_error(
                Errno::EISCONN,
                "connect",
                "The descriptor is already connected",
            );
        }

        //In the case that the domain is AF_UNIX, AF_INET, or AF_INET6, perform a
        // connection Otherwise, return with an error due to the domain being
        // unsupported in lind
        match sockhandle.domain {
            AF_UNIX => self.connect_tcp_unix(&mut *sockhandle, sockfdobj, remoteaddr),
            AF_INET | AF_INET6 => self.connect_tcp_inet(&mut *sockhandle, sockfdobj, remoteaddr),
            _ => return syscall_error(Errno::EINVAL, "connect", "Unsupported domain provided"),
        }
    }

    //The function sets up a connection on a TCP socket with a unix address family
    //Args: sockhandle is a mut reference to the SocketHandle of the local socket
    //      sockfdobj is a mut reference to the Socket Description of the local
    // socket      remoteaddr is a reference to the remote address that the
    // local socket will connect to On success, zero is returned. On error,
    // -errno is returned, and errno is set to indicate the error.
    fn connect_tcp_unix(
        &self,
        sockhandle: &mut SocketHandle,
        sockfdobj: &mut SocketDesc,
        remoteaddr: &interface::GenSockaddr,
    ) -> i32 {
        // TCP domain socket logic
        //Check if the local address of the socket handle is not set
        if let None = sockhandle.localaddr {
            //Assign a new local address for the Unix domain socket in sockhandle. This is
            // necessary as each Unix domain socket needs a unique address
            // (path) to distinguish it from other sockets.
            let localaddr = Self::assign_new_addr_unix(&sockhandle);
            self.bind_inner_socket(&mut *sockhandle, &localaddr, false);
        }
        //Normalize the remote address to a path buffer
        let remotepathbuf = normpath(convpath(remoteaddr.path()), self);

        //NET_METADATA.domsock_paths is the set of all currently bound domain sockets
        //try to get and hold reference to the key-value pair, so other process can't
        // alter it
        let path_ref = NET_METADATA.domsock_paths.get(&remotepathbuf);
        // if the entry doesn't exist, return an error.
        if path_ref.is_none() {
            return syscall_error(Errno::ENOENT, "connect", "not valid unix domain path");
        }

        let (pipe1, pipe2) = create_unix_sockpipes();

        //Setup the socket handle with the remote address
        sockhandle.remoteaddr = Some(remoteaddr.clone());
        sockhandle.unix_info.as_mut().unwrap().sendpipe = Some(pipe1.clone());
        sockhandle.unix_info.as_mut().unwrap().receivepipe = Some(pipe2.clone());

        //Check if the socket is set to blocking mode
        //connvar is necessary to synchronize connect and accept
        //as we are performing it in the user space
        let connvar = if sockfdobj.flags & O_NONBLOCK == 0 {
            Some(interface::RustRfc::new(ConnCondVar::new()))
        } else {
            None
        };

        //The receive_pipe of the socket that is accepting the connection is assigned
        // the send_pipe of the socket that is requesting the connection.
        //Similiarly, the send_pipe of the socket that is accepting the connection is
        // assigned the receive_pipe of the socket that is requesting the
        // connection. Swapping the pipes here means that in accept sys call we
        // can use the pipes as placed in the domsock_accept_table without
        // confusion
        let entry = DomsockTableEntry {
            sockaddr: sockhandle.localaddr.unwrap().clone(),
            receive_pipe: Some(pipe1.clone()).unwrap(),
            send_pipe: Some(pipe2.clone()).unwrap(),
            cond_var: connvar.clone(),
        };
        //Access the domsock_accept_table, which keeps track of socket paths and
        //details pertaining to them: the socket address, receive and send pipes, and
        // cond_var
        NET_METADATA
            .domsock_accept_table
            .insert(remotepathbuf, entry);
        // TODO: Add logics to handle nonblocking connects here
        //Update the sock handle state to indicate that it is connected
        sockhandle.state = ConnState::CONNECTED;
        //If the socket is set to blocking mode, wait until a thread
        //accepts the connection
        if sockfdobj.flags & O_NONBLOCK == 0 {
            connvar.unwrap().wait();
        }
        return 0; //successful TCP connection over Unix domain
    }

    //The function sets up a connection on a TCP socket with an inet address family
    //Args: sockhandle is a mut reference to the SocketHandle of the local socket
    //      sockfdobj is a mut reference to the Socket Description of the local
    // socket      remoteaddr is a reference to the remote address that the
    // local socket will connect to On success, zero is returned. On error,
    // -errno is returned, and errno is set to indicate the error.
    fn connect_tcp_inet(
        &self,
        sockhandle: &mut SocketHandle,
        sockfdobj: &mut SocketDesc,
        remoteaddr: &interface::GenSockaddr,
    ) -> i32 {
        // TCP inet domain logic
        //for TCP, actually create the internal socket object and connect it
        let remoteclone = remoteaddr.clone();

        //In the case that the socket is connected, return with error
        //as we do not want a new connection
        if sockhandle.state != ConnState::NOTCONNECTED {
            return syscall_error(
                Errno::EISCONN,
                "connect",
                "The descriptor is already connected",
            );
        }

        //In the case that the socket is not connected
        if let None = sockhandle.localaddr {
            //Set the socket fd in the socket handle
            Self::force_innersocket(sockhandle);

            //Return a new address based on the domain of the socket handle
            //This won't return a clone of the local address as the socket handle
            //does not contain a local address based on the check above
            let localaddr = match Self::assign_new_addr(
                &*sockhandle,
                sockhandle.domain,
                sockhandle.protocol & (1 << SO_REUSEPORT) != 0,
            ) {
                Ok(a) => a,
                Err(e) => return e,
            };

            //Performs libc bind call to assign the local address to the fd in
            //Socket within innersocket
            //Any errors are a result of the libc bind call
            //Here are the list of possible errors https://man7.org/linux/man-pages/man2/bind.2.html
            let bindret = sockhandle.innersocket.as_ref().unwrap().bind(&localaddr);
            if bindret < 0 {
                sockhandle.localaddr = Some(localaddr);
                match Errno::from_discriminant(interface::get_errno()) {
                    Ok(i) => {
                        return syscall_error(
                            i,
                            "connect",
                            "The libc call to bind within connect failed",
                        );
                    }
                    Err(()) => {
                        panic!("Unknown errno value from socket bind within connect returned!")
                    }
                };
            }
        }

        let mut inprogress = false;
        //Performs libc connect call to connect the socket referred to by the
        // raw_sys_fd in Socket to the address specified by remoteclone
        //Here are the list of possible errors https://www.man7.org/linux/man-pages/man2/connect.2.html
        let connectret = sockhandle
            .innersocket
            .as_ref()
            .unwrap()
            .connect(&remoteclone);
        if connectret < 0 {
            match Errno::from_discriminant(interface::get_errno()) {
                //EINPROGRESS signifies that the socket is non-blocking and
                //the connection could not be established immediately.
                //https://www.gnu.org/software/libc/manual/html_node/Connecting.html
                // BUG: Another connect call on the same socket, before the connection is completely
                // established, should fail with EALREADY.
                Ok(i) => {
                    if i == Errno::EINPROGRESS {
                        inprogress = true;
                    } else {
                        return syscall_error(i, "connect", "The libc call to connect failed!");
                    };
                }
                Err(()) => panic!("Unknown errno value from socket connect returned!"),
            };
        }

        //Setup the socket handle as connected, insert the remote address of the
        // connection, and reset the errno in case it is set to EINPROGRESS
        sockhandle.state = ConnState::CONNECTED;
        sockhandle.remoteaddr = Some(remoteaddr.clone());
        sockhandle.errno = 0;
        // Set the rawfd for select_syscall as we cannot implement the select
        // logics for AF_INET socket right now, so we have to call the select
        // syscall from libc, which takes the rawfd as the argument instead of
        // the fake fd used by lind.
        // The raw fd of the socket is the set to be the same as the fd set by the
        // kernel in the libc connect call
        sockfdobj.rawfd = sockhandle.innersocket.as_ref().unwrap().raw_sys_fd;
        if inprogress {
            sockhandle.state = ConnState::INPROGRESS;
            return syscall_error(
                Errno::EINPROGRESS,
                "connect",
                "The libc call to connect is in progress.",
            );
        } else {
            return 0; //successfull TCP connection over INET domain
        }
    }

    fn mksockhandle(
        domain: i32,
        socktype: i32,
        protocol: i32,
        conn: ConnState,
        socket_options: i32,
    ) -> SocketHandle {
        SocketHandle {
            innersocket: None,
            socket_options: socket_options,
            tcp_options: 0,
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

    /// ### Description
    ///
    /// `sendto_syscall` sends a message on a socket
    ///
    ///  Note:
    ///  send(fd, buf, buflen, flags);
    ///  is equivalent to
    ///  sendto(fd, buf, buflen, flags, destaddr);
    ///  where destaddr has an unspecified port or IP
    ///
    /// ### Arguments
    ///
    /// it accepts five parameters:
    /// * `fd` - the file descriptor of the sending socket
    /// * `buf` - the message is found in buf
    /// * `buflen` - the len of the message found in buf
    /// * `flags` - bitwise OR of zero or more flags. Refer to man page to find
    ///   possible args
    /// * 'destaddr' - the address of the target socket
    ///
    /// ### Returns
    ///
    /// On success, the call returns the number of bytes sent. On error, a
    /// negative error number is returned, with the errorno set to represent
    /// the corresponding error
    ///
    /// ### Errors
    ///
    /// These are some standard errors generated by the socket layer.
    ///    Additional errors may be generated and returned from the
    ///    underlying protocol modules; see their respective manual pages.
    ///
    /// * EACCES - (For UNIX domain sockets, which are identified by pathname)
    ///   Write permission is denied on the destination socket file, or search
    ///   permission is denied for one of the directories the path prefix.  (See
    ///   path_resolution(7).)  (For UDP sockets) An attempt was made to send to
    ///   a network/broadcast address as though it was a unicast address.
    ///
    /// ** EAGAIN or EWOULDBLOCK - The socket is marked nonblocking and the
    ///   requested operation would block.  POSIX.1-2001 allows either error to
    ///   be returned for this case, and does not require these constants to
    ///   have the same value, so a portable application should check for both
    ///   possibilities.
    ///
    /// ** EAGAIN - (Internet domain datagram sockets) The socket referred to by
    ///   sockfd had not previously been bound to an address and, upon
    ///   attempting to bind it to an ephemeral port, it was determined that all
    ///   port numbers in the ephemeral port range are currently in use.  See
    ///   the discussion of /proc/sys/net/ipv4/ip_local_port_range in ip(7).
    ///
    /// * EALREADY - Another Fast Open is in progress.
    ///
    /// ** EBADF - sockfd is not a valid open file descriptor.
    ///
    /// * ECONNRESET - Connection reset by peer.
    ///
    /// * EDESTADDRREQ - The socket is not connection-mode, and no peer address
    ///   is set.
    ///
    /// * EFAULT - An invalid user space address was specified for an argument.
    ///
    /// * EINTR - A signal occurred before any data was transmitted; see
    ///   signal(7).
    ///
    /// ** EINVAL - Invalid argument passed.
    ///
    /// * EISCONN - The connection-mode socket was connected already but a
    ///   recipient was specified.  (Now either this error is returned, or the
    ///   recipient specification is ignored.)
    ///
    /// * EMSGSIZE - The socket type requires that message be sent atomically,
    ///   and the size of the message to be sent made this impossible.
    ///
    /// * ENOBUFS - The output queue for a network interface was full.  This
    ///   generally indicates that the interface has stopped sending, but may be
    ///   caused by transient congestion. (Normally, this does not occur in
    ///   Linux.  Packets are just silently dropped when a device queue
    ///   overflows.)
    ///
    /// * ENOMEM - No memory available.
    ///
    /// ** ENOTCONN - The socket is not connected, and no target has been given.
    ///
    /// ** ENOTSOCK - The file descriptor sockfd does not refer to a socket.
    ///
    /// ** EOPNOTSUPP - Some bit in the flags argument is inappropriate for the
    ///   socket type.
    ///
    /// ** EPIPE - The local end has been shut down on a connection oriented
    ///   socket.  In this case, the process will also receive a SIGPIPE unless
    ///   MSG_NOSIGNAL is set.
    ///
    /// ** Indicates the error may be returned from RustPOSIX
    ///
    /// ### Panics TODO:
    ///
    /// * invalid or out-of-bounds file descriptor, calling unwrap() on it will
    ///   cause a panic.
    /// * Unknown errno value from sendto returned, will cause panic.
    ///
    /// for more detailed description of all the commands and return values, see
    /// [send(2)](https://linux.die.net/man/2/send)
    pub fn sendto_syscall(
        &self,
        fd: i32,
        buf: *const u8,
        buflen: usize,
        flags: i32,
        dest_addr: &interface::GenSockaddr,
    ) -> i32 {
        //if ip and port are not specified, shunt off to send
        //to check for a possible connection to another socket that may exist
        if dest_addr.port() == 0 && dest_addr.addr().is_unspecified() {
            return self.send_syscall(fd, buf, buflen, flags);
        }
        //BUG:
        //If fd is out of range of [0,MAXFD], process will panic
        //Otherwise, we obtain a write gaurd to the Option<FileDescriptor> object
        let checkedfd = self.get_filedescriptor(fd).unwrap();
        let mut unlocked_fd = checkedfd.write();
        //Check if the write gaurd holds a valid FileDescriptor
        if let Some(filedesc_enum) = &mut *unlocked_fd {
            match filedesc_enum {
                //In this case, the file descriptor refers to a socket
                Socket(ref mut sockfdobj) => {
                    //Grab a write guard to the socket handle
                    let sock_tmp = sockfdobj.handle.clone();
                    let mut sockhandle = sock_tmp.write();

                    //If the socket's domain is UNIX, return with error as UNIX
                    //sockets are connection based.
                    //TODO: Check whether the socket is connected and return
                    //EISCONN or ENOTCONN accordingly.
                    if sockhandle.domain == AF_UNIX {
                        return syscall_error(
                            Errno::EISCONN,
                            "sendto",
                            "The descriptor is connection-oriented",
                        );
                    }

                    //The destaddr's address family must match that of the
                    //socket's address from 'fd'. Otherwise, a message can't be sent
                    if dest_addr.get_family() != sockhandle.domain as u16 {
                        return syscall_error(
                            Errno::EINVAL,
                            "sendto",
                            "An address with an invalid family for the given domain was specified",
                        );
                    }

                    //If sendto_syscall is used on a connection-mode socket, then
                    // the error EISCONN may be returned when destaddr is not NULL,
                    // as we checked above
                    if sockhandle.state != ConnState::NOTCONNECTED {
                        return syscall_error(
                            Errno::EISCONN,
                            "sendto",
                            "The descriptor is connected",
                        );
                    }

                    //Pattern match based on the socket's protocol
                    match sockhandle.protocol {
                        //The TCP protocol is a connection-mode
                        //If sendto_syscall is used on a connection-mode socket, then
                        // the error EISCONN may be returned when destaddr is not NULL,
                        // as we checked above
                        IPPROTO_TCP => {
                            return syscall_error(
                                Errno::EISCONN,
                                "sendto",
                                "The descriptor is connection-oriented",
                            );
                        }

                        //UDP protocol
                        IPPROTO_UDP => {
                            //An implicit bind refers to the automatic binding of a socket to an
                            // address and port by the system, without
                            // an explicit call to the bind() function by
                            // the programmer.
                            let tmpdest = *dest_addr;
                            let ibindret =
                                self._implicit_bind(&mut *sockhandle, tmpdest.get_family() as i32);
                            //Call to _implicit_bind may panic upon unknown error values
                            //Otherwise, the error value is returned here and passed through
                            if ibindret < 0 {
                                return ibindret;
                            }

                            //unwrap is safe because of the call to _implicit_bind
                            //above. innersocket/raw_sys_fd will be set since
                            //lind passes TCP sockets to the OS to partially handle.
                            //Here we call sendto from libc
                            let sockret = sockhandle.innersocket.as_ref().unwrap().sendto(
                                buf,
                                buflen,
                                Some(dest_addr),
                            );

                            //If the call to sendto from libc returns
                            //-1, indicating an error, retrieve the err
                            //and return appropriately
                            //Otherwise, return the number of bytes
                            //written to the connected socket
                            if sockret < 0 {
                                match Errno::from_discriminant(interface::get_errno()) {
                                    Ok(i) => {
                                        return syscall_error(
                                            i,
                                            "sendto",
                                            "The libc call to sendto failed!",
                                        );
                                    }
                                    Err(()) => {
                                        panic!("Unknown errno value from socket sendto returned!")
                                    }
                                };
                            } else {
                                return sockret; //on success, return the number
                                                // of bytes sent
                            }
                        }
                        //If the protocol of the socket is not TCP or UDP,
                        //lind does not support it
                        _ => {
                            return syscall_error(
                                Errno::EOPNOTSUPP,
                                "sendto",
                                "Unkown protocol in sendto",
                            );
                        }
                    }
                }
                //If the file descriptor does not refer to a socket,
                //return with error
                _ => {
                    return syscall_error(
                        Errno::ENOTSOCK,
                        "sendto",
                        "file descriptor refers to something other than a socket",
                    );
                }
            }
        //Otherwise, the write gaurd does not hold a FileDescriptor
        } else {
            return syscall_error(Errno::EBADF, "sendto", "invalid file descriptor");
        }
    }

    /// ### Description
    ///
    /// `send_syscall` sends a message on a socket
    ///
    ///  The send() call may be used only when the socket is in a
    ///  connected state (so that the intended recipient is known).
    ///
    /// ### Arguments
    ///
    /// it accepts four parameters:
    /// * `fd` - the file descriptor of the sending socket
    /// * `buf` - the message is found in buf
    /// * `buflen` - the len of the message found in buf
    /// * `flags` - bitwise OR of zero or more flags. Refer to man page to find
    ///   possible args
    ///
    /// ### Returns
    ///
    /// On success, the call returns the number of bytes sent. On error, a
    /// negative error number is returned, with the errorno set to represent
    /// the corresponding error
    ///
    /// ### Errors
    ///
    /// These are some standard errors generated by the socket layer.
    ///    Additional errors may be generated and returned from the
    ///    underlying protocol modules; see their respective manual pages.
    ///
    /// * EACCES - (For UNIX domain sockets, which are identified by pathname)
    ///   Write permission is denied on the destination socket file, or search
    ///   permission is denied for one of the directories the path prefix.  (See
    ///   path_resolution(7).)  (For UDP sockets) An attempt was made to send to
    ///   a network/broadcast address as though it was a unicast address.
    ///
    /// ** EAGAIN or EWOULDBLOCK - The socket is marked nonblocking and the
    ///   requested operation would block.  POSIX.1-2001 allows either error to
    ///   be returned for this case, and does not require these constants to
    ///   have the same value, so a portable application should check for both
    ///   possibilities.
    ///
    /// ** EAGAIN - (Internet domain datagram sockets) The socket referred to by
    ///   sockfd had not previously been bound to an address and, upon
    ///   attempting to bind it to an ephemeral port, it was determined that all
    ///   port numbers in the ephemeral port range are currently in use.  See
    ///   the discussion of /proc/sys/net/ipv4/ip_local_port_range in ip(7).
    ///
    /// * EALREADY - Another Fast Open is in progress.
    ///
    /// ** EBADF - sockfd is not a valid open file descriptor.
    ///
    /// * ECONNRESET - Connection reset by peer.
    ///
    /// * EDESTADDRREQ - The socket is not connection-mode, and no peer address
    ///   is set.
    ///
    /// * EFAULT - An invalid user space address was specified for an argument.
    ///
    /// * EINTR - A signal occurred before any data was transmitted; see
    ///   signal(7).
    ///
    /// ** EINVAL - Invalid argument passed.
    ///
    /// * EISCONN - The connection-mode socket was connected already but a
    ///   recipient was specified.  (Now either this error is returned, or the
    ///   recipient specification is ignored.)
    ///
    /// * EMSGSIZE - The socket type requires that message be sent atomically,
    ///   and the size of the message to be sent made this impossible.
    ///
    /// * ENOBUFS - The output queue for a network interface was full.  This
    ///   generally indicates that the interface has stopped sending, but may be
    ///   caused by transient congestion. (Normally, this does not occur in
    ///   Linux.  Packets are just silently dropped when a device queue
    ///   overflows.)
    ///
    /// * ENOMEM - No memory available.
    ///
    /// ** ENOTCONN - The socket is not connected, and no target has been given.
    ///
    /// ** ENOTSOCK - The file descriptor sockfd does not refer to a socket.
    ///
    /// ** EOPNOTSUPP - Some bit in the flags argument is inappropriate for the
    ///   socket type.
    ///
    /// ** EPIPE - The local end has been shut down on a connection oriented
    ///   socket.  In this case, the process will also receive a SIGPIPE unless
    ///   MSG_NOSIGNAL is set.
    ///
    /// ** Indicates the error may be returned from RustPOSIX
    ///
    /// ### Panics TODO:
    ///
    /// * invalid or out-of-bounds file descriptor, calling unwrap() on it will
    ///   cause a panic.
    /// * Unknown errno value from sendto returned, will cause panic.
    ///
    /// for more detailed description of all the commands and return values, see
    /// [send(2)](https://linux.die.net/man/2/send)
    pub fn send_syscall(&self, fd: i32, buf: *const u8, buflen: usize, flags: i32) -> i32 {
        //BUG:
        //If fd is out of range of [0,MAXFD], process will panic
        //Otherwise, we obtain a write gaurd to the Option<FileDescriptor> object
        let checkedfd = self.get_filedescriptor(fd).unwrap();
        let mut unlocked_fd = checkedfd.write();
        //Check if the write gaurd holds a valid FileDescriptor
        if let Some(filedesc_enum) = &mut *unlocked_fd {
            match filedesc_enum {
                //In this case, the file descriptor refers to a socket
                Socket(ref mut sockfdobj) => {
                    //Grab a write guard to the socket handle
                    let sock_tmp = sockfdobj.handle.clone();
                    let sockhandle = sock_tmp.write();

                    //Pattern match based on the domain of the socket
                    //Lind handles UNIX sockets internally,
                    //but will call send from libc for INET sockets
                    let socket_type = sockhandle.domain;
                    match socket_type {
                        AF_UNIX => {
                            //Pattern match based on the socket protocol
                            match sockhandle.protocol {
                                //TCP socket
                                //Across UNIX connections, it rarely exists
                                //that it is preferable to use UDP sockets
                                //rather than TCP sockets.
                                //As of right now, lind only implements
                                //UNIX sockets with the TCP protocol
                                IPPROTO_TCP => {
                                    //For a TCP socket to be able to send here we
                                    //either need to be fully connected, or connected for write
                                    // only
                                    if (sockhandle.state != ConnState::CONNECTED)
                                        && (sockhandle.state != ConnState::CONNWRONLY)
                                    {
                                        //Otherwise, return with an error as
                                        //TCP sockets taht aren't connected
                                        //can't send messages
                                        return syscall_error(
                                            Errno::ENOTCONN,
                                            "writev",
                                            "The descriptor is not connected",
                                        );
                                    }
                                    // get the socket pipe, write to it, and return bytes written
                                    let sockinfo = &sockhandle.unix_info.as_ref().unwrap();
                                    //When the message does not fit into the send buffer of the
                                    // socket, send() normally
                                    // blocks, unless the socket has been placed in
                                    // nonblocking I/O mode.  In nonblocking mode it would fail with
                                    // the error EAGAIN or
                                    // EWOULDBLOCK in this case.
                                    let mut nonblocking = false;
                                    if sockfdobj.flags & O_NONBLOCK != 0 {
                                        nonblocking = true;
                                    }
                                    let retval = match sockinfo.sendpipe.as_ref() {
                                        //sendpipe is available in unix socket info
                                        //it is needed to send the message found in buf
                                        //to the connected socket
                                        Some(sendpipe) => {
                                            sendpipe.write_to_pipe(buf, buflen, nonblocking) as i32
                                        }
                                        //sendpipe is not available in unix socket info
                                        //transmission of data is not possible so return with error
                                        None => {
                                            return syscall_error(Errno::EAGAIN, "writev", "there is no data available right now, try again later");
                                        }
                                    };
                                    //In the case that the write_to_pipe call returns EPIPE,
                                    // meaning the local end has been shut down on a connection
                                    // oriented socket. Then, the process will also receive
                                    // a SIGPIPE unless MSG_NOSIGNAL is set.
                                    if (retval == -(Errno::EPIPE as i32))
                                        && ((flags & MSG_NOSIGNAL) == 0)
                                    {
                                        // The default action for SIGPIPE is to terminate the
                                        // process without a core dump. This simplifies error
                                        // handling in programs that
                                        // are meant to run as part of a shell pipeline: reading
                                        // input, transforming it, and then writing it to another
                                        // process. SIGPIPE allows the program to skip error
                                        // handling and blindly write data until it’s killed
                                        //
                                        // Trigger SIGPIPE
                                        interface::lind_kill_from_id(self.cageid, SIGPIPE);
                                    }
                                    retval //on success, return number of bytes
                                           // sent
                                }
                                //PROTOCOL is not TCP
                                _ => {
                                    return syscall_error(
                                        Errno::EOPNOTSUPP,
                                        "send",
                                        "Unkown protocol in send",
                                    );
                                }
                            }
                        }
                        //Pattern match based on the socket protocol
                        AF_INET | AF_INET6 => {
                            match sockhandle.protocol {
                                //For a TCP socket to be able to send here we
                                //either need to be fully connected, or connected for write
                                // only
                                IPPROTO_TCP => {
                                    if (sockhandle.state != ConnState::CONNECTED)
                                        && (sockhandle.state != ConnState::CONNWRONLY)
                                    {
                                        //Otherwise, return with an error as
                                        //TCP sockets taht aren't connected
                                        //can't send messages
                                        return syscall_error(
                                            Errno::ENOTCONN,
                                            "send",
                                            "The descriptor is not connected",
                                        );
                                    }

                                    //We passed the above check so the TCP socket must be connected
                                    //Hence, it must have a valid inner socket/raw sys fd
                                    //Call sendto from libc to send the buff
                                    let retval = sockhandle
                                        .innersocket
                                        .as_ref()
                                        .unwrap()
                                        .sendto(buf, buflen, None);
                                    //If the call to sendto from libc returns
                                    //-1, indicating an error, retrieve the err
                                    //and return appropriately
                                    //Otherwise, return the number of bytes
                                    //written to the connected socket
                                    if retval < 0 {
                                        match Errno::from_discriminant(interface::get_errno()) {
                                            Ok(i) => {
                                                return syscall_error(
                                                    i,
                                                    "send",
                                                    "The libc call to sendto failed!",
                                                );
                                            }
                                            Err(()) => panic!(
                                                "Unknown errno value from socket sendto returned!"
                                            ),
                                        };
                                    } else {
                                        return retval; //return the number of
                                                       // bytes written to the
                                                       // connected socket
                                    }
                                }

                                //For INET sockets following the UDP protocol,
                                //we don't need to check for connection status
                                //as UDP is connection-less. This lets us grab
                                //the remote address of the socket and send the
                                //message in buf to it by calling sendto_syscall
                                IPPROTO_UDP => {
                                    let remoteaddr = match &sockhandle.remoteaddr {
                                        Some(x) => x.clone(),
                                        None => {
                                            return syscall_error(
                                                Errno::ENOTCONN,
                                                "send",
                                                "The descriptor is not connected",
                                            );
                                        }
                                    };
                                    //** Why is it necessary to drop here?? */
                                    drop(unlocked_fd);
                                    drop(sockhandle);
                                    //remote address is set in sendto from libc
                                    //as UDP socket is connection-less
                                    return self.sendto_syscall(
                                        fd,
                                        buf,
                                        buflen,
                                        flags,
                                        &remoteaddr,
                                    ); //return the number of bytes written to the connected socket
                                    //** TODO: Add error checking on the return value */
                                }

                                //Protcol besides UDP and TCP are not supported
                                //for INET sockets in lind
                                _ => {
                                    return syscall_error(
                                        Errno::EOPNOTSUPP,
                                        "send",
                                        "Unkown protocol in send",
                                    );
                                }
                            }
                        }
                        //If the domain of the socket is not UNIX or INET
                        //lind does not support it
                        _ => {
                            return syscall_error(
                                Errno::EINVAL,
                                "connect",
                                "Unsupported domain provided",
                            )
                        }
                    }
                }
                //If the file descriptor does not refer to a socket,
                //return with error
                _ => {
                    return syscall_error(
                        Errno::ENOTSOCK,
                        "send",
                        "file descriptor refers to something other than a socket",
                    );
                }
            }
        //Otherwise, the write gaurd does not hold a FileDescriptor
        } else {
            return syscall_error(Errno::EBADF, "send", "invalid file descriptor");
        }
    }

    fn recv_common_inner(
        &self,
        filedesc_enum: &mut FileDescriptor,
        buf: *mut u8,
        buflen: usize,
        flags: i32,
        addr: &mut Option<&mut interface::GenSockaddr>,
    ) -> i32 {
        match &mut *filedesc_enum {
            Socket(ref mut sockfdobj) => {
                let sock_tmp = sockfdobj.handle.clone();
                let mut sockhandle = sock_tmp.write();
                match sockhandle.protocol {
                    IPPROTO_TCP => {
                        return self.recv_common_inner_tcp(
                            &mut sockhandle,
                            sockfdobj,
                            buf,
                            buflen,
                            flags,
                            addr,
                        )
                    }
                    IPPROTO_UDP => {
                        return self.recv_common_inner_udp(
                            &mut sockhandle,
                            sockfdobj,
                            buf,
                            buflen,
                            addr,
                        )
                    }

                    _ => {
                        return syscall_error(
                            Errno::EOPNOTSUPP,
                            "recvfrom",
                            "Unkown protocol in recvfrom",
                        );
                    }
                }
            }
            _ => {
                return syscall_error(
                    Errno::ENOTSOCK,
                    "recvfrom",
                    "file descriptor refers to something other than a socket",
                );
            }
        }
    }

    fn recv_common_inner_tcp(
        &self,
        sockhandle: &mut interface::RustLockWriteGuard<SocketHandle>,
        sockfdobj: &mut SocketDesc,
        buf: *mut u8,
        buflen: usize,
        flags: i32,
        addr: &mut Option<&mut interface::GenSockaddr>,
    ) -> i32 {
        // maybe select reported a INPROGRESS tcp socket as readable, so re-check the
        // state here
        if sockhandle.state == ConnState::INPROGRESS
            && sockhandle
                .innersocket
                .as_ref()
                .unwrap()
                .check_rawconnection()
        {
            sockhandle.state = ConnState::CONNECTED;
        }

        if (sockhandle.state != ConnState::CONNECTED) && (sockhandle.state != ConnState::CONNRDONLY)
        {
            return syscall_error(
                Errno::ENOTCONN,
                "recvfrom",
                "The descriptor is not connected",
            );
        }

        let mut newbuflen = buflen;
        let mut newbufptr = buf;

        //if we have peeked some data before, fill our buffer with that data before
        // moving on
        if !sockhandle.last_peek.is_empty() {
            let bytecount = interface::rust_min(sockhandle.last_peek.len(), newbuflen);
            interface::copy_fromrustdeque_sized(buf, bytecount, &sockhandle.last_peek);
            newbuflen -= bytecount;
            newbufptr = newbufptr.wrapping_add(bytecount);

            //if we're not still peeking data, consume the data we peeked from our peek
            // buffer and if the bytecount is more than the length of the peeked
            // data, then we remove the entire buffer
            if flags & MSG_PEEK == 0 {
                let len = sockhandle.last_peek.len();
                sockhandle
                    .last_peek
                    .drain(..(if bytecount > len { len } else { bytecount }));
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
            if sockfdobj.flags & O_NONBLOCK != 0 {
                nonblocking = true;
            }
            loop {
                let sockinfo = &sockhandle.unix_info.as_ref().unwrap();
                let receivepipe = sockinfo.receivepipe.as_ref().unwrap();
                retval = receivepipe.read_from_pipe(bufleft, buflenleft, nonblocking) as i32;
                if retval < 0 {
                    //If we have already read from a peek but have failed to read more, exit!
                    if buflen != buflenleft {
                        return (buflen - buflenleft) as i32;
                    }
                    if sockfdobj.flags & O_NONBLOCK == 0 && retval == -(Errno::EAGAIN as i32) {
                        // with blocking sockets, we return EAGAIN here to check for cancellation,
                        // then return to reading
                        if self
                            .cancelstatus
                            .load(interface::RustAtomicOrdering::Relaxed)
                        {
                            // if the cancel status is set in the cage, we trap around a cancel
                            // point until the individual thread is
                            // signaled to cancel itself
                            loop {
                                interface::cancelpoint(self.cageid)
                            }
                        }
                        // in order to prevent deadlock
                        interface::RustLockWriteGuard::<SocketHandle>::bump(sockhandle);
                        continue;
                    } else {
                        //if not EAGAIN, return the error
                        return retval;
                    }
                }
                break;
            }
        } else {
            loop {
                // we loop here so we can cancel blocking recvs
                //socket must be connected so unwrap ok
                if sockfdobj.flags & O_NONBLOCK != 0 {
                    retval = sockhandle
                        .innersocket
                        .as_ref()
                        .unwrap()
                        .recvfrom_nonblocking(bufleft, buflenleft, addr);
                } else {
                    retval = sockhandle
                        .innersocket
                        .as_ref()
                        .unwrap()
                        .recvfrom(bufleft, buflenleft, addr);
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
                                if self
                                    .cancelstatus
                                    .load(interface::RustAtomicOrdering::Relaxed)
                                {
                                    // if the cancel status is set in the cage, we trap around a
                                    // cancel point
                                    // until the individual thread is signaled to cancel itself
                                    loop {
                                        interface::cancelpoint(self.cageid);
                                    }
                                }
                                interface::RustLockWriteGuard::<SocketHandle>::bump(sockhandle);
                                continue; // EAGAIN, try again
                            }

                            return syscall_error(
                                i,
                                "recvfrom",
                                "Internal call to recvfrom failed",
                            );
                        }
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

    fn recv_common_inner_udp(
        &self,
        sockhandle: &mut interface::RustLockWriteGuard<SocketHandle>,
        sockfdobj: &mut SocketDesc,
        buf: *mut u8,
        buflen: usize,
        addr: &mut Option<&mut interface::GenSockaddr>,
    ) -> i32 {
        let binddomain = if let Some(baddr) = addr {
            baddr.get_family() as i32
        } else {
            AF_INET
        };

        let ibindret = self._implicit_bind(&mut *sockhandle, binddomain);
        if ibindret < 0 {
            return ibindret;
        }

        loop {
            // loop for blocking sockets
            //if the remoteaddr is set and addr is not, use remoteaddr
            //unwrap is ok because of implicit bind
            let retval = if let (None, Some(ref mut remoteaddr)) = (&addr, sockhandle.remoteaddr) {
                sockhandle.innersocket.as_ref().unwrap().recvfrom(
                    buf,
                    buflen,
                    &mut Some(remoteaddr),
                )
            } else {
                sockhandle
                    .innersocket
                    .as_ref()
                    .unwrap()
                    .recvfrom(buf, buflen, addr)
            };

            if retval < 0 {
                match Errno::from_discriminant(interface::get_errno()) {
                    Ok(i) => {
                        if sockfdobj.flags & O_NONBLOCK == 0 && i == Errno::EAGAIN {
                            if self
                                .cancelstatus
                                .load(interface::RustAtomicOrdering::Relaxed)
                            {
                                // if the cancel status is set in the cage, we trap around a cancel
                                // point until the individual thread
                                // is signaled to cancel itself
                                loop {
                                    interface::cancelpoint(self.cageid);
                                }
                            }
                            interface::RustLockWriteGuard::<SocketHandle>::bump(sockhandle);
                            continue; //received EAGAIN on blocking socket, try
                                      // again
                        }
                        return syscall_error(i, "recvfrom", "Internal call to recvfrom failed");
                    }
                    Err(()) => panic!("Unknown errno value from socket recvfrom returned!"),
                };
            } else {
                return retval; // we can proceed
            }
        }
    }

    pub fn recv_common(
        &self,
        fd: i32,
        buf: *mut u8,
        buflen: usize,
        flags: i32,
        addr: &mut Option<&mut interface::GenSockaddr>,
    ) -> i32 {
        let checkedfd = self.get_filedescriptor(fd).unwrap();
        let mut unlocked_fd = checkedfd.write();
        if let Some(ref mut filedesc_enum) = &mut *unlocked_fd {
            return self.recv_common_inner(filedesc_enum, buf, buflen, flags, addr);
        } else {
            return syscall_error(Errno::EBADF, "recvfrom", "invalid file descriptor");
        }
    }

    pub fn recvfrom_syscall(
        &self,
        fd: i32,
        buf: *mut u8,
        buflen: usize,
        flags: i32,
        addr: &mut Option<&mut interface::GenSockaddr>,
    ) -> i32 {
        return self.recv_common(fd, buf, buflen, flags, addr);
    }

    pub fn recv_syscall(&self, fd: i32, buf: *mut u8, buflen: usize, flags: i32) -> i32 {
        return self.recv_common(fd, buf, buflen, flags, &mut None);
    }

    /// ### Description
    ///
    /// `listen_syscall` listen for connections on a socket
    ///
    /// ### Arguments
    ///
    /// it accepts two parameters:
    /// * `sockfd` - a file descriptor that refers to a socket of type
    ///   SOCK_STREAM Note, we do not implement sockets of type SOCK_SEQPACKET
    /// * `backlog` - defines the maximum length to which the queue of pending
    ///   connections for sockfd may grow.  If a connection request arrives when
    ///   the queue is full, the client may receive an error with an indication
    ///   of ECONNREFUSED or, if the underlying protocol supports
    ///   retransmission, the request may be ignored so that a later reattempt
    ///   at connection succeeds.
    ///
    /// ### Returns
    ///
    /// for a successful call, zero is returned. On error, -errno is
    /// returned and errno is set to indicate the error
    ///
    /// ### Errors
    ///
    /// * EADDRINUSE - Another socket is already listening on the same port.
    ///
    /// * EADDRINUSE - (Internet domain sockets) The socket referred to by
    ///   sockfd had not previously been bound to an address and, upon
    ///   attempting to bind it to an ephemeral port, it was determined that all
    ///   port numbers in the ephemeral port range are currently in use.  See
    ///   the discussion of /proc/sys/net/ipv4/ip_local_port_range in ip(7).
    ///
    /// * EBADF - The argument sockfd is not a valid file descriptor.
    ///
    /// * ENOTSOCK - The file descriptor sockfd does not refer to a socket.
    ///
    /// * EOPNOTSUPP - The socket is not of a type that supports the listen()
    ///   operation.
    ///
    /// ### Panics
    ///
    /// * invalid or out-of-bounds file descriptor), calling unwrap() on it will
    ///   cause a panic.
    /// * unknown errno value from socket bind sys call from libc in the case
    ///   that the socket isn't assigned an address
    /// * unknown errno value from socket listen sys call from libc
    ///
    /// for more detailed description of all the commands and return values, see
    /// [listen(2)](https://linux.die.net/man/2/listen)
    //
    // TODO: We are currently ignoring backlog
    pub fn listen_syscall(&self, fd: i32, backlog: i32) -> i32 {
        //BUG:s
        //If fd is out of range of [0,MAXFD], process will panic
        //Otherwise, we obtain a write gaurd to the Option<FileDescriptor> object
        let checkedfd = self.get_filedescriptor(fd).unwrap();
        let mut unlocked_fd = checkedfd.write();
        if let Some(filedesc_enum) = &mut *unlocked_fd {
            match filedesc_enum {
                //If the file descriptor refers to a socket
                Socket(ref mut sockfdobj) => {
                    //get or create the socket and bind it before listening
                    //Gain write access to the socket handle
                    let sock_tmp = sockfdobj.handle.clone();
                    let mut sockhandle = sock_tmp.write();

                    //If the given socket is already listening, return with
                    //success
                    match sockhandle.state {
                        ConnState::LISTEN => {
                            return 0;
                        }

                        //Possible connection states in which the socket
                        //can not be set to listening mode:
                        // * Connected to another socket and can send
                        // and receive data
                        // * Connected to another socket and can only send
                        // data
                        // * Connected to another socket and can only receive
                        // data
                        // * A non-blocking socket is in progress of connecting
                        // to another socket
                        ConnState::CONNECTED
                        | ConnState::CONNRDONLY
                        | ConnState::CONNWRONLY
                        | ConnState::INPROGRESS => {
                            return syscall_error(
                                Errno::EOPNOTSUPP,
                                "listen",
                                "We don't support closing a prior socket connection on listen",
                            );
                        }

                        //If the given socket is not connected, it is ready
                        //to begin listening
                        ConnState::NOTCONNECTED => {
                            //If the given socket is not a TCP socket, then the
                            //socket can not listen for connections
                            if sockhandle.protocol != IPPROTO_TCP {
                                return syscall_error(
                                    Errno::EOPNOTSUPP,
                                    "listen",
                                    "This protocol doesn't support listening",
                                );
                            }

                            //TODO: Implement backlog for UNIX
                            //If the given socket is a Unix socket, lind handles
                            //the connection, return with success
                            if sockhandle.domain == AF_UNIX {
                                sockhandle.state = ConnState::LISTEN;
                                return 0;
                            }

                            //If the given socket is not assigned an address,
                            //attempt to bind the socket to an address.
                            //
                            //An implicit bind refers to the automatic binding of a socket to an
                            // address and port by the system, without
                            // an explicit call to the bind() function by
                            // the programmer.
                            //
                            //If implicit bind fails, return with the errno if known
                            //Otherwise, panic!
                            if sockhandle.localaddr.is_none() {
                                let shd = sockhandle.domain as i32;
                                let ibindret = self._implicit_bind(&mut *sockhandle, shd);
                                if ibindret < 0 {
                                    match Errno::from_discriminant(interface::get_errno()) {
                                        Ok(i) => {return syscall_error(i, "listen", "The libc call to bind within listen failed");},
                                        Err(()) => panic!("Unknown errno value from socket bind within listen returned!"),
                                    };
                                }
                            }

                            //The socket must have been assigned an address by implicit bind
                            let ladr = sockhandle.localaddr.unwrap().clone();
                            //Grab a tuple of the address, port, and port type
                            //to be inserted into the set of listening ports
                            let porttuple = mux_port(
                                ladr.addr().clone(),
                                ladr.port(),
                                sockhandle.domain,
                                TCPPORT,
                            );

                            //Set the socket connection state to listening
                            //to readily accept connections
                            NET_METADATA.listening_port_set.insert(porttuple.clone());
                            sockhandle.state = ConnState::LISTEN;

                            //Call listen from libc on the socket
                            let listenret =
                                sockhandle.innersocket.as_ref().unwrap().listen(backlog);
                            if listenret < 0 {
                                let lr = match Errno::from_discriminant(interface::get_errno()) {
                                    Ok(i) => syscall_error(
                                        i,
                                        "listen",
                                        "The libc call to listen failed!",
                                    ),
                                    Err(()) => {
                                        panic!("Unknown errno value from socket listen returned!")
                                    }
                                };
                                //Remove the tuple of the address, port, and
                                //port type from the set of listening ports
                                //as we are returning from an error
                                NET_METADATA.listening_port_set.remove(&porttuple);

                                //Set the socket state to NOTCONNECTED, as
                                //the socket is not listening
                                sockhandle.state = ConnState::NOTCONNECTED;
                                return lr;
                            };

                            // Set the rawfd for select_syscall as we cannot implement the select
                            // logics for AF_INET socket right now, so we have to call the select
                            // syscall from libc, which takes the rawfd as the argument instead of
                            // the fake fd used by lind.
                            // The raw fd of the socket is the set to be the same as the fd set by
                            // the kernal in the libc connect call
                            sockfdobj.rawfd = sockhandle.innersocket.as_ref().unwrap().raw_sys_fd;

                            //If listening socket is not in the table of pending
                            //connections, we must insert it as the key with
                            //an empty vector as the value
                            //We can now track incoming connections
                            if !NET_METADATA.pending_conn_table.contains_key(&porttuple) {
                                NET_METADATA
                                    .pending_conn_table
                                    .insert(porttuple.clone(), vec![]);
                            }

                            return 0; //return on success
                        }
                    }
                }

                //Otherwise, the file descriptor refers to something other
                //than a socket, return with error
                _ => {
                    return syscall_error(
                        Errno::ENOTSOCK,
                        "listen",
                        "file descriptor refers to something other than a socket",
                    );
                }
            }
        //Otherwise, file descriptor is invalid, return with error
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
                return syscall_error(
                    Errno::EINVAL,
                    "netshutdown",
                    "the shutdown how argument passed is not supported",
                );
            }
        }
    }

    pub fn _cleanup_socket_inner_helper(
        sockhandle: &mut SocketHandle,
        how: i32,
        shutdown: bool,
    ) -> i32 {
        // we need to do a bunch of actual socket cleanup for INET sockets
        if sockhandle.domain != AF_UNIX {
            let mut releaseflag = false;
            if let Some(ref sobj) = sockhandle.innersocket {
                if shutdown {
                    let shutresult = sobj.shutdown(how);

                    if shutresult < 0 {
                        match Errno::from_discriminant(interface::get_errno()) {
                            Ok(i) => {
                                return syscall_error(
                                    i,
                                    "shutdown",
                                    "The libc call to setsockopt failed!",
                                );
                            }
                            Err(()) => panic!("Unknown errno value from setsockopt returned!"),
                        };
                    }

                    match how {
                        SHUT_RD => {
                            if sockhandle.state == ConnState::CONNRDONLY {
                                releaseflag = true;
                            }
                        }
                        SHUT_WR => {
                            if sockhandle.state == ConnState::CONNWRONLY {
                                releaseflag = true;
                            }
                        }
                        SHUT_RDWR => {
                            releaseflag = true;
                        }
                        _ => {
                            //See http://linux.die.net/man/2/shutdown for nuance to this error
                            return syscall_error(
                                Errno::EINVAL,
                                "netshutdown",
                                "the shutdown how argument passed is not supported",
                            );
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
                    let release_ret_val = NET_METADATA._release_localport(
                        localaddr.addr(),
                        localaddr.port(),
                        sockhandle.protocol,
                        sockhandle.domain,
                    );
                    sockhandle.localaddr = None;
                    if let Err(e) = release_ret_val {
                        return e;
                    }
                }
            }
        }

        // now change the connections for all socket types
        match how {
            SHUT_RD => {
                if sockhandle.state == ConnState::CONNWRONLY {
                    sockhandle.state = ConnState::NOTCONNECTED;
                } else {
                    sockhandle.state = ConnState::CONNWRONLY;
                }
            }
            SHUT_WR => {
                if sockhandle.state == ConnState::CONNRDONLY {
                    sockhandle.state = ConnState::NOTCONNECTED;
                } else {
                    sockhandle.state = ConnState::CONNRDONLY;
                }
            }
            SHUT_RDWR => {
                sockhandle.state = ConnState::NOTCONNECTED;
            }
            _ => {
                //See http://linux.die.net/man/2/shutdown for nuance to this error
                return syscall_error(
                    Errno::EINVAL,
                    "netshutdown",
                    "the shutdown how argument passed is not supported",
                );
            }
        }

        return 0;
    }

    pub fn _cleanup_socket_inner(
        &self,
        filedesc: &mut FileDescriptor,
        how: i32,
        shutdown: bool,
    ) -> i32 {
        if let Socket(sockfdobj) = filedesc {
            let sock_tmp = sockfdobj.handle.clone();
            let mut sockhandle = sock_tmp.write();

            Self::_cleanup_socket_inner_helper(&mut *sockhandle, how, shutdown)
        } else {
            syscall_error(
                Errno::ENOTSOCK,
                "cleanup socket",
                "file descriptor is not a socket",
            )
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

    /// ### Description
    ///
    /// `accept_syscall` accepts a connection on a socket
    ///
    /// ### Arguments
    ///
    /// it accepts two parameters:
    /// * `fd` - the file descriptor that refers to the listening socket
    /// * `addr` - the address of the incoming connection's socket
    ///
    /// ### Returns
    ///
    /// for a successful call, the return value will be a file descriptor for
    /// the accepted socket (a nonnegative integer). On error, a negative
    /// error number is returned, with the errorno set to represent the
    /// corresponding error
    ///
    /// ### Errors
    ///
    /// ** EAGAIN or EWOULDBLOCK - The socket is marked nonblocking and no
    ///     connections are present to be accepted.  
    ///     POSIX.1-2001 and POSIX.1-2008
    ///     allow either error to be returned for this case, and do
    ///     not require these constants to have the same value, so a
    ///     portable application should check for both possibilities.
    ///
    /// ** EBADF - sockfd is not an open file descriptor.
    ///
    /// * ECONNABORTED - A connection has been aborted.
    ///
    /// * EFAULT - The addr argument is not in a writable part of the user
    ///   address space.
    ///
    /// * EINTR - The system call was interrupted by a signal that was caught
    ///   before a valid connection arrived; see signal(7).
    ///
    /// ** EINVAL - Socket is not listening for connections, or addrlen is
    ///            invalid (e.g., is negative).
    ///
    /// * EMFILE - The per-process limit on the number of open file descriptors
    ///   has been reached.
    ///
    /// * ENFILE - The system-wide limit on the total number of open files has
    ///   been reached.
    ///
    /// * ENOMEM - Not enough free memory.  This often means that the memory
    ///   allocation is limited by the socket buffer limits, not by the system
    ///   memory.
    ///
    /// ** ENOTSOCK - The file descriptor sockfd does not refer to a socket.
    ///
    /// ** EOPNOTSUPP - The referenced socket is not of type SOCK_STREAM.
    ///
    /// * EPERM - Firewall rules forbid connection.
    ///
    /// * EPROTO - Protocol error.
    ///
    /// In addition, network errors for the new socket and as defined for
    /// the protocol may be returned.  Various Linux kernels can return
    /// other errors such as ENOSR, ESOCKTNOSUPPORT, EPROTONOSUPPORT,
    /// ETIMEDOUT.  The value ERESTARTSYS may be seen during a trace.
    ///
    /// ** Indicates the error may be returned from RustPOSIX
    ///
    /// ### Panics
    ///
    /// * invalid or out-of-bounds file descriptor), calling unwrap() on it will
    ///   cause a panic.
    /// * Unknown errno value from fcntl returned, will cause panic.
    ///
    /// for more detailed description of all the commands and return values, see
    /// [accept(2)](https://linux.die.net/man/2/accept)
    pub fn accept_syscall(&self, fd: i32, addr: &mut interface::GenSockaddr) -> i32 {
        //If fd is out of range of [0,MAXFD], process will panic
        //Otherwise, we obtain a write gaurd to the Option<FileDescriptor> object
        let checkedfd = self.get_filedescriptor(fd).unwrap();
        let mut unlocked_fd = checkedfd.write();
        if let Some(filedesc_enum) = &mut *unlocked_fd {
            //Find the next available file descriptor and grab a mutable reference
            //to the Option<FileDescriptor> object
            let (newfd, guardopt) = self.get_next_fd(None);
            //In the case that no fd is available from the call to get_next_fd,
            //fd is set to -ENFILE = -23 and the error is propagated forward
            if newfd < 0 {
                return fd;
            }
            let newfdoption: &mut Option<FileDescriptor> = &mut *guardopt.unwrap();

            //Pattern match such that FileDescriptor object must be the Socket variant
            //Otherwise, return with an err as the fd refers to something other than a
            // socket
            match filedesc_enum {
                Socket(ref mut sockfdobj) => {
                    //Clone the socket handle as it may be in use by other threads and
                    //obtain a read lock, blocking the calling thread until
                    //there are no other writers that hold the lock
                    let sock_tmp = sockfdobj.handle.clone();
                    let mut sockhandle = sock_tmp.read();

                    //Match the domain of the socket to accept the connection
                    match sockhandle.domain {
                        AF_UNIX => {
                            return self.accept_unix(
                                &mut sockhandle,
                                sockfdobj,
                                newfd,
                                newfdoption,
                                addr,
                            )
                        }
                        AF_INET | AF_INET6 => {
                            return self.accept_inet(
                                &mut sockhandle,
                                sockfdobj,
                                newfd,
                                newfdoption,
                                addr,
                            )
                        }
                        _ => {
                            return syscall_error(
                                Errno::EINVAL,
                                "accept",
                                "Unsupported domain provided",
                            )
                        }
                    }
                }
                _ => {
                    return syscall_error(
                        Errno::ENOTSOCK,
                        "accept",
                        "file descriptor refers to something other than a socket",
                    );
                }
            }
        } else {
            return syscall_error(Errno::EBADF, "accept", "invalid file descriptor");
        }
    }

    //The function accepts a connection over the Unix domain
    //
    //Args: sockhandle is a mut reference to a read lock on the SocketHandle of the
    // listening socket      sockfdobj is a mut reference to the Socket
    // Description of the listening socket      newfd is an available file
    // descriptor      newfdoption is a mut reference to a
    // Option<FileDescriptor> object                  at the newfd index in the
    // file descriptor table      addr is the address of the incoming
    // connection's socket
    //
    //upon success return newfd, the new socket file descriptor from the "server
    // side" otherwise, return -errno with errno set to the error
    fn accept_unix(
        &self,
        sockhandle: &mut interface::RustLockReadGuard<SocketHandle>,
        sockfdobj: &mut SocketDesc,
        newfd: i32,
        newfdoption: &mut Option<FileDescriptor>,
        addr: &mut interface::GenSockaddr,
    ) -> i32 {
        match sockhandle.protocol {
            //UDP Sockets do not support listening as UDP is a
            //connectionless based protocol
            IPPROTO_UDP => {
                return syscall_error(
                    Errno::EOPNOTSUPP,
                    "accept",
                    "Protocol does not support listening",
                );
            }
            //TCP Sockets support listening as TCP is a connection
            //based protocol
            IPPROTO_TCP => {
                //Socket must be listening to readily accept a connection
                if sockhandle.state != ConnState::LISTEN {
                    return syscall_error(
                        Errno::EINVAL,
                        "accept",
                        "Socket must be listening before accept is called",
                    );
                }
                //Initialize a new socket file descriptor and set necessary flags
                //based on the listening socket
                let newsockfd = self._socket_initializer(
                    sockhandle.domain,
                    sockhandle.socktype,
                    sockhandle.protocol,
                    sockfdobj.flags & O_NONBLOCK != 0,
                    sockfdobj.flags & O_CLOEXEC != 0,
                    ConnState::CONNECTED,
                );
                //Initialize pipes
                //In the Unix domain, lind emulates communcation
                let remote_addr: interface::GenSockaddr;
                let sendpipenumber;
                let receivepipenumber;
                // We loop here to accept the connection.
                // If we get a connection object from the accept table,
                // we complete the connection and set up the address and pipes.
                // If theres no object, we retry, except in the case of
                // non-blocking accept where we return EAGAIN
                loop {
                    //Normalize the path to the listening socket
                    let localpathbuf =
                        normpath(convpath(sockhandle.localaddr.unwrap().path()), self);
                    //Note, NET_METADATA.domsock_accept_table stores pending
                    //connections (from client calling `connect`)
                    //Retrieve one of the pending connections if it exists
                    let dsconnobj = NET_METADATA.domsock_accept_table.get(&localpathbuf);

                    //Check if a pending connection exists
                    if let Some(ds) = dsconnobj {
                        //Pattern match to retrieve the connvar
                        if let Some(connvar) = ds.get_cond_var() {
                            //If the incoming connection's socket is not waiting, drop the
                            //connection loop to the next one
                            if !connvar.broadcast() {
                                drop(ds);
                                continue;
                            }
                        }
                        //Grab the incoming connection's address, receive pipe, and send pipe,
                        //and then remove the incoming connection's socket from pending connections
                        let addr = ds.get_sockaddr().clone();
                        remote_addr = addr.clone();
                        receivepipenumber = ds.get_receive_pipe().clone();
                        sendpipenumber = ds.get_send_pipe().clone();
                        drop(ds);
                        NET_METADATA.domsock_accept_table.remove(&localpathbuf);
                        break;
                    } else {
                        //The listening socket is marked nonblocking and no
                        //connections are present to be accepted
                        if 0 != (sockfdobj.flags & O_NONBLOCK) {
                            return syscall_error(
                                Errno::EAGAIN,
                                "accept",
                                "host system accept call failed",
                            );
                        }
                    }
                }

                //Gain write access to the socket handle to insert
                //info regarding the unix connection
                let newsock_tmp = newsockfd.handle.clone();
                let mut newsockhandle = newsock_tmp.write();

                //Retrieve the inodenum of the incoming connection's socket
                let pathclone = normpath(convpath(remote_addr.path()), self);
                if let Some(inodenum) = metawalk(pathclone.as_path()) {
                    //Insert necessary info about the socket communication
                    newsockhandle.unix_info = Some(UnixSocketInfo {
                        inode: inodenum.clone(),
                        mode: sockhandle.unix_info.as_ref().unwrap().mode,
                        sendpipe: Some(sendpipenumber.clone()),
                        receivepipe: Some(receivepipenumber.clone()),
                    });
                    //Grab the incoming connection's socket inode from the inodetable
                    //and increase the refcount by 1, as the socket is accepting
                    //a connection. Thus, we do not want the socket to be closed
                    //before the connection ends.
                    if let Inode::Socket(ref mut sock) =
                        *(FS_METADATA.inodetable.get_mut(&inodenum).unwrap())
                    {
                        sock.refcount += 1;
                    }
                };

                //Finalize values for the new "server" socket handle that was
                //created to connect with the incoming connection's socket
                newsockhandle.localaddr = Some(sockhandle.localaddr.unwrap().clone());
                newsockhandle.remoteaddr = Some(remote_addr.clone());
                newsockhandle.state = ConnState::CONNECTED;

                //Insert the socket FileDescriptor object into the
                //file descriptor table
                let _insertval = newfdoption.insert(Socket(newsockfd));
                *addr = remote_addr; //populate addr with what address it connected to

                return newfd;
            }
            //Socket Protocol is not UDP nor TCP, therefore unsupported by lind
            _ => {
                return syscall_error(Errno::EOPNOTSUPP, "accept", "Unkown protocol in accept");
            }
        }
    }

    //The function accepts a connection over the INET domain
    //
    //Args: sockhandle is a mut reference to a read lock on the SocketHandle of the
    // listening socket      sockfdobj is a mut reference to the Socket
    // Description of the listening socket      newfd is an available file
    // descriptor      newfdoption is a mut reference to a
    // Option<FileDescriptor> object                  at the newfd index in the
    // file descriptor table      addr is the address of the incoming
    // connection's socket
    //
    //upon success return newfd, the new socket file descriptor from the "server
    // side" otherwise, return -errno with errno set to the error
    fn accept_inet(
        &self,
        sockhandle: &mut interface::RustLockReadGuard<SocketHandle>,
        sockfdobj: &mut SocketDesc,
        newfd: i32,
        newfdoption: &mut Option<FileDescriptor>,
        addr: &mut interface::GenSockaddr,
    ) -> i32 {
        match sockhandle.protocol {
            //UDP sockets do not support listening as UDP is a connectionless
            //based protocol
            IPPROTO_UDP => {
                return syscall_error(
                    Errno::EOPNOTSUPP,
                    "accept",
                    "Protocol does not support listening",
                );
            }
            //TCP Sockets support listening as TCP is a connection
            //based protocol
            IPPROTO_TCP => {
                //Socket must be listening to readily accept a connection
                if sockhandle.state != ConnState::LISTEN {
                    return syscall_error(
                        Errno::EINVAL,
                        "accept",
                        "Socket must be listening before accept is called",
                    );
                }
                //Initialize a new socket file descriptor and set necessary flags
                //based on the listening socket
                let mut newsockfd = self._socket_initializer(
                    sockhandle.domain,
                    sockhandle.socktype,
                    sockhandle.protocol,
                    sockfdobj.flags & O_NONBLOCK != 0,
                    sockfdobj.flags & O_CLOEXEC != 0,
                    ConnState::CONNECTED,
                );

                //we loop here so we can cancel blocking accept,
                //see comments below and in Socket::new in interface/comm.rs
                loop {
                    // if we got a pending connection in select/poll/whatever, return that here
                    // instead

                    //Socket must have been populated by implicit bind
                    let ladr = sockhandle.localaddr.unwrap().clone();
                    //Obtain a tuple of the address, port, and port type of the listening socket
                    //Panics if domain is not INET or INET6
                    let porttuple =
                        mux_port(ladr.addr().clone(), ladr.port(), sockhandle.domain, TCPPORT);

                    //Check if there are any pending incoming connections to the listening socket
                    //and grab the incoming connection's socket raw fd and its address
                    let mut pendingvec =
                        NET_METADATA.pending_conn_table.get_mut(&porttuple).unwrap();
                    let pendingoption = pendingvec.pop();
                    let (acceptedresult, remote_addr) = match pendingoption {
                        Some(pendingtup) => pendingtup,
                        None => {
                            //If the socket is blocking, call the accept syscall
                            //from libc
                            if 0 == (sockfdobj.flags & O_NONBLOCK) {
                                match sockhandle.domain {
                                    PF_INET => {
                                        sockhandle.innersocket.as_ref().unwrap().accept(true)
                                    }
                                    PF_INET6 => {
                                        sockhandle.innersocket.as_ref().unwrap().accept(false)
                                    }
                                    _ => panic!("Unknown domain in accepting socket"),
                                }
                            //otherwise the socket is nonblocking so call the
                            // nonblocking accept syscall from libc and
                            // set the the raw sys fd of the listening socket to
                            // nonblocking
                            } else {
                                match sockhandle.domain {
                                    PF_INET => sockhandle
                                        .innersocket
                                        .as_ref()
                                        .unwrap()
                                        .nonblock_accept(true),
                                    PF_INET6 => sockhandle
                                        .innersocket
                                        .as_ref()
                                        .unwrap()
                                        .nonblock_accept(false),
                                    _ => panic!("Unknown domain in accepting socket"),
                                }
                            }
                        }
                    };

                    //If the accept libc call returns with an error
                    if let Err(_) = acceptedresult {
                        match Errno::from_discriminant(interface::get_errno()) {
                            Ok(i) => {
                                //We have the socket timeout set to every one second, so
                                //if our blocking socket ever returns EAGAIN, it must be
                                //the case that this timeout was exceeded, and we
                                //should thus not treat this as a failure in our emulated
                                //socket; see comment in Socket::new in interface/comm.rs
                                if sockfdobj.flags & O_NONBLOCK == 0 && i == Errno::EAGAIN {
                                    // if the cancel status is set in the cage, we trap around a
                                    // cancel point
                                    // until the individual thread is signaled to kill itself
                                    if self
                                        .cancelstatus
                                        .load(interface::RustAtomicOrdering::Relaxed)
                                    {
                                        loop {
                                            interface::cancelpoint(self.cageid);
                                        }
                                    }
                                    continue;
                                }

                                return syscall_error(
                                    i,
                                    "accept",
                                    "Internal call to accept failed",
                                );
                            }
                            Err(()) => panic!("Unknown errno value from socket accept returned!"),
                        };
                    }

                    //If we get here we have an accepted socket
                    let acceptedsock = acceptedresult.unwrap();
                    //Set the address and the port of the new socket
                    //created to handle the connection to the incoming connection's socket
                    let mut newaddr = sockhandle.localaddr.unwrap().clone();
                    let newport = match NET_METADATA._reserve_localport(
                        newaddr.addr(),
                        0,
                        sockhandle.protocol,
                        sockhandle.domain,
                        false,
                    ) {
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

                    //create Socket object for new connected socket
                    newsockhandle.innersocket = Some(acceptedsock);
                    //set lock-free rawfd for select
                    newsockfd.rawfd = newsockhandle.innersocket.as_ref().unwrap().raw_sys_fd;

                    let _insertval = newfdoption.insert(Socket(newsockfd));
                    *addr = remote_addr; //populate addr with the address of the incoming connection's socket

                    return newfd;
                }
            }
            _ => {
                return syscall_error(Errno::EOPNOTSUPP, "accept", "Unkown protocol in accept");
            }
        }
    }

    /// ## ------------------SELECT SYSCALL------------------
    /// ### Description
    /// The `select_syscall()` allows a program to monitor multiple file
    /// descriptors, waiting until one or more of the file descriptors become
    /// "ready" for some class of I/O operation (e.g., input possible).  A
    /// file descriptor is considered ready if it is possible to perform a
    /// corresponding I/O operation (e.g., `read_syscall()`) without blocking.

    /// ### Function Arguments
    /// The `select_syscall()` receives five arguments:
    /// * `nfds` - This argument should be set to the highest-numbered file
    ///   descriptor in any of the three sets, plus 1.  The indicated file
    ///   descriptors in each set are checked, up to this limit.
    /// * `readfds` -  The file descriptors in this set are watched to see if
    ///   they are ready for reading.  A file descriptor is ready for reading if
    ///   a read operation will not block; in particular, a file descriptor is
    ///   also ready on end-of-file. After select() has returned, readfds will
    ///   be cleared of all file descriptors except for those that are ready for
    ///   reading.
    /// * `writefds` - The file descriptors in this set are watched to see if
    ///   they are ready for writing.  A file descriptor is ready for writing if
    ///   a write operation will not block.  However, even if a file descriptor
    ///   indicates as writable, a large write may still block. After select()
    ///   has returned, writefds will be cleared of all file descriptors except
    ///   for those that are ready for writing.
    /// * `exceptfds` - currently not supported, only the validity of the fds
    ///   will be checked
    /// * `timeout` - The timeout argument is a RustDuration structure that
    ///   specifies the interval that select() should block waiting for a file
    ///   descriptor to become ready.  The call will block until either: •  a
    ///   file descriptor becomes ready; •  the call is interrupted by a signal
    ///   handler; or •  the timeout expires.

    /// ### Returns
    /// On success, select() and return the number of file descriptors contained
    /// in the two returned descriptor sets (that is, the total number of
    /// bits that are set in readfds, writefds). The return value may be zero if
    /// the timeout expired before any file descriptors became ready.
    /// Otherwise, errors or panics are returned for different scenarios.
    ///
    /// ### Errors
    /// * EBADF - An invalid file descriptor was given in one of the sets. (e.g.
    ///   a file descriptor that was already closed.)
    /// * EINTR - A signal was caught.
    /// * EINVAL -  nfds is negative or exceeds the FD_SET_MAX_FD.
    ///
    /// ### Panics
    /// No panic is expected from this syscall
    pub fn select_syscall(
        &self,
        nfds: i32,
        readfds: Option<&mut interface::FdSet>,
        writefds: Option<&mut interface::FdSet>,
        exceptfds: Option<&mut interface::FdSet>,
        timeout: Option<interface::RustDuration>,
    ) -> i32 {
        // nfds should be in the allowed range (i.e. 0 to 1024 according to standard)
        if nfds < STARTINGFD || nfds >= FD_SET_MAX_FD {
            return syscall_error(Errno::EINVAL, "select", "Number of FDs is wrong");
        }

        let start_time = interface::starttimer();

        let end_time = match timeout {
            Some(time) => time,
            None => interface::RustDuration::MAX,
        };

        let mut retval = 0;
        // in the loop below, we always read from original fd_sets, but make updates to
        // the new copies
        let new_readfds = &mut interface::FdSet::new();
        let new_writefds = &mut interface::FdSet::new();
        loop {
            //we must block manually
            // 1. iterate thru readfds
            if let Some(readfds_ref) = readfds.as_ref() {
                let res = self.select_readfds(nfds, readfds_ref, new_readfds, &mut retval);
                if res != 0 {
                    return res;
                }
            }

            // 2. iterate thru writefds
            if let Some(writefds_ref) = writefds.as_ref() {
                let res = self.select_writefds(nfds, writefds_ref, new_writefds, &mut retval);
                if res != 0 {
                    return res;
                }
            }

            // 3. iterate thru exceptfds
            // TODO: we currently don't implement exceptfds but possibly could if necessary
            if let Some(exceptfds_ref) = exceptfds.as_ref() {
                for fd in 0..nfds {
                    // find the bit and see if it's on
                    if !exceptfds_ref.is_set(fd) {
                        continue;
                    }
                    let checkedfd = self.get_filedescriptor(fd).unwrap();
                    let unlocked_fd = checkedfd.read();
                    if unlocked_fd.is_none() {
                        return syscall_error(Errno::EBADF, "select", "invalid file descriptor");
                    }
                }
            }

            // check for timeout
            if retval != 0 || interface::readtimer(start_time) > end_time {
                break;
            } else {
                // at this point lets check if we got a signal before sleeping
                if interface::sigcheck() {
                    return syscall_error(Errno::EINTR, "select", "interrupted function call");
                }
                interface::lind_yield();
            }
        }

        // Now we copy our internal FdSet struct results back into the *mut libc::fd_set
        if readfds.is_some() {
            readfds.unwrap().copy_from(&new_readfds);
        }

        if writefds.is_some() {
            writefds.unwrap().copy_from(&new_writefds);
        }

        return retval;
    }

    /// This function is used to select on readfds specifically
    /// This function monitors all readfds to check if they are ready to read
    /// readfds could be one of the followings:
    /// 1. Regular files (these files are always marked as readable)
    /// 2. Pipes
    /// 3. Sockets
    fn select_readfds(
        &self,
        nfds: i32,
        readfds: &interface::FdSet,
        new_readfds: &mut interface::FdSet,
        retval: &mut i32,
    ) -> i32 {
        // For INET: prepare the data structures for the kernel_select's use
        let mut inet_info = SelectInetInfo::new();

        for fd in 0..nfds {
            // ignore file descriptors that are not in the set
            if !readfds.is_set(fd) {
                continue;
            }

            // try to get the FileDescriptor Object from fd number
            // if the fd exists, do further processing based on the file descriptor type
            // otherwise, raise an error
            let checkedfd = self.get_filedescriptor(fd).unwrap();
            let unlocked_fd = checkedfd.read();
            if let Some(filedesc_enum) = &*unlocked_fd {
                match filedesc_enum {
                    Socket(ref sockfdobj) => {
                        let mut newconnection = false;
                        match sockfdobj.domain {
                            AF_UNIX => {
                                // sockethandle lock with read access
                                let sock_tmp = sockfdobj.handle.clone();
                                let sockhandle = sock_tmp.read();
                                if sockhandle.state == ConnState::INPROGRESS {
                                    // if connection state is INPROGRESS, in case of AF_UNIX socket,
                                    // that would mean the socket connects in non-blocking mode

                                    // BUG: current implementation of AF_UNIX socket non-blocking
                                    // connection is not working
                                    // correctly, according to standards, when the connection
                                    // is ready, select should report writability instead of
                                    // readability so the code
                                    // here should be removed. Interestingly, since connect_tcp_unix
                                    // hasn't changed the state to INPROGRESS, so this piece of code
                                    // inside the if statement
                                    // is a dead code that would never be executed currently
                                    let remotepathbuf = normpath(
                                        convpath(sockhandle.remoteaddr.unwrap().path()),
                                        self,
                                    );
                                    let dsconnobj =
                                        NET_METADATA.domsock_accept_table.get(&remotepathbuf);
                                    if dsconnobj.is_none() {
                                        newconnection = true;
                                    }
                                }

                                if sockhandle.state == ConnState::LISTEN {
                                    // if connection state is LISTEN
                                    // then check if there are any pending connections

                                    // get the path of the socket
                                    let localpathbuf = normpath(
                                        convpath(sockhandle.localaddr.unwrap().path()),
                                        self,
                                    );
                                    // check if there is any connections associated with the path
                                    let dsconnobj =
                                        NET_METADATA.domsock_accept_table.get(&localpathbuf);
                                    if dsconnobj.is_some() {
                                        // we have a connecting domain socket, return as readable to
                                        // be accepted
                                        new_readfds.set(fd);
                                        *retval += 1;
                                    }
                                } else if sockhandle.state == ConnState::CONNECTED || newconnection
                                {
                                    // otherwise, the connection is already established
                                    // check if the pipe has any thing
                                    let sockinfo = &sockhandle.unix_info.as_ref().unwrap();
                                    let receivepipe = sockinfo.receivepipe.as_ref().unwrap();
                                    if receivepipe.check_select_read() {
                                        new_readfds.set(fd);
                                        *retval += 1;
                                    }
                                }
                            }
                            AF_INET | AF_INET6 => {
                                // For AF_INET or AF_INET6 socket, currently we still rely on kernel
                                // implementation, so here we simply
                                // prepare the kernel fd set by translating fd into kernel fd
                                // and will pass it to kernel_select later
                                if sockfdobj.rawfd < 0 {
                                    continue;
                                }

                                inet_info.kernel_fds.set(sockfdobj.rawfd);
                                inet_info.rawfd_lindfd_tuples.push((sockfdobj.rawfd, fd));
                                if sockfdobj.rawfd > inet_info.highest_raw_fd {
                                    inet_info.highest_raw_fd = sockfdobj.rawfd;
                                }
                            }
                            _ => {
                                return syscall_error(
                                    Errno::EINVAL,
                                    "select",
                                    "Unsupported domain provided",
                                )
                            }
                        }

                        // newconnection seems to be used for AF_UNIX socket with INPROGRESS state
                        // (non-blocking AF_UNIX socket connection), which is a broken feature
                        // currently
                        if newconnection {
                            let sock_tmp = sockfdobj.handle.clone();
                            let mut sockhandle = sock_tmp.write();
                            sockhandle.state = ConnState::CONNECTED;
                        }
                    }

                    // we don't support selecting streams
                    Stream(_) => {
                        continue;
                    }

                    Pipe(pipefdobj) => {
                        // check if the pipe has anything to read
                        if pipefdobj.pipe.check_select_read() {
                            new_readfds.set(fd);
                            *retval += 1;
                        }
                    }

                    // these file reads never block
                    _ => {
                        new_readfds.set(fd);
                        *retval += 1;
                    }
                }
            } else {
                return syscall_error(Errno::EBADF, "select", "invalid file descriptor");
            }
        }

        // if kernel_fds is not empty, that would mean we will need to call the
        // kernel_select (which is calling real select syscall under the hood)
        // for these fds for AF_INET/AF_INET6 sockets
        if !inet_info.kernel_fds.is_empty() {
            let kernel_ret = update_readfds_from_kernel_select(new_readfds, &mut inet_info, retval);
            // NOTE: we ignore the kernel_select error if some domsocks are ready
            if kernel_ret < 0 && *retval <= 0 {
                return kernel_ret;
            }
        }

        return 0;
    }

    /// This function is used to select on writefds specifically
    /// This function monitors all writefds to check if they are ready to write
    /// writefds could be one of the followings:
    /// 1. Regular files (these files are always marked as writable)
    /// 2. Pipes
    /// 3. Sockets
    fn select_writefds(
        &self,
        nfds: i32,
        writefds: &interface::FdSet,
        new_writefds: &mut interface::FdSet,
        retval: &mut i32,
    ) -> i32 {
        for fd in 0..nfds {
            // ignore file descriptors that are not in the set
            if !writefds.is_set(fd) {
                continue;
            }

            // try to get the FileDescriptor Object from fd number
            // if the fd exists, do further processing based on the file descriptor type
            // otherwise, raise an error
            let checkedfd = self.get_filedescriptor(fd).unwrap();
            let unlocked_fd = checkedfd.read();
            if let Some(filedesc_enum) = &*unlocked_fd {
                match filedesc_enum {
                    Socket(ref sockfdobj) => {
                        // sockethandle lock with read access
                        let sock_tmp = sockfdobj.handle.clone();
                        let sockhandle = sock_tmp.read();
                        let mut newconnection = false;
                        match sockhandle.domain {
                            AF_UNIX => {
                                if sockhandle.state == ConnState::INPROGRESS {
                                    // if connection state is INPROGRESS, in case of AF_UNIX socket,
                                    // that would mean the socket connects in non-blocking mode

                                    // BUG: current implementation of AF_UNIX socket non-blocking
                                    // connection is not working
                                    // correctly, according to standards, when the connection
                                    // is ready, select should report for writability, but current
                                    // implementation
                                    // does not make much sense
                                    let remotepathbuf =
                                        convpath(sockhandle.remoteaddr.unwrap().path());
                                    let dsconnobj =
                                        NET_METADATA.domsock_accept_table.get(&remotepathbuf);
                                    if dsconnobj.is_none() {
                                        newconnection = true;
                                    }
                                }
                                // BUG: need to check if send_pipe is ready to
                                // write
                            }
                            AF_INET | AF_INET6 => {
                                // For AF_INET or AF_INET6 socket, currently we still rely on kernel
                                // implementation, so here we simply
                                // call check_rawconnection with innersocket if connection state
                                // is INPROGRESS (non-blocking AF_INET/AF_INET6 socket connection)
                                if sockhandle.state == ConnState::INPROGRESS
                                    && sockhandle
                                        .innersocket
                                        .as_ref()
                                        .unwrap()
                                        .check_rawconnection()
                                {
                                    newconnection = true;
                                }
                            }
                            _ => {
                                return syscall_error(Errno::EINVAL, "select", "Unsupported domain")
                            }
                        }

                        // non-blocking AF_INET/AF_INET6 socket connection now established
                        // change the state to connected
                        if newconnection {
                            let mut newconnhandle = sock_tmp.write();
                            newconnhandle.state = ConnState::CONNECTED;
                        }

                        // BUG: socket are not always writable, it could block in cases
                        // like the kernel send buffer is full
                        new_writefds.set(fd);
                        *retval += 1;
                    }

                    // we always say streams are writable?
                    Stream(_) => {
                        new_writefds.set(fd);
                        *retval += 1;
                    }

                    Pipe(pipefdobj) => {
                        // check if the pipe has any space to write
                        if pipefdobj.pipe.check_select_write() {
                            new_writefds.set(fd);
                            *retval += 1;
                        }
                    }

                    // these file writes never block
                    _ => {
                        new_writefds.set(fd);
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
                let optbit = 1 << optname;
                let sock_tmp = sockfdobj.handle.clone();
                let mut sockhandle = sock_tmp.write();
                match level {
                    SOL_UDP => {
                        return syscall_error(
                            Errno::EOPNOTSUPP,
                            "getsockopt",
                            "UDP is not supported for getsockopt",
                        );
                    }
                    SOL_TCP => {
                        // Checking the tcp_options here
                        // Currently only support TCP_NODELAY option for SOL_TCP
                        if optname == TCP_NODELAY {
                            let optbit = 1 << optname;
                            if optbit & sockhandle.tcp_options == optbit {
                                *optval = 1;
                            } else {
                                *optval = 0;
                            }
                            return 0;
                        }
                        return syscall_error(
                            Errno::EOPNOTSUPP,
                            "getsockopt",
                            "TCP options not remembered by getsockopt",
                        );
                    }
                    SOL_SOCKET => {
                        // checking the socket_options here
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
                            SO_LINGER | SO_KEEPALIVE | SO_SNDLOWAT | SO_RCVLOWAT | SO_REUSEPORT
                            | SO_REUSEADDR => {
                                if sockhandle.socket_options & optbit == optbit {
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
                                return syscall_error(
                                    Errno::EOPNOTSUPP,
                                    "getsockopt",
                                    "unknown optname passed into syscall",
                                );
                            }
                        }
                    }
                    _ => {
                        return syscall_error(
                            Errno::EOPNOTSUPP,
                            "getsockopt",
                            "unknown level passed into syscall",
                        );
                    }
                }
            } else {
                return syscall_error(
                    Errno::ENOTSOCK,
                    "getsockopt",
                    "the provided file descriptor is not a socket",
                );
            }
        } else {
            return syscall_error(
                Errno::EBADF,
                "getsockopt",
                "the provided file descriptor is invalid",
            );
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
                        return syscall_error(
                            Errno::EOPNOTSUPP,
                            "setsockopt",
                            "UDP is not supported for getsockopt",
                        );
                    }
                    SOL_TCP => {
                        // Here we check and set tcp_options
                        // Currently only support TCP_NODELAY for SOL_TCP
                        if optname == TCP_NODELAY {
                            let optbit = 1 << optname;
                            let sock_tmp = sockfdobj.handle.clone();
                            let mut sockhandle = sock_tmp.write();
                            let mut newoptions = sockhandle.tcp_options;
                            //now let's set this if we were told to
                            if optval != 0 {
                                //optval should always be 1 or 0.
                                newoptions |= optbit;
                            } else {
                                newoptions &= !optbit;
                            }

                            if newoptions != sockhandle.tcp_options {
                                if let Some(sock) = sockhandle.innersocket.as_ref() {
                                    let sockret = sock.setsockopt(SOL_TCP, optname, optval);
                                    if sockret < 0 {
                                        match Errno::from_discriminant(interface::get_errno()) {
                                            Ok(i) => {
                                                return syscall_error(
                                                    i,
                                                    "setsockopt",
                                                    "The libc call to setsockopt failed!",
                                                );
                                            }
                                            Err(()) => panic!(
                                                "Unknown errno value from setsockopt returned!"
                                            ),
                                        };
                                    }
                                }
                            }
                            sockhandle.tcp_options = newoptions;
                            return 0;
                        }
                        return syscall_error(
                            Errno::EOPNOTSUPP,
                            "setsockopt",
                            "This TCP option is not remembered by setsockopt",
                        );
                    }
                    SOL_SOCKET => {
                        // Here we check and set socket_options
                        let optbit = 1 << optname;
                        let sock_tmp = sockfdobj.handle.clone();
                        let mut sockhandle = sock_tmp.write();

                        match optname {
                            SO_ACCEPTCONN | SO_TYPE | SO_SNDLOWAT | SO_RCVLOWAT => {
                                let error_string =
                                    format!("Cannot set option using setsockopt. {}", optname);
                                return syscall_error(
                                    Errno::ENOPROTOOPT,
                                    "setsockopt",
                                    &error_string,
                                );
                            }
                            SO_LINGER | SO_KEEPALIVE => {
                                if optval == 0 {
                                    sockhandle.socket_options &= !optbit;
                                } else {
                                    //optval should always be 1 or 0.
                                    sockhandle.socket_options |= optbit;
                                }

                                return 0;
                            }

                            SO_REUSEPORT | SO_REUSEADDR => {
                                let mut newoptions = sockhandle.socket_options;
                                //now let's set this if we were told to
                                if optval != 0 {
                                    //optval should always be 1 or 0.
                                    newoptions |= optbit;
                                } else {
                                    newoptions &= !optbit;
                                }

                                if newoptions != sockhandle.socket_options {
                                    if let Some(sock) = sockhandle.innersocket.as_ref() {
                                        let sockret = sock.setsockopt(SOL_SOCKET, optname, optval);
                                        if sockret < 0 {
                                            match Errno::from_discriminant(interface::get_errno()) {
                                                Ok(i) => {
                                                    return syscall_error(
                                                        i,
                                                        "setsockopt",
                                                        "The libc call to setsockopt failed!",
                                                    );
                                                }
                                                Err(()) => panic!(
                                                    "Unknown errno value from setsockopt returned!"
                                                ),
                                            };
                                        }
                                    }
                                }

                                sockhandle.socket_options = newoptions;

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
                                    return syscall_error(
                                        Errno::EOPNOTSUPP,
                                        "getsockopt",
                                        "does not support OOBINLINE being set to anything but 1",
                                    );
                                }
                                return 0;
                            }
                            _ => {
                                return syscall_error(
                                    Errno::EOPNOTSUPP,
                                    "getsockopt",
                                    "unknown optname passed into syscall",
                                );
                            }
                        }
                    }
                    _ => {
                        return syscall_error(
                            Errno::EOPNOTSUPP,
                            "getsockopt",
                            "unknown level passed into syscall",
                        );
                    }
                }
            } else {
                return syscall_error(
                    Errno::ENOTSOCK,
                    "getsockopt",
                    "the provided file descriptor is not a socket",
                );
            }
        } else {
            return syscall_error(
                Errno::EBADF,
                "getsockopt",
                "the provided file descriptor is invalid",
            );
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
                    return syscall_error(
                        Errno::ENOTCONN,
                        "getpeername",
                        "the socket is not connected",
                    );
                }
                *ret_addr = sockhandle.remoteaddr.unwrap();
                return 0;
            } else {
                return syscall_error(
                    Errno::ENOTSOCK,
                    "getpeername",
                    "the provided file is not a socket",
                );
            }
        } else {
            return syscall_error(
                Errno::EBADF,
                "getpeername",
                "the provided file descriptor is not valid",
            );
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
                        *ret_addr = interface::GenSockaddr::Unix(interface::new_sockaddr_unix(
                            sockhandle.domain as u16,
                            null_path,
                        ));
                        return 0;
                    }
                    //if the socket is not none, then return the socket
                    *ret_addr = sockhandle.localaddr.unwrap();
                    return 0;
                } else {
                    if sockhandle.localaddr == None {
                        //sets the address to 0.0.0.0 if the address is not initialized yet
                        //setting the family as well based on the domain
                        let addr = match sockhandle.domain {
                            AF_INET => interface::GenIpaddr::V4(interface::V4Addr::default()),
                            AF_INET6 => interface::GenIpaddr::V6(interface::V6Addr::default()),
                            _ => {
                                unreachable!()
                            }
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
                return syscall_error(
                    Errno::ENOTSOCK,
                    "getsockname",
                    "the provided file is not a socket",
                );
            }
        } else {
            return syscall_error(
                Errno::EBADF,
                "getsockname",
                "the provided file descriptor is not valid",
            );
        }
    }

    //we only return the default host name because we do not allow for the user to
    // change the host name right now
    pub fn gethostname_syscall(&self, address_ptr: *mut u8, length: isize) -> i32 {
        if length < 0 {
            return syscall_error(
                Errno::EINVAL,
                "gethostname_syscall",
                "provided length argument is invalid",
            );
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

    /// ## ------------------POLL SYSCALL------------------
    /// ### Description
    /// poll_syscall performs a similar task to select_syscall: it waits for
    /// one of a set of file descriptors to become ready to perform I/O.

    /// ### Function Arguments
    /// The `poll_syscall()` receives two arguments:
    /// * `fds` - The set of file descriptors to be monitored is specified in
    ///   the fds argument, which is an array of PollStruct structures
    ///   containing three fields: fd, events and revents. events and revents
    ///   are requested events and returned events, respectively. The field fd
    ///   contains a file descriptor for an open file. If this field is
    ///   negative, then the corresponding events field is ignored and the
    ///   revents field returns zero. The field events is an input parameter, a
    ///   bit mask specifying the events the application is interested in for
    ///   the file descriptor fd. The bits returned in revents can include any
    ///   of those specified in events, or POLLNVAL. The bits that may be
    ///   set/returned in events and revents are: 1. POLLIN: There is data to
    ///   read. 2. POLLPRI: There is some exceptional condition on the file
    ///   descriptor, currently not supported 3. POLLOUT: Writing is now
    ///   possible, though a write larger than the available space in a socket
    ///   or pipe will still block
    ///   4. POLLNVAL: Invalid request: fd not open (only returned in revents;
    ///   ignored in events).
    /// * `timeout` - The timeout argument is a RustDuration structure that
    ///   specifies the interval that poll() should block waiting for a file
    ///   descriptor to become ready. The call will block until either: 1.  a
    ///   file descriptor becomes ready; 2. the call is interrupted by a signal
    ///   handler; 3. the timeout expires.

    /// ### Returns
    /// On success, poll_syscall returns a nonnegative value which is the
    /// number of elements in the pollfds whose revents fields have been
    /// set to a nonzero value (indicating an event or an error). A
    /// return value of zero indicates that the system call timed out
    /// before any file descriptors became ready.
    ///
    /// ### Errors
    /// * EINTR - A signal was caught.
    /// * EINVAL - fd exceeds the FD_SET_MAX_FD.
    ///
    /// ### Panics
    /// No panic is expected from this syscall
    pub fn poll_syscall(
        &self,
        fds: &mut [PollStruct],
        timeout: Option<interface::RustDuration>,
    ) -> i32 {
        // timeout is supposed to be in milliseconds

        // current implementation of poll_syscall is based on select_syscall
        // which gives several issues:
        // 1. according to standards, select_syscall should only support file descriptor
        //    that is smaller than 1024, while poll_syscall should not have such
        //    limitation but our implementation of poll_syscall is actually calling
        //    select_syscall directly which would mean poll_syscall would also have the
        //    1024 maximum size limitation However, rustposix itself only support file
        //    descriptor that is smaller than 1024 which solves this issue automatically
        //    in an interesting way
        // 2. current implementation of poll_syscall is very inefficient, that it passes
        //    each of the file descriptor into select_syscall one by one. A better
        //    solution might be transforming pollstruct into fdsets and pass into
        //    select_syscall once (TODO). A even more efficienct way would be completely
        //    rewriting poll_syscall so it does not depend on select_syscall anymore.
        //    This is also how Linux does for poll_syscall since Linux claims that poll
        //    have a better performance than select.
        // 3. several revent value such as POLLERR (which should be set when pipe is
        //    broken), or POLLHUP (when peer closed its channel) are not possible to
        //    monitor. Since select_syscall does not have these features, so our
        //    poll_syscall, which derived from select_syscall, would subsequently not be
        //    able to support these features.

        let mut return_code: i32 = 0;
        let start_time = interface::starttimer();

        let end_time = match timeout {
            Some(time) => time,
            None => interface::RustDuration::MAX,
        };

        // according to standard, we should clear all revents
        for structpoll in &mut *fds {
            structpoll.revents = 0;
        }

        // we loop until either timeout
        // or any of the file descriptor is ready
        loop {
            // iterate through each file descriptor
            for structpoll in &mut *fds {
                // get the file descriptor
                let fd = structpoll.fd;

                // according to standard, we should ignore all file descriptor
                // that is smaller than 0
                if fd < 0 {
                    continue;
                }

                // get the associated events to monitor
                let events = structpoll.events;

                // init FdSet structures
                let reads = &mut interface::FdSet::new();
                let writes = &mut interface::FdSet::new();
                let errors = &mut interface::FdSet::new();

                // POLLIN for readable fd
                if events & POLLIN > 0 {
                    reads.set(fd)
                }
                // POLLOUT for writable fd
                if events & POLLOUT > 0 {
                    writes.set(fd)
                }
                // POLLPRI for except fd
                if events & POLLPRI > 0 {
                    errors.set(fd)
                }

                // this mask is used for storing final revent result
                let mut mask: i16 = 0;

                // here we just call select_syscall with timeout of zero,
                // which essentially just check each fd set once then return
                // NOTE that the nfds argument is highest fd + 1
                let selectret = Self::select_syscall(
                    &self,
                    fd + 1,
                    Some(reads),
                    Some(writes),
                    Some(errors),
                    Some(interface::RustDuration::ZERO),
                );
                // if there is any file descriptor ready
                if selectret > 0 {
                    // is the file descriptor ready to read?
                    mask |= if !reads.is_empty() { POLLIN } else { 0 };
                    // is the file descriptor ready to write?
                    mask |= if !writes.is_empty() { POLLOUT } else { 0 };
                    // is there any exception conditions on the file descriptor?
                    mask |= if !errors.is_empty() { POLLPRI } else { 0 };
                    // this file descriptor is ready for something,
                    // increment the return value
                    return_code += 1;
                } else if selectret < 0 {
                    // if there is any error, first check if the error
                    // is EBADF, which refers to invalid file descriptor error
                    // in this case, we should set POLLNVAL to revent
                    if selectret == -(Errno::EBADF as i32) {
                        mask |= POLLNVAL;
                        // according to standard, return value is the number of fds
                        // with non-zero revent, which may indicate an error as well
                        return_code += 1;
                    } else {
                        return selectret;
                    }
                }
                // set the revents
                structpoll.revents = mask;
            }

            // we break if there is any file descriptor ready
            // or timeout is reached
            if return_code != 0 || interface::readtimer(start_time) > end_time {
                break;
            } else {
                // otherwise, check for signal and loop again
                if interface::sigcheck() {
                    return syscall_error(Errno::EINTR, "poll", "interrupted function call");
                }
                // We yield to let other threads continue if we've found no ready descriptors
                interface::lind_yield();
            }
        }
        return return_code;
    }

    pub fn _epoll_object_allocator(&self) -> i32 {
        // create a Epoll file descriptor
        let epollobjfd = Epoll(EpollDesc {
            mode: 0000,
            registered_fds: interface::RustHashMap::<i32, EpollEvent>::new(),
            advlock: interface::RustRfc::new(interface::AdvisoryLock::new()),
            errno: 0,
            flags: 0,
        });
        // get a file descriptor
        let (fd, guardopt) = self.get_next_fd(None);
        if fd < 0 {
            return fd;
        }
        let fdoption = &mut *guardopt.unwrap();
        let _insertval = fdoption.insert(epollobjfd);

        return fd;
    }

    /// ## ------------------EPOLL_CREATE SYSCALL------------------
    /// ### Description
    /// epoll_create_syscall creates a new epoll instance: it waits for
    /// one of the file descriptors from the sets to become ready to perform
    /// I/O.

    /// ### Function Arguments
    /// The `epoll_create_syscall()` receives one argument:
    /// * `size` - the size argument is a legacy argument in Linux and is
    ///   ignored, but must be greater than zero

    /// ### Returns
    /// On success, the system calls return a file descriptor (a nonnegative
    /// integer).
    ///
    /// ### Errors
    /// * ENFILE - file descriptor number reached the limit
    /// * EINVAL - size is not positive.
    ///
    /// ### Panics
    /// No panic is expected from this syscall
    ///
    /// more details at https://man7.org/linux/man-pages/man2/epoll_create.2.html
    pub fn epoll_create_syscall(&self, size: i32) -> i32 {
        if size <= 0 {
            return syscall_error(
                Errno::EINVAL,
                "epoll create",
                "provided size argument is invalid",
            );
        }
        return Self::_epoll_object_allocator(self);
    }

    /// ## ------------------EPOLL_CTL SYSCALL------------------
    /// ### Description
    /// This system call is used to add, modify, or remove entries in the
    /// interest list of the epoll instance referred to by the file
    /// descriptor epfd.  It requests the operation op to be performed for the
    /// target file descriptor, fd.

    /// ### Function Arguments
    /// The `epoll_ctl_syscall()` receives four arguments:
    /// * `epfd` - the epoll file descriptor to be applied the action
    /// * `op` - the operation to be performed, valid values for the op argument
    ///   are:
    /// 1. EPOLL_CTL_ADD: Add an entry to the interest list of the epoll file
    ///    descriptor, epfd. The entry includes the file descriptor, fd, a
    ///    reference to the corresponding open file description, and the
    ///    settings specified in event.
    /// 2. EPOLL_CTL_MOD: Change the settings associated with fd in the interest
    ///    list to the new settings specified in event.
    /// 3. EPOLL_CTL_DEL: Remove (deregister) the target file descriptor fd from
    ///    the interest list.
    /// * `fd` - the target file descriptor to be performed by op
    /// * `event` - The event argument describes the object linked to the file
    ///   descriptor fd.

    /// ### Returns
    /// When successful, epoll_ctl_syscall returns zero.
    ///
    /// ### Errors
    /// * EBADF - epfd or fd is not a valid file descriptor.
    /// * EEXIST - op was EPOLL_CTL_ADD, and the supplied file descriptor fd is
    ///   already registered with this epoll instance.
    /// * EINVAL - epfd is not an epoll file descriptor, or fd is the same as
    ///   epfd, or the requested operation op is not supported by this
    ///   interface.
    /// * ENOENT - op was EPOLL_CTL_MOD or EPOLL_CTL_DEL, and fd is not
    ///   registered with this epoll instance.
    /// * EPERM - The target file fd does not support epoll.  This error can
    ///   occur if fd refers to, for example, a regular file or a directory.
    ///
    /// ### Panics
    /// No panic is expected from this syscall
    ///
    /// more details at https://man7.org/linux/man-pages/man2/epoll_ctl.2.html
    pub fn epoll_ctl_syscall(&self, epfd: i32, op: i32, fd: i32, event: &EpollEvent) -> i32 {
        // first check the fds are within the valid range
        if epfd < 0 || epfd >= MAXFD {
            return syscall_error(
                Errno::EBADF,
                "epoll ctl",
                "provided epoll fd is not a valid file descriptor",
            );
        }

        if fd < 0 || fd >= MAXFD {
            return syscall_error(
                Errno::EBADF,
                "epoll ctl",
                "provided fd is not a valid file descriptor",
            );
        }

        // making sure that the epfd is really an epoll fd
        let checkedfd = self.get_filedescriptor(epfd).unwrap();
        let mut unlocked_fd = checkedfd.write();
        if let Some(filedesc_enum_epollfd) = &mut *unlocked_fd {
            if let Epoll(epollfdobj) = filedesc_enum_epollfd {
                // first check if fd equals to epfd
                // standard says EINVAL should be returned when fd equals to epfd
                // must check before trying to get the read lock of fd
                // otherwise deadlock would occur (trying to get read lock while the
                // same fd is already hold with write lock)
                if fd == epfd {
                    return syscall_error(
                        Errno::EINVAL,
                        "epoll ctl",
                        "provided fd is the same as epfd",
                    );
                }

                // check if the other fd is an epoll or not...
                let checkedfd = self.get_filedescriptor(fd).unwrap();
                let unlocked_fd = checkedfd.read();
                if let Some(filedesc_enum) = &*unlocked_fd {
                    match filedesc_enum {
                        Epoll(_) => {
                            // nested Epoll (i.e. Epoll monitoring on Epoll file descriptor)
                            // is allowed on Linux with some restrictions, though we currently do
                            // not support this

                            return syscall_error(
                                Errno::EBADF,
                                "epoll ctl",
                                "provided fd is not a valid file descriptor",
                            );
                        }
                        File(_) => {
                            // according to standard, EPERM should be returned when
                            // fd refers to a file or directory
                            return syscall_error(
                                Errno::EPERM,
                                "epoll ctl",
                                "The target file fd does not support epoll.",
                            );
                        }
                        // other file descriptors are valid
                        _ => {}
                    }
                } else {
                    // fd is not a valid file descriptor
                    return syscall_error(
                        Errno::EBADF,
                        "epoll ctl",
                        "provided fd is not a valid file descriptor",
                    );
                }

                // now that we know that the types are all good...
                match op {
                    EPOLL_CTL_DEL => {
                        // check if the fd that we are modifying exists or not
                        if !epollfdobj.registered_fds.contains_key(&fd) {
                            return syscall_error(
                                Errno::ENOENT,
                                "epoll ctl",
                                "fd is not registered with this epfd",
                            );
                        }
                        // if the fd already exists, remove the entry
                        epollfdobj.registered_fds.remove(&fd);
                    }
                    EPOLL_CTL_MOD => {
                        // check if the fd that we are modifying exists or not
                        if !epollfdobj.registered_fds.contains_key(&fd) {
                            return syscall_error(
                                Errno::ENOENT,
                                "epoll ctl",
                                "fd is not registered with this epfd",
                            );
                        }
                        // if the fd already exists, insert overwrites the prev entry
                        epollfdobj.registered_fds.insert(
                            fd,
                            EpollEvent {
                                events: event.events,
                                fd: event.fd,
                            },
                        );
                    }
                    EPOLL_CTL_ADD => {
                        //check if the fd that we are modifying exists or not
                        if epollfdobj.registered_fds.contains_key(&fd) {
                            return syscall_error(
                                Errno::EEXIST,
                                "epoll ctl",
                                "fd is already registered",
                            );
                        }
                        // add the fd and events
                        epollfdobj.registered_fds.insert(
                            fd,
                            EpollEvent {
                                events: event.events,
                                fd: event.fd,
                            },
                        );
                    }
                    _ => {
                        return syscall_error(Errno::EINVAL, "epoll ctl", "provided op is invalid");
                    }
                }
            } else {
                // epfd is not epoll object
                return syscall_error(
                    Errno::EINVAL,
                    "epoll ctl",
                    "provided epoll fd is not a valid epoll file descriptor",
                );
            }
        } else {
            // epfd is not a valid file descriptor
            return syscall_error(
                Errno::EBADF,
                "epoll ctl",
                "provided fd is not a valid file descriptor",
            );
        }
        return 0;
    }

    /// ## ------------------EPOLL_WAIT SYSCALL------------------
    /// ### Description
    /// The epoll_wait_syscall waits for events on the epoll instance
    /// referred to by the file descriptor epfd. The buffer pointed to by events
    /// is used to return information from the ready list about file descriptors
    /// in the interest list that have some events available.  Up to maxevents
    /// are returned by epoll_wait_syscall(). The maxevents argument must be
    /// greater than zero.

    /// ### Function Arguments
    /// The `epoll_wait_syscall()` receives four arguments:
    /// * `epfd` - the epoll file descriptor on which the action is to be
    ///   performed
    /// * `events` - The buffer of array of EpollEvent used to store returned
    ///   information from the ready list about file descriptors in the interest
    ///   list that have some events available
    /// * `maxevents` - maximum number of returned events. The maxevents
    ///   argument must be greater than zero.
    /// * `timeout` - The timeout argument is a RustDuration structure that
    ///   specifies the interval that epoll_wait_syscall should block waiting
    ///   for a file descriptor to become ready.

    /// ### Returns
    /// On success, epoll_wait_syscall returns the number of file descriptors
    /// ready for the requested I/O operation, or zero if no file descriptor
    /// became ready when timeout expires
    ///
    /// ### Errors
    /// * EBADF - epfd is not a valid file descriptor.
    /// * EINTR - The call was interrupted by a signal handler before either (1)
    ///   any of the requested events occurred or (2) the timeout expired
    /// * EINVAL - epfd is not an epoll file descriptor, or maxevents is less
    ///   than or equal to zero.
    ///
    /// ### Panics
    /// * when maxevents is larger than the size of events, index_out_of_bounds
    ///   panic may occur
    ///
    /// more details at https://man7.org/linux/man-pages/man2/epoll_wait.2.html
    pub fn epoll_wait_syscall(
        &self,
        epfd: i32,
        events: &mut [EpollEvent],
        maxevents: i32,
        timeout: Option<interface::RustDuration>,
    ) -> i32 {
        // current implementation of epoll is still based on poll_syscall,
        // we are essentially transforming the epoll input to poll input then
        // feeding into poll_syscall, and transforming the poll_syscall output
        // back to epoll result. Such method gives several issues:
        // 1. epoll is supposed to support a brand new mode called edge-triggered
        // mode, which only considers a fd to be ready only when new changes are made
        // to the fd. Currently, we do not support this feature
        // 2. several flags, such as EPOLLRDHUP, EPOLLERR, etc. are not supported
        // since poll_syscall currently does not support these flags, so epoll_syscall
        // that relies on poll_syscall, as a consequence, does not support them

        // first check the fds are within the valid range
        if epfd < 0 || epfd >= MAXFD {
            return syscall_error(
                Errno::EBADF,
                "epoll wait",
                "provided epoll fd is not a valid file descriptor",
            );
        }

        // get the file descriptor object
        let checkedfd = self.get_filedescriptor(epfd).unwrap();
        let mut unlocked_fd = checkedfd.write();
        if let Some(filedesc_enum) = &mut *unlocked_fd {
            // check if epfd is a valid Epoll object
            if let Epoll(epollfdobj) = filedesc_enum {
                // maxevents should be larger than 0
                if maxevents <= 0 {
                    return syscall_error(
                        Errno::EINVAL,
                        "epoll wait",
                        "max events argument is not a positive number",
                    );
                }
                // transform epoll instance into poll instance
                let mut poll_fds_vec: Vec<PollStruct> = vec![];
                let mut rm_fds_vec: Vec<i32> = vec![];
                // iterate through each registered fds
                for set in epollfdobj.registered_fds.iter() {
                    let (&key, &value) = set.pair();

                    // check if any of the registered fds were closed, add them to remove list
                    let checkedregfd = self.get_filedescriptor(key).unwrap();
                    let unlocked_regfd = checkedregfd.read();
                    if unlocked_regfd.is_none() {
                        rm_fds_vec.push(key);
                        continue;
                    }

                    // get the events to monitor
                    let events = value.events;
                    let mut structpoll = PollStruct {
                        fd: key,
                        events: 0,
                        revents: 0,
                    };
                    // check for each supported event
                    // EPOLLIN: if the fd is ready to read
                    if events & EPOLLIN as u32 > 0 {
                        structpoll.events |= POLLIN;
                    }
                    // EPOLLOUT: if the fd is ready to write
                    if events & EPOLLOUT as u32 > 0 {
                        structpoll.events |= POLLOUT;
                    }
                    // EPOLLPRI: if the fd has any exception?
                    if events & EPOLLPRI as u32 > 0 {
                        structpoll.events |= POLLPRI;
                    }
                    // now PollStruct is constructed, push it to the vector
                    poll_fds_vec.push(structpoll);
                }

                for fd in rm_fds_vec.iter() {
                    epollfdobj.registered_fds.remove(fd);
                } // remove closed fds

                // call poll_syscall
                let poll_fds_slice = &mut poll_fds_vec[..];
                let pollret = Self::poll_syscall(&self, poll_fds_slice, timeout);
                if pollret < 0 {
                    // in case of error, return the error
                    return pollret;
                }
                // the counter is used for making sure the number of returned ready fds
                // is smaller than or equal to maxevents
                let mut count = 0;
                for result in poll_fds_slice.iter() {
                    // transform the poll result into epoll result
                    // poll_event is used for marking if the fd is ready for something

                    // events are requested events for poll
                    // revents are the returned events and all results are stored here
                    let mut poll_event = false;
                    let mut event = EpollEvent {
                        events: 0,
                        fd: epollfdobj.registered_fds.get(&result.fd).unwrap().fd,
                    };
                    // check for POLLIN
                    if result.revents & POLLIN > 0 {
                        event.events |= EPOLLIN as u32;
                        poll_event = true;
                    }
                    // check for POLLOUT
                    if result.revents & POLLOUT > 0 {
                        event.events |= EPOLLOUT as u32;
                        poll_event = true;
                    }
                    // check for POLLPRI
                    if result.revents & POLLPRI > 0 {
                        event.events |= EPOLLPRI as u32;
                        poll_event = true;
                    }

                    // if the fd is ready for something
                    // add it to the return array
                    if poll_event {
                        events[count] = event;
                        count += 1;
                        // if already reached maxevents, break
                        if count >= maxevents as usize {
                            break;
                        }
                    }
                }
                return count as i32;
            } else {
                // the fd is not an epoll object
                return syscall_error(
                    Errno::EINVAL,
                    "epoll wait",
                    "provided fd is not an epoll file descriptor",
                );
            }
        } else {
            // epfd is not a valid file descriptor
            return syscall_error(
                Errno::EBADF,
                "epoll wait",
                "provided fd is not a valid file descriptor",
            );
        }
    }

    /// ## ------------------SOCKETPAIR SYSCALL------------------
    /// ### Description
    /// The `socketpair_syscall()` call creates an unnamed pair of connected
    /// sockets in the specified domain, of the specified type, and using
    /// the optionally specified protocol.
    /// The file descriptors used in referencing the new sockets are returned
    /// in sv.sock1 and sv.sock2. The two sockets are indistinguishable.
    /// ### Function Arguments
    /// The `socketpair_syscall()` receives four arguments:
    /// * `domain` -  The domain argument specifies a communication domain; this
    ///   selects the protocol family which will be used for communication.
    ///   Currently supported domains are AF_UNIX, AF_INET and AF_INET6.
    /// * `socktype` - specifies the communication semantics. Currently defined
    ///   types are:
    ///                1. SOCK_STREAM Provides sequenced, reliable, two-way,
    ///                   connection-based byte streams.  An out-of-band data
    ///                   transmission mechanism may be supported.
    ///                2. SOCK_DGRAM Supports datagrams (connectionless,
    ///                   unreliable messages of a fixed maximum length). The
    ///                   type argument serves a second purpose: in addition to
    ///                   specifying a socket type, it may include the bitwise
    ///                   OR of any of the following values, to modify the
    ///                   behavior of the socket:
    ///                1. SOCK_NONBLOCK Set the O_NONBLOCK file status flag on
    ///                   the open file description referred to by the new file
    ///                   descriptor.
    ///                2. SOCK_CLOEXEC Set the close-on-exec flag on the new
    ///                   file descriptor.
    /// * `protocol` - The protocol specifies a particular protocol to be used
    ///   with the socket. Currently only support the default protocol
    ///   (IPPROTO_TCP).
    /// * `sv` -  The file descriptors used in referencing the new sockets are
    ///   returned in sv.sock1 and sv.sock2. The two sockets are
    ///   indistinguishable.

    /// ### Returns
    /// On success, zero is returned. Otherwise, errors or panics are returned
    /// for different scenarios.
    ///
    /// ### Errors
    /// * EAFNOSUPPORT - The specified address family is not supported on this
    ///   machine.
    /// * EOPNOTSUPP - The specified protocol does not support creation of
    ///   socket pairs.
    /// * EINVAL - The specified flag is not valid
    /// * ENFILE - no enough file descriptors can be assigned
    ///
    /// ### Panics
    /// No Panic is expected for this syscall.

    // Because socketpair needs to spawn off a helper thread to connect the two ends
    // of the socket pair, and because that helper thread, along with the main
    // thread, need to access the cage to call methods (syscalls) of it, and because
    // rust's threading model states that any reference passed into a thread but
    // not moved into it mut have a static lifetime, we cannot use a standard member
    // function to perform this syscall, and must use an arc wrapped cage
    // instead as a "this" parameter in lieu of self
    pub fn socketpair_syscall(
        this: interface::RustRfc<Cage>,
        domain: i32,
        socktype: i32,
        protocol: i32,
        sv: &mut interface::SockPair,
    ) -> i32 {
        let newprotocol = if protocol == 0 { IPPROTO_TCP } else { protocol }; // only support protocol of 0 currently
                                                                              // BUG: current implementation of socketpair creates two sockets and bind to an
                                                                              // unique address.
                                                                              // But according to standard, the sockets created from socketpair should not
                                                                              // bind to any address

        // firstly check the parameters
        // socketpair should always be a AF_UNIX TCP socket
        if domain != AF_UNIX {
            return syscall_error(
                Errno::EOPNOTSUPP,
                "socketpair",
                "Linux socketpair only supports AF_UNIX aka AF_LOCAL domain.",
            );
        // socket type is stored at the lowest 3 bits
        // so we and it with 0x7 to retrieve it
        } else if socktype & 0x7 != SOCK_STREAM || newprotocol != IPPROTO_TCP {
            return syscall_error(
                Errno::EOPNOTSUPP,
                "socketpair",
                "Socketpair currently only supports SOCK_STREAM TCP.",
            );
        }

        // check if socktype contains any invalid flag bits
        if socktype & !(SOCK_NONBLOCK | SOCK_CLOEXEC | 0x7) != 0 {
            return syscall_error(Errno::EINVAL, "socket", "Invalid combination of flags");
        }

        // get the flags
        let nonblocking = (socktype & SOCK_NONBLOCK) != 0;
        let cloexec = (socktype & SOCK_CLOEXEC) != 0;

        // create 2 file discriptors
        let sock1fdobj = this._socket_initializer(
            domain,
            socktype,
            newprotocol,
            nonblocking,
            cloexec,
            ConnState::NOTCONNECTED,
        );
        let sock1fd = this._socket_inserter(Socket(sock1fdobj.clone()));
        let sock2fdobj = this._socket_initializer(
            domain,
            socktype,
            newprotocol,
            nonblocking,
            cloexec,
            ConnState::NOTCONNECTED,
        );
        let sock2fd = this._socket_inserter(Socket(sock2fdobj.clone()));

        // assign local addresses and connect
        // we are not supposed to assign and bind address to socketpair sockets
        let sock1tmp = sock1fdobj.handle.clone();
        let sock2tmp = sock2fdobj.handle.clone();
        let mut sock1handle = sock1tmp.write();
        let mut sock2handle = sock2tmp.write();
        let localaddr1 = Self::assign_new_addr_unix(&sock1handle);
        let localaddr2 = Self::assign_new_addr_unix(&sock2handle);
        this.bind_inner_socket(&mut *sock1handle, &localaddr1, false);
        this.bind_inner_socket(&mut *sock2handle, &localaddr2, false);

        // setup the pipes
        let (pipe1, pipe2) = create_unix_sockpipes();
        // one handle's remote address is the other's local address
        sock1handle.remoteaddr = Some(localaddr2.clone());
        sock2handle.remoteaddr = Some(localaddr1.clone());
        // one handle's sendpipe is the other's receivepipe
        sock1handle.unix_info.as_mut().unwrap().sendpipe = Some(pipe1.clone());
        sock1handle.unix_info.as_mut().unwrap().receivepipe = Some(pipe2.clone());
        sock2handle.unix_info.as_mut().unwrap().sendpipe = Some(pipe2.clone());
        sock2handle.unix_info.as_mut().unwrap().receivepipe = Some(pipe1.clone());

        // now they are connected
        sock1handle.state = ConnState::CONNECTED;
        sock2handle.state = ConnState::CONNECTED;

        sv.sock1 = sock1fd;
        sv.sock2 = sock2fd;

        // since socket file should not exist in the first place
        // the code below is supposed to be removed as well

        // we need to increment the refcount of the sockets we created
        // reason: in bind_inner_socket, we added entries to the inode table
        let inode1num = sock1handle.unix_info.as_mut().unwrap().inode;
        if let Inode::Socket(ref mut sock) = *(FS_METADATA.inodetable.get_mut(&inode1num).unwrap())
        {
            sock.refcount += 1;
        }
        let inode2num = sock2handle.unix_info.as_mut().unwrap().inode;
        if let Inode::Socket(ref mut sock) = *(FS_METADATA.inodetable.get_mut(&inode2num).unwrap())
        {
            sock.refcount += 1;
        }

        return 0;
    }

    // all this does is send the net_devs data in a string to libc, where we will
    // later parse and alloc into getifaddrs structs
    pub fn getifaddrs_syscall(&self, buf: *mut u8, count: usize) -> i32 {
        if NET_IFADDRS_STR.len() < count {
            interface::fill(
                buf,
                NET_IFADDRS_STR.len(),
                &NET_IFADDRS_STR.as_bytes().to_vec(),
            );
            0 // return success
        } else {
            return syscall_error(Errno::EOPNOTSUPP, "getifaddrs", "invalid ifaddrs length");
        }
    }
}
