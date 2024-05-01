# RustPOSIX [![Build Status](https://github.com/Lind-Project/RustPOSIX-rust/actions/workflows/lind-selfhost.yml/badge.svg?branch=develop)](https://github.com/Lind-Project/RustPOSIX-rust/actions/workflows/lind-selfhost.yml)

# Table of Contents
- [RustPOSIX ](#rustposix-)
- [Table of Contents](#table-of-contents)
  - [Development Guideline](#development-guideline)
  - [Run RustPOSIX-Rust](#run-rustposix-rust)
  - [Test RustPOSIX-Rust](#test-rustposix-rust)
  - [FAQs](#faqs)

RustPOSIX refers to a library operating system (libOS) built using the Rust programming language as part of the Lind project.
It is a component of the Lind sandbox that provides a subset of the POSIX API (for things like networking, file I/O, etc.) to applications running within the Google Native Client (NaCl) sandbox. leveraging Rust's memory safety and concurrency safety guarantees to provide a secure implementation of OS functionality.

More implementation details could be found at [wiki](https://github.com/Lind-Project/RustPOSIX-rust/wiki).

![alt text](docs/RustPOSIX-README.jpg)


## Development Guideline

- All PRs should be merged to the Develop branch

- Any imports from the standard library or any crates should be done in an interface file

More detailed guideline will be in [RustPOSIX's wiki](https://github.com/Lind-Project/RustPOSIX-rust/wiki/Style-Guide)

## Run RustPOSIX-Rust

Quick start
Use Develop branch for the most stable behaviour.
```
docker build -t --platform <your platform> <image_name> .devcontainer
docker run -it <image_name>

```
This will create a quick container with rustposix build at your local changes.
helpful for exploration and easy testing.


See reference at [Run RustPOSIX Independently](https://github.com/Lind-Project/RustPOSIX-rust/wiki/Run-Independently)


## Test RustPOSIX-Rust

Use Develop branch for the most stable behaviour.

```
cargo build
cargo test --lib
```

Overview of Tests being run:

1) fs_tests

   The fs_tests.rs file contains a test suite function test_fs(), which runs a series of tests related to file system operations.

2) ipc_tests

   These tests aim to validate the proper functioning of different Inter-Process Communication (IPC) mechanisms within the Lind Rust environment. They ensure that data can be transmitted and received accurately, and that system calls such as `fork`, `pipe`, `bind`, `listen`, `accept`, `send`, and `recv` behave as intended. Additionally, the tests verify that the system handles process termination correctly.

3) networking_tests

    The `networking_tests.rs` file contains a single function called `net_tests()`. This function serves as a test suite, running a series of tests that validate various networking operations within the system under test.

See reference at [Testing and Debugging](https://github.com/Lind-Project/safeposix-rust/wiki/Testing-and-Debugging)

## FAQs

1) If you encounter the net device couldn't find error, follow these steps:
   1) RUN ./gen_netdevs.sh - This script will generate or configure the necessary network devices for your application.
2) If you encounter access denied/ permissions error:
   1) RUN chmod +x gen_netdevs.sh - Giving executable permissions.
3) If you encounter any test failures with respect to networking tests
   1) something similar to below - potential reasons could be binding issues as the tests create sockets very quickly.
   ```
    src/tests/networking_tests.rs:77:9:
   ``` 
