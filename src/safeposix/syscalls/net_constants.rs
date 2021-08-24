// Network related constants
#![allow(dead_code)]
#![allow(non_upper_case_globals)]

use crate::interface;

//used for gethostname syscall
pub const DEFAULT_HOSTNAME: &str = "Lind";

// Define constants using static or const
// Imported into net_calls file

pub const SOCK_STREAM: i32 = 1; //stream socket
pub const SOCK_DGRAM: i32 = 2; //datagram socket
pub const SOCK_RAW: i32 = 3; //raw protocol interface
pub const SOCK_RDM: i32 = 4; //reliably delivered message
pub const SOCK_SEQPACKET: i32 = 5; //sequenced packet stream
pub const SOCK_CLOEXEC: i32 = 02000000;
pub const SOCK_NONBLOCK: i32 = 0x4000;


// Address families...

pub const AF_UNSPEC: i32 = 0;       // unspecified
pub const AF_UNIX: i32 = 1;         // local to host (pipes)
pub const AF_LOCAL: i32 = AF_UNIX;  // backward compatibility
pub const AF_INET: i32 = 2;         // internetwork: UDP, TCP, etc.
pub const AF_IMPLINK: i32 = 3;      // arpanet imp addresses
pub const AF_PUP: i32 = 4;          // pup protocols: e.g. BSP
pub const AF_CHAOS: i32 = 5;        // mit CHAOS protocols
pub const AF_NS: i32 = 6;           // XEROX NS protocols
pub const AF_ISO: i32 = 7;          // ISO protocols
pub const AF_OSI: i32 = AF_ISO;
pub const AF_ECMA: i32 = 8;         // European computer manufacturers
pub const AF_DATAKIT: i32 = 9;      // datakit protocols
pub const AF_CCITT: i32 = 10;       // CCITT protocols, X.25 etc
pub const AF_SNA: i32 = 11;         // IBM SNA
pub const AF_DECnet: i32 = 12;      // DECnet
pub const AF_DLI: i32 = 13;         // DEC Direct data link interface
pub const AF_LAT: i32 = 14;         // LAT
pub const AF_HYLINK: i32 = 15;      // NSC Hyperchannel
pub const AF_APPLETALK: i32 = 16;   // Apple Talk
pub const AF_ROUTE: i32 = 17;       // Internal Routing Protocol
pub const AF_LINK: i32 = 18;        // Link layer interface
pub const pseudo_AF_XTP: i32 = 19;  // eXpress Transfer Protocol (no AF)
pub const AF_COIP: i32 = 20;        // connection-oriented IP, aka ST II
pub const AF_CNT: i32 = 21;         // Computer Network Technology
pub const pseudo_AF_RTIP: i32 = 22; // Help Identify RTIP packets
pub const AF_IPX: i32 = 23;         // Novell Internet Protocol
pub const AF_SIP: i32 = 24;         // Simple Internet Protocol
pub const pseudo_AF_PIP: i32 = 25;  // Help Identify PIP packets
pub const pseudo_AF_BLUE: i32 = 26; // Identify packets for Blue Box - Not used
pub const AF_NDRV: i32 = 27;        // Network Driver 'raw' access
pub const AF_ISDN: i32 = 28;        // Integrated Services Digital Network
pub const AF_E164: i32 = AF_ISDN;   // CCITT E.164 recommendation
pub const pseudo_AF_KEY: i32 = 29;  // Internal key-management function
pub const AF_INET6: i32 = 30;       // IPv6
pub const AF_NATM: i32 = 31;        // native ATM access
pub const AF_SYSTEM: i32 = 32;      // Kernel event messages
pub const AF_NETBIOS: i32 = 33;     // NetBIOS
pub const AF_PPP: i32 = 34;         // PPP communication protocol
pub const pseudo_AF_HDRCMPLT: i32 = 35;// Used by BPF to not rewrite headers in interface output routines
pub const AF_RESERVED_36: i32 = 36; // Reserved for internal usage
pub const AF_IEEE80211: i32 = 37;   // IEEE 802.11 protocol
pub const AF_MAX: i32 = 38;

// protocols...

