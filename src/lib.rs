//! # My Project
//!
//! This is a brief description of your project.
//!
//! ## Usage
//!
//! Here's an example of how to use your project:
//!
//! ```rust
//! use my_project::MyStruct;
//!
//! let my_struct = MyStruct::new();
//! my_struct.do_something();
//! ```
//!
//! ## Features
//!
//! - Feature 1: This feature does something.
//! - Feature 2: This feature does something else.
//!
//! ## Contributing
//!
//! If you'd like to contribute to this project, please follow these guidelines.
//!
//! ## License
//!
//! This project is licensed under the [MIT License](LICENSE).
//!
//! ## Acknowledgements
//!
//! Thanks to the contributors and users of this project.

#![feature(lazy_cell)]
#![feature(rustc_private)] //for private crate imports for tests
#![feature(vec_into_raw_parts)]
#![feature(thread_local)]
#![allow(unused_imports)]

mod interface;
mod safeposix;
mod tests;
mod lib_fs_utils;

// Additional documentation for your project