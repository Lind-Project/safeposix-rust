#[cfg(test)]
pub mod net_tests {
    use crate::interface;
    use crate::safeposix::{cage::*, dispatcher::*, filesystem};
    use super::super::*;
    use std::mem::size_of;

    pub fn net_tests() {
        ut_lind_net_bind();
        ut_lind_net_bind_multiple();
        // ut_lind_net_bind_on_zero(); //not done
        ut_lind_net_connect_basic_udp();
        ut_lind_net_getpeername();
        ut_lind_net_getsockname();
        ut_lind_net_listen();
        ut_lind_net_recvfrom(); //not done and have no idea why this is not working
        ut_lind_net_select();   //not done
    }



    //not finished
    pub fn ut_lind_net_bind() {
        lindrustinit();
        let cage = {CAGE_TABLE.read().unwrap().get(&1).unwrap().clone()};
        let sockfd = cage.socket_syscall(AF_INET, SOCK_STREAM, 0);

        //should work...
        let socket = interface::GenSockaddr::V4(interface::SockaddrV4{ sin_family: AF_INET as u16, sin_port: 50102u16.to_be(), sin_addr: interface::V4Addr{ s_addr: u32::from_ne_bytes([127, 0, 0, 1]) }, padding: 0}); //127.0.0.1

        assert_eq!(cage.bind_syscall(sockfd, &socket, 4096), 0);
        assert_eq!(cage.bind_syscall(sockfd, &socket, 4096), -(Errno::EINVAL as i32)); //already bound so should fail

        //trying to bind another to the same IP/PORT
        let sockfd2 = cage.socket_syscall(AF_INET, SOCK_STREAM, 0);
        assert_eq!(cage.bind_syscall(sockfd2, &socket, 4096), -(Errno::EADDRINUSE as i32)); //already bound so should fail

        //UDP should still work...
        let sockfd3 = cage.socket_syscall(AF_INET, SOCK_DGRAM, 0);
        assert_eq!(cage.bind_syscall(sockfd3, &socket, 4096), 0);

        assert_eq!(cage.exit_syscall(), 0);
        lindrustfinalize();
    }

    

    pub fn ut_lind_net_bind_on_zero() {
        lindrustinit();
        let cage = {CAGE_TABLE.read().unwrap().get(&1).unwrap().clone()};

        //both the server and the socket are run from this file
        let serversockfd = cage.socket_syscall(AF_INET, SOCK_STREAM, 0);

        let clientsockfd = cage.socket_syscall(AF_INET, SOCK_STREAM, 0);
        let clientsockfd2 = cage.socket_syscall(AF_INET, SOCK_STREAM, 0);

        let socket = interface::GenSockaddr::V4(interface::SockaddrV4{ sin_family: AF_INET as u16, sin_port: 50103u16.to_be(), sin_addr: interface::V4Addr{ s_addr: 0 }, padding: 0}); //127.0.0.1
        assert_eq!(cage.bind_syscall(serversockfd, &socket, 4096), 0);
        assert_eq!(cage.listen_syscall(serversockfd, 1), 0);

        assert_eq!(cage.exit_syscall(), 0);
        lindrustfinalize();
    }



    pub fn ut_lind_net_bind_multiple() {
        lindrustinit();
        let cage = {CAGE_TABLE.read().unwrap().get(&1).unwrap().clone()};

        let mut sockfd = cage.socket_syscall(AF_INET, SOCK_STREAM, 0);
        let socket = interface::GenSockaddr::V4(interface::SockaddrV4{ sin_family: AF_INET as u16, sin_port: 50103u16.to_be(), sin_addr: interface::V4Addr{ s_addr: u32::from_ne_bytes([127, 0, 0, 1]) }, padding: 0}); //127.0.0.1
        assert_eq!(cage.bind_syscall(sockfd, &socket, 4096), 0);

        let sockfd2 = cage.socket_syscall(AF_INET, SOCK_STREAM, 0);

        //allowing port reuse
        assert_eq!(cage.setsockopt_syscall(sockfd, SOL_SOCKET, SO_REUSEPORT, 1), 0);
        assert_eq!(cage.setsockopt_syscall(sockfd2, SOL_SOCKET, SO_REUSEPORT, 1), 0);

        assert_eq!(cage.bind_syscall(sockfd2, &socket, 4096), 0);

        //double listen should be allowed
        assert_eq!(cage.listen_syscall(sockfd, 1), 0);
        assert_eq!(cage.listen_syscall(sockfd2, 1), 0);

        //UDP bind should be allowed
        sockfd = cage.socket_syscall(AF_INET, SOCK_DGRAM, 0);
        assert_eq!(cage.bind_syscall(sockfd, &socket, 4096), 0);

        assert_eq!(cage.exit_syscall(), 0);
        lindrustfinalize();
    }