pub const IPPROTO_IP: i32 = 0;         // dummy for IP
pub const IPPROTO_ICMP: i32 = 1;       // control message protocol
pub const IPPROTO_IGMP: i32 = 2;       // group mgmt protocol
pub const IPPROTO_GGP: i32 = 3;        // gateway^2 (deprecated)
pub const IPPROTO_IPV4: i32 = 4;       // IPv4 encapsulation
pub const IPPROTO_IPIP: i32 = IPPROTO_IPV4;       // for compatibility
pub const IPPROTO_TCP: i32 = 6;        // tcp
pub const IPPROTO_ST: i32 = 7;         // Stream protocol II
pub const IPPROTO_EGP: i32 = 8;        // exterior gateway protocol
pub const IPPROTO_PIGP: i32 = 9;       // private interior gateway
pub const IPPROTO_RCCMON: i32 = 10;    // BBN RCC Monitoring
pub const IPPROTO_NVPII: i32 = 11;     // network voice protocol
pub const IPPROTO_PUP: i32 = 12;       // pup
pub const IPPROTO_ARGUS: i32 = 13;     // Argus
pub const IPPROTO_EMCON: i32 = 14;     // EMCON
pub const IPPROTO_XNET: i32 = 15;      // Cross Net Debugger
pub const IPPROTO_CHAOS: i32 = 16;     // Chaos
pub const IPPROTO_UDP: i32 = 17;       // user datagram protocol
pub const IPPROTO_MUX: i32 = 18;       // Multiplexing
pub const IPPROTO_MEAS: i32 = 19;      // DCN Measurement Subsystems
pub const IPPROTO_HMP: i32 = 20;       // Host Monitoring
pub const IPPROTO_PRM: i32 = 21;       // Packet Radio Measurement
pub const IPPROTO_IDP: i32 = 22;       // xns idp
pub const IPPROTO_TRUNK1: i32 = 23;    // Trunk-1
pub const IPPROTO_TRUNK2: i32 = 24;    // Trunk-2
pub const IPPROTO_LEAF1: i32 = 25;     // Leaf-1
pub const IPPROTO_LEAF2: i32 = 26;     // Leaf-2
pub const IPPROTO_RDP: i32 = 27;       // Reliable Data
pub const IPPROTO_IRTP: i32 = 28;      // Reliable Transaction
pub const IPPROTO_TP: i32 = 29;        // tp-4 w/ class negotiation
pub const IPPROTO_BLT: i32 = 30;       // Bulk Data Transfer
pub const IPPROTO_NSP: i32 = 31;       // Network Services
pub const IPPROTO_INP: i32 = 32;       // Merit Internodal
pub const IPPROTO_SEP: i32 = 33;       // Sequential Exchange
pub const IPPROTO_3PC: i32 = 34;       // Third Party Connect
pub const IPPROTO_IDPR: i32 = 35;      // InterDomain Policy Routing
pub const IPPROTO_XTP: i32 = 36;       // XTP
pub const IPPROTO_DDP: i32 = 37;       // Datagram Delivery
pub const IPPROTO_CMTP: i32 = 38;      // Control Message Transport
pub const IPPROTO_TPXX: i32 = 39;      // TP++ Transport
pub const IPPROTO_IL: i32 = 40;        // IL transport protocol
pub const IPPROTO_IPV6: i32 = 41;      // IP6 header
pub const IPPROTO_SDRP: i32 = 42;      // Source Demand Routing
pub const IPPROTO_ROUTING: i32 = 43;   // IP6 routing header
pub const IPPROTO_FRAGMENT: i32 = 44;  // IP6 fragmentation header
pub const IPPROTO_IDRP: i32 = 45;      // InterDomain Routing
pub const IPPROTO_RSVP: i32 = 46;      // resource reservation
pub const IPPROTO_GRE: i32 = 47;       // General Routing Encap.
pub const IPPROTO_MHRP: i32 = 48;      // Mobile Host Routing
pub const IPPROTO_BHA: i32 = 49;       // BHA
pub const IPPROTO_ESP: i32 = 50;       // IP6 Encap Sec. Payload
pub const IPPROTO_AH: i32 = 51;        // IP6 Auth Header
pub const IPPROTO_INLSP: i32 = 52;     // Integ. Net Layer Security
pub const IPPROTO_SWIPE: i32 = 53;     // IP with encryption
pub const IPPROTO_NHRP: i32 = 54;      // Next Hop Resolution
// 55-57: Unassigned
pub const IPPROTO_ICMPV6: i32 = 58;    // ICMP6
pub const IPPROTO_NONE: i32 = 59;      // IP6 no next header
pub const IPPROTO_DSTOPTS: i32 = 60;   // IP6 destination option
pub const IPPROTO_AHIP: i32 = 61;      // any host internal protocol
pub const IPPROTO_CFTP: i32 = 62;      // CFTP
pub const IPPROTO_HELLO: i32 = 63;     // "hello" routing protocol
pub const IPPROTO_SATEXPAK: i32 = 64;  // SATNET/Backroom EXPAK
pub const IPPROTO_KRYPTOLAN: i32 = 65; // Kryptolan
pub const IPPROTO_RVD: i32 = 66;       // Remote Virtual Disk
pub const IPPROTO_IPPC: i32 = 67;      // Pluribus Packet Core
pub const IPPROTO_ADFS: i32 = 68;      // Any distributed FS
pub const IPPROTO_SATMON: i32 = 69;    // Satnet Monitoring
pub const IPPROTO_VISA: i32 = 70;      // VISA Protocol
pub const IPPROTO_IPCV: i32 = 71;      // Packet Core Utility
pub const IPPROTO_CPNX: i32 = 72;      // Comp. Prot. Net. Executive
pub const IPPROTO_CPHB: i32 = 73;      // Comp. Prot. HeartBeat
pub const IPPROTO_WSN: i32 = 74;       // Wang Span Network
pub const IPPROTO_PVP: i32 = 75;       // Packet Video Protocol
pub const IPPROTO_BRSATMON: i32 = 76;  // BackRoom SATNET Monitoring
pub const IPPROTO_ND: i32 = 77;        // Sun net disk proto (temp.)
pub const IPPROTO_WBMON: i32 = 78;     // WIDEBAND Monitoring
pub const IPPROTO_WBEXPAK: i32 = 79;   // WIDEBAND EXPAK
pub const IPPROTO_EON: i32 = 80;       // ISO cnlp
pub const IPPROTO_VMTP: i32 = 81;      // VMTP
pub const IPPROTO_SVMTP: i32 = 82;     // Secure VMTP
pub const IPPROTO_VINES: i32 = 83;     // Banyon VINES
pub const IPPROTO_TTP: i32 = 84;       // TTP
pub const IPPROTO_IGP: i32 = 85;       // NSFNET-IGP
pub const IPPROTO_DGP: i32 = 86;       // dissimilar gateway prot.
pub const IPPROTO_TCF: i32 = 87;       // TCF
pub const IPPROTO_IGRP: i32 = 88;      // Cisco/GXS IGRP
pub const IPPROTO_OSPFIGP: i32 = 89;   // OSPFIGP
pub const IPPROTO_SRPC: i32 = 90;      // Strite RPC protocol
pub const IPPROTO_LARP: i32 = 91;      // Locus Address Resoloution
pub const IPPROTO_MTP: i32 = 92;       // Multicast Transport
pub const IPPROTO_AX25: i32 = 93;      // AX.25 Frames
pub const IPPROTO_IPEIP: i32 = 94;     // IP encapsulated in IP
pub const IPPROTO_MICP: i32 = 95;      // Mobile Int.ing control
pub const IPPROTO_SCCSP: i32 = 96;     // Semaphore Comm. security
pub const IPPROTO_ETHERIP: i32 = 97;   // Ethernet IP encapsulation
pub const IPPROTO_ENCAP: i32 = 98;     // encapsulation header
pub const IPPROTO_APES: i32 = 99;      // any private encr. scheme
pub const IPPROTO_GMTP: i32 = 100;     // GMTP
pub const IPPROTO_PIM: i32 = 103;      // Protocol Independent Mcast
pub const IPPROTO_IPCOMP: i32 = 108;   // payload compression (IPComp)
pub const IPPROTO_PGM: i32 = 113;      // PGM
pub const IPPROTO_SCTP: i32 = 132;     // SCTP
pub const IPPROTO_DIVERT: i32 = 254;   // divert pseudo-protocol
pub const IPPROTO_RAW: i32 = 255;      // raw IP packet
pub const IPPROTO_MAX: i32 = 256;
// last return value of *_input(), meaning "all job for this pkt is done".
pub const IPPROTO_DONE: i32 = 257;


