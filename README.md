# safeposix-rustc [![Build Status](https://github.com/Lind-Project/safeposix-rust/actions/workflows/lind-selfhost.yml/badge.svg?branch=develop)](https://github.com/Lind-Project/safeposix-rust/actions/workflows/lind-selfhost.yml)
Rust implementation of SafePOSIX

SafePOSIX refers to a library operating system (libOS) built using the Rust programming language as part of the Lind project.
It is a component of the Lind sandbox that provides a subset of the POSIX API (for things like networking, file I/O, etc.) to applications running within the Google Native Client (NaCl) sandbox. leveraging Rust's memory safety and concurrency safety guarantees to provide a secure implementation of OS functionality.

More implementation details could be found at [wiki](https://github.com/Lind-Project/safeposix-rust/wiki).

## Development Guideline

- All PRs should be merged to the Develop branch

- Any imports from the standard library or any crates should be done in an interface file

More detailed guideline will be in [SafePOSIX's wiki](https://github.com/Lind-Project/safeposix-rust/wiki/Style-Guide)

## Run SafePOSIX-Rust

See reference at [Run RustPOSIX Independently](https://github.com/Lind-Project/safeposix-rust/wiki/Run-Independently)

## Test SafePOSIX-Rust

Use Develop branch for the most stable behaviour.

```
cargo build
chmod +x gen_netdevs.sh 
./gen_netdevs.sh 
cargo test --lib
```