    pub fn ut_lind_net_connect_basic_udp() {
        lindrustinit();
        let cage = {CAGE_TABLE.read().unwrap().get(&1).unwrap().clone()};

        //should be okay...
        let sockfd = cage.socket_syscall(AF_INET, SOCK_DGRAM, 0);
        let mut socket = interface::GenSockaddr::V4(interface::SockaddrV4{ sin_family: AF_INET as u16, sin_port: 50103u16.to_be(), sin_addr: interface::V4Addr{ s_addr: u32::from_ne_bytes([127, 0, 0, 1]) }, padding: 0}); //127.0.0.1
        assert_eq!(cage.connect_syscall(sockfd, &socket), 0);

        //should be able to retarget the socket
        socket = interface::GenSockaddr::V4(interface::SockaddrV4{ sin_family: AF_INET as u16, sin_port: 50104u16.to_be(), sin_addr: interface::V4Addr{ s_addr: u32::from_ne_bytes([127, 0, 0, 1]) }, padding: 0}); //127.0.0.1
        assert_eq!(cage.connect_syscall(sockfd, &socket), 0);

        assert_eq!(cage.exit_syscall(), 0);
        lindrustfinalize();
    }



    pub fn ut_lind_net_getpeername() {
        lindrustinit();
        let cage = {CAGE_TABLE.read().unwrap().get(&1).unwrap().clone()};

        //doing a few things with connect -- only UDP right now
        let sockfd = cage.socket_syscall(AF_INET, SOCK_DGRAM, 0);
        let mut socket = interface::GenSockaddr::V4(interface::SockaddrV4{ sin_family: AF_INET as u16, sin_port: 50103u16.to_be(), sin_addr: interface::V4Addr{ s_addr: u32::from_ne_bytes([127, 0, 0, 1]) }, padding: 0}); //127.0.0.1
        let mut retsocket = interface::GenSockaddr::V4(interface::SockaddrV4::default()); //127.0.0.1
        
        assert_eq!(cage.connect_syscall(sockfd, &socket), 0);
        assert_eq!(cage.getpeername_syscall(sockfd, &mut retsocket), 0);
        assert_eq!(retsocket, socket);

        //should be able to retarget
        socket = interface::GenSockaddr::V4(interface::SockaddrV4{ sin_family: AF_INET as u16, sin_port: 50104u16.to_be(), sin_addr: interface::V4Addr{ s_addr: u32::from_ne_bytes([127, 0, 0, 1]) }, padding: 0}); //127.0.0.1
        assert_eq!(cage.connect_syscall(sockfd, &socket), 0);
        assert_eq!(cage.getpeername_syscall(sockfd, &mut retsocket), 0);
        assert_eq!(retsocket, socket);

        assert_eq!(cage.exit_syscall(), 0);
        lindrustfinalize();
    }