// Protocol families are derived from above...

pub const PF_UNSPEC: i32 = AF_UNSPEC;
pub const PF_LOCAL: i32 = AF_LOCAL;
pub const PF_UNIX: i32 = PF_LOCAL;          // backward compatibility
pub const PF_FILE: i32 = PF_LOCAL;          // used on Linux
pub const PF_INET: i32 = AF_INET;
pub const PF_IMPLINK: i32 = AF_IMPLINK;
pub const PF_PUP: i32 = AF_PUP;
pub const PF_CHAOS: i32 = AF_CHAOS;
pub const PF_NS: i32 = AF_NS;
pub const PF_ISO: i32 = AF_ISO;
pub const PF_OSI: i32 = AF_ISO;
pub const PF_ECMA: i32 = AF_ECMA;
pub const PF_DATAKIT: i32 = AF_DATAKIT;
pub const PF_CCITT: i32 = AF_CCITT;
pub const PF_SNA: i32 = AF_SNA;
pub const PF_DECnet: i32 = AF_DECnet;
pub const PF_DLI: i32 = AF_DLI;
pub const PF_LAT: i32 = AF_LAT;
pub const PF_HYLINK: i32 = AF_HYLINK;
pub const PF_APPLETALK: i32 = AF_APPLETALK;
pub const PF_ROUTE: i32 = AF_ROUTE;
pub const PF_LINK: i32 = AF_LINK;
pub const PF_XTP: i32 = pseudo_AF_XTP;     // really just proto family, no AF
pub const PF_COIP: i32 = AF_COIP;
pub const PF_CNT: i32 = AF_CNT;
pub const PF_SIP: i32 = AF_SIP;
pub const PF_IPX: i32 = AF_IPX;            // same format as AF_NS
pub const PF_RTIP: i32 = pseudo_AF_RTIP;   // same format as AF_INET
pub const PF_PIP: i32 = pseudo_AF_PIP;
pub const PF_NDRV: i32 = AF_NDRV;
pub const PF_ISDN: i32 = AF_ISDN;
pub const PF_KEY: i32 = pseudo_AF_KEY;
pub const PF_INET6: i32 = AF_INET6;
pub const PF_NATM: i32 = AF_NATM;
pub const PF_SYSTEM: i32 = AF_SYSTEM;
pub const PF_NETBIOS: i32 = AF_NETBIOS;
pub const PF_PPP: i32 = AF_PPP;
pub const PF_RESERVED_36: i32 = AF_RESERVED_36;
pub const PF_MAX: i32 = AF_MAX;

