// Author: Nicholas Renner
//! Module definitions for the RustPOSIX interface
//!
//! ## Interface Module
//!
//! Secure interface module that enforces containment of kernel calls to "popular paths" to enhance security. It restricts access to libraries only through specified paths in order to limit kernel calls to these popular paths.
//!
//! This interface limits kernel access from Rust to the popular paths as defined in Lock-in-Pop
//! Libraries are imported only via `use` statements within these files, allowing for focused testing and verification of kernel access via the slimmer interface to ensure restricted access to popular paths.

mod comm;
pub mod errnos;
mod file;
mod misc;
mod pipe;
mod timer;
pub mod types;
pub use comm::*;
pub use errnos::*;
pub use file::*;
pub use misc::*;
pub use pipe::*;
pub use timer::*;
pub use types::*;