    pub fn ut_lind_net_getsockname() {
        lindrustinit();
        let cage = {CAGE_TABLE.read().unwrap().get(&1).unwrap().clone()};
        
        let sockfd = cage.socket_syscall(AF_INET, SOCK_STREAM, 0);
        let mut retsocket = interface::GenSockaddr::V4(interface::SockaddrV4::default()); 

        assert_eq!(cage.getsockname_syscall(sockfd, &mut retsocket), 0);
        assert_eq!(retsocket.port(), 0);
        assert_eq!(retsocket.addr(), interface::GenIpaddr::V4(interface::V4Addr::default()));

        let mut socket = interface::GenSockaddr::V4(interface::SockaddrV4{ sin_family: AF_INET as u16, sin_port: 50104u16.to_be(), sin_addr: interface::V4Addr{ s_addr: u32::from_ne_bytes([127, 0, 0, 1]) }, padding: 0}); //127.0.0.1
        
        assert_eq!(cage.bind_syscall(sockfd, &socket, 4096), 0);
        assert_eq!(cage.getsockname_syscall(sockfd, &mut retsocket), 0);
        assert_eq!(retsocket, socket);    

        //checking that we cannot rebind the socket
        assert_eq!(cage.bind_syscall(sockfd, &socket, 4096), -(Errno::EINVAL as i32)); //already bound so should fail
        assert_eq!(cage.getsockname_syscall(sockfd, &mut retsocket), 0);
        assert_eq!(retsocket, socket);

        assert_eq!(cage.exit_syscall(), 0);
        lindrustfinalize();
    }
    
    
    
    pub fn ut_lind_net_listen() {
        lindrustinit();
        let cage = {CAGE_TABLE.read().unwrap().get(&1).unwrap().clone()};
        
        let serversockfd = cage.socket_syscall(AF_INET, SOCK_STREAM, 0);
        let clientsockfd = cage.socket_syscall(AF_INET, SOCK_STREAM, 0);
        
        assert!(serversockfd > 0);
        assert!(clientsockfd > 0);
        
        //binding to a socket
        let mut sockaddr = interface::SockaddrV4{ sin_family: AF_INET as u16, sin_port: 53000_u16.to_be(), sin_addr: interface::V4Addr{ s_addr: u32::from_ne_bytes([127, 0, 0, 1]) }, padding: 0};
        let mut socket = interface::GenSockaddr::V4(sockaddr); //127.0.0.1
        assert_eq!(cage.bind_syscall(serversockfd, &socket, 4096), 0);
        assert_eq!(cage.listen_syscall(serversockfd, 10), 0);
        
        assert_eq!(cage.fork_syscall(2), 0);
        
        let builder = std::thread::Builder::new().name("THREAD".into());
        
        let sender = builder.spawn(move || {
            let cage2 = {CAGE_TABLE.read().unwrap().get(&2).unwrap().clone()};
            
            interface::sleep(interface::RustDuration::from_millis(500)); //why does this make the whole thread block?

            let mut socket2 = interface::GenSockaddr::V4(interface::SockaddrV4{ sin_family: AF_INET as u16, sin_port: 53000_u16.to_be(), sin_addr: interface::V4Addr{ s_addr: u32::from_ne_bytes([127, 0, 0, 1]) }, padding: 0}); //127.0.0.1
            assert!(cage2.accept_syscall(serversockfd, &mut socket2) > 0); //really can only make sure that the fd is valid
            
            interface::sleep(interface::RustDuration::from_millis(100));
            
            assert_eq!(cage2.close_syscall(serversockfd), 0);
            assert_eq!(cage2.exit_syscall(), 0);
        }).unwrap();

        assert_eq!(cage.connect_syscall(clientsockfd, &socket), 0); //so connect works but accept doesn't?
        
        let mut retsocket = interface::GenSockaddr::V4(interface::SockaddrV4::default());
        assert_eq!(cage.getsockname_syscall(clientsockfd, &mut retsocket), 0);
        assert_ne!(retsocket, socket);
        
        sender.join().unwrap();
        
        assert_eq!(cage.exit_syscall(), 0);
        lindrustfinalize();
    }
    
    
    
        pub fn ut_lind_net_recvfrom() {
            lindrustinit();
            let cage = {CAGE_TABLE.read().unwrap().get(&1).unwrap().clone()};
    
            assert_eq!(cage.exit_syscall(), 0);
            lindrustfinalize();
        }
    


    pub fn ut_lind_net_select() {
        lindrustinit();
        let cage = {CAGE_TABLE.read().unwrap().get(&1).unwrap().clone()};

        assert_eq!(cage.exit_syscall(), 0);
        lindrustfinalize();
    }
}
