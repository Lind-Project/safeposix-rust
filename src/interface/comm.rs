// // Author: Nicholas Renner
// //
// //

// use std::collections::{HashMap, HashSet};


// static _BOUND_SOCKETS: HashMap = HashMap::new();

// static OPEN_SOCKET_INFO: HashMap = HashMap::new();

// static PENDING_SOCKETS: HashSet = HashSet::new();


// static user_ip_interface_preferences: bool = false;

// static allow_nonspecified_ips: bool = true;

pub use std::net::{SocketAddr as RustSockAddr, IpAddr as RustIpAddr, UdpSocket as RustUdpSocket, Ipv4Addr as RustIpv4Addr, Ipv6Addr as RustIpv6Addr};
