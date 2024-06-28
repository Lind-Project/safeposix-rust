//! This module handles system call requests from the Native Client in the
//! RustPOSIX environment.
//!
//! ## top-level features:
//!
//! - ### Dispatcher/RPC:
//!     - The dispatcher receives system call requests from Native Client. It
//!       checks if the cage exists in the cage table, and if it doesn't,
//!       initializes a new cage. It then takes the cage object corresponding to
//!       that ID number, and calls the method corresponding to the sent call
//!       number.
//!
//! - ### Cage Objects:
//!
//!     - Each cage object has a Cage ID, Current Working Directory, Parent ID,
//!       and a File Descriptor Table.
//!
//! - ### File Descriptor Table:
//!     - The file descriptor table is a hash map of file descriptor integers to
//!       our file descriptor representations. File Descriptors are implemented
//!       as an Enum that can correspond to five descriptor types (File, Stream,
//!       Socket, Pipe, Epoll).
//!
//! - ### System Calls:
//!     - Each cage object has public methods corresponding to each system call.
//!       These calls are implemented either as filesystem related calls, system
//!       related calls, or network related calls in their respective files.
//!
//! - ### FS Metadata:
//!     - The table is represented by a struct with fields: nextinode, dev_ud,
//!       inodetable. The Inode Enum can describe a variety of Inode structs
//!       which include: File(generic), CharDev, Socket, Directory.
//!
//! - ### Public Methods:
//!     - The module provides several public methods for interacting with the
//!       file descriptor table and the cage objects. Some of them are
//!       get_next_fd, load_lower_handle_stubs, insert_next_pipe. There are also
//!       some unused methods like add_to_fd_table, rm_from_fd_table, and
//!       changedir.

pub mod cage;
pub mod dispatcher;
pub mod filesystem;
pub mod net;
pub mod shm;
pub mod syscalls;