pub const MSG_OOB: i32        = 0x01; /* Process out-of-band data.  */
pub const MSG_PEEK: i32       = 0x02; /* Peek at incoming messages.  */
pub const MSG_DONTROUTE: i32  = 0x04; /* Don't use local routing.  */
pub const MSG_EOR: i32        = 0x08; /* Data completes record.  */
pub const MSG_TRUNC: i32      = 0x10; /* Data discarded before delivery.  */
pub const MSG_CTRUNC: i32     = 0x20; /* Control data lost before delivery.  */
pub const MSG_WAITALL: i32    = 0x40; /* Wait for full request or error.  */
pub const MSG_DONTWAIT: i32   = 0x80; /* This message should be nonblocking.  */
pub const MSG_NOSIGNAL: i32   = 0x0400;       /* Do not generate SIGPIPE on EPIPE.  */

//shutdown
pub const SHUT_RD: i32 = 0;
pub const SHUT_WR: i32 = 1;
pub const SHUT_RDWR: i32 = 2;


////////////////////// setsockopt / getsockopt...
pub const SOL_SOCKET: i32 = 1;
pub const SO_DEBUG: i32 = 0x0001;
pub const SO_ACCEPTCONN: i32 = 0x0002;
pub const SO_REUSEADDR: i32 = 0x0004;
pub const SO_KEEPALIVE: i32 = 0x0008;
pub const SO_DONTROUTE: i32 = 0x0010;
pub const SO_BROADCAST: i32 = 0x0020;
pub const SO_USELOOPBACK: i32 = 0x0040;
pub const SO_LINGER: i32 = 0x0080;
pub const SO_OOBINLINE: i32 = 0x0100;
pub const SO_REUSEPORT: i32 = 0x0200;
pub const SO_SNDBUF: i32 = 0x1001;
pub const SO_RCVBUF: i32 = 0x1002;
pub const SO_SNDLOWAT: i32 = 0x1003;
pub const SO_RCVLOWAT: i32 = 0x1004;
pub const SO_SNDTIMEO: i32 = 0x1005;
pub const SO_RCVTIMEO: i32 = 0x1006;
pub const SO_ERROR: i32 = 0x1007;
pub const SO_STYLE: i32 = 0x1008;
pub const SO_TYPE: i32 = SO_STYLE;

//haven't found libc values for the rest, yet
pub const SO_SNDBUFFORCE: i32 = 32;
pub const SO_RCVBUFFORCE: i32 = 33;
pub const SO_NO_CHECK: i32 = 11;
pub const SO_PRIORITY: i32 = 12;
pub const SO_BSDCOMPAT: i32 = 14;
pub const SO_PASSCRED: i32 = 16;
pub const SO_PEERCRED: i32 = 17;

pub const SO_SECURITY_AUTHENTICATION: i32 = 22;
pub const SO_SECURITY_ENCRYPTION_TRANSPORT: i32 = 23;
pub const SO_SECURITY_ENCRYPTION_NETWORK: i32 = 24;

pub const SO_BINDTODEVICE: i32 = 25;

/* Socket filtering */
pub const SO_ATTACH_FILTER: i32 = 26;
pub const SO_DETACH_FILTER: i32 = 27;

pub const SO_PEERNAME: i32 = 28;
pub const SO_TIMESTAMP: i32 = 29;
pub const SCM_TIMESTAMP: i32 = SO_TIMESTAMP;


pub const SO_PEERSEC: i32 = 31;
pub const SO_PASSSEC: i32 = 34;
pub const SO_TIMESTAMPNS: i32 = 35;
pub const SCM_TIMESTAMPNS: i32 = SO_TIMESTAMPNS;

pub const SO_MARK: i32 = 36;

pub const SO_TIMESTAMPING: i32 = 37;
pub const SCM_TIMESTAMPING: i32 = SO_TIMESTAMPING;

pub const SO_PROTOCOL: i32 = 38;
pub const SO_DOMAIN: i32 = 39;

pub const SO_RXQ_OVFL: i32 = 40;

// Use this to specify options on a socket. Use the protocol with setsockopt
// to specify something for all sockets with a protocol
pub const SOL_TCP: i32 = IPPROTO_TCP;
pub const SOL_UDP: i32 = IPPROTO_UDP;


pub const TCP_NODELAY: i32 = 0x01;           // don't delay send to coalesce packets
pub const TCP_MAXSEG: i32 = 0x02;            // set maximum segment size
pub const TCP_NOPUSH: i32 = 0x04;            // don't push last block of write
pub const TCP_NOOPT: i32 = 0x08;             // don't use TCP options
pub const TCP_KEEPALIVE: i32 = 0x10;         // idle time used when SO_KEEPALIVE is enabled
pub const TCP_CONNECTIONTIMEOUT: i32 = 0x20; // connection timeout
pub const PERSIST_TIMEOUT: i32 = 0x40;       // time after which a connection in persist timeout
                                        // will terminate.
                                        // see draft-ananth-tcpm-persist-02.txt
pub const TCP_RXT_CONNDROPTIME: i32 = 0x80;  // time after which tcp retransmissions will be
                                        // stopped and the connection will be dropped
pub const TCP_RXT_FINDROP: i32 = 0x100;      // When set, a connection is dropped after 3 FINs

pub const MINSOCKOBJID: i32 = 0;
pub const MAXSOCKOBJID: i32 = 1024;

//POLL CONSTANTS
pub const POLLIN: u32 = 01;  // There is data to read.
pub const POLLPRI: u32 = 02; //There is urgent data to read.
pub const POLLOUT: u32 = 04; // Writing now will not block.
pub const POLLERR: u32 = 010; // Error condition.
pub const POLLHUP: u32 = 020; // Hung up.
pub const POLLNVAL: u32 = 040; // Invalid polling request.

//EPOLL CONSTANTS
pub const EPOLLIN: i32 = 0x001;
pub const EPOLLPRI: i32 = 0x002;
pub const EPOLLOUT: i32 = 0x004;
pub const EPOLLRDNORM: i32 = 0x040;
pub const EPOLLRDBAND: i32 = 0x080;
pub const EPOLLWRNORM: i32 = 0x100;
pub const EPOLLWRBAND: i32 = 0x200;
pub const EPOLLMSG: i32 = 0x400;
pub const EPOLLERR: i32 = 0x008;
pub const EPOLLHUP: i32 = 0x010;
pub const EPOLLRDHUP: i32 = 0x2000;
pub const EPOLLWAKEUP: i32 = 1 << 29;
pub const EPOLLONESHOT: i32 = 1 << 30;
pub const EPOLLET: i32 = 1 << 31;

pub const EPOLL_CTL_ADD: i32 = 1;
pub const EPOLL_CTL_DEL: i32 = 2;
pub const EPOLL_CTL_MOD: i32 = 3;

//for internal use
#[derive(Debug, PartialEq, Eq)]
pub enum ConnState {
    NOTCONNECTED, CONNECTED, LISTEN
}
