#[cfg(test)]
pub mod net_tests {
    use crate::interface;
    use crate::safeposix::{cage::*, dispatcher::*, filesystem};
    use super::super::*;
    use std::mem::size_of;

    pub fn net_tests() {
        ut_lind_net_bind();
        ut_lind_net_bind_multiple();
        ut_lind_net_bind_on_zero(); 
        ut_lind_net_connect_basic_udp();
        ut_lind_net_getpeername();
        ut_lind_net_getsockname();
        ut_lind_net_listen();
        ut_lind_net_recvfrom(); 
        ut_lind_net_shutdown();
        ut_lind_net_socket();
        ut_lind_net_socketoptions();
        ut_lind_net_udp_bad_bind();
        ut_lind_net_udp_simple(); //not working right now
        ut_lind_net_udp_connect();
    }



    pub fn ut_lind_net_bind() {
        lindrustinit();
        let cage = {CAGE_TABLE.read().unwrap().get(&1).unwrap().clone()};
        let sockfd = cage.socket_syscall(AF_INET, SOCK_STREAM, 0);

        //should work...
        let socket = interface::GenSockaddr::V4(interface::SockaddrV4{ sin_family: AF_INET as u16, sin_port: 50102u16.to_be(), sin_addr: interface::V4Addr{ s_addr: u32::from_ne_bytes([127, 0, 0, 1]) }, padding: 0}); //127.0.0.1

        assert_eq!(cage.bind_syscall(sockfd, &socket), 0);
        assert_eq!(cage.bind_syscall(sockfd, &socket), -(Errno::EINVAL as i32)); //already bound so should fail

        //trying to bind another to the same IP/PORT
        let sockfd2 = cage.socket_syscall(AF_INET, SOCK_STREAM, 0);
        assert_eq!(cage.bind_syscall(sockfd2, &socket), -(Errno::EADDRINUSE as i32)); //already bound so should fail

        //UDP should still work...
        let sockfd3 = cage.socket_syscall(AF_INET, SOCK_DGRAM, 0);
        assert_eq!(cage.bind_syscall(sockfd3, &socket), 0);

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

        let port: u16 = 53002;
        
        //making sure that the assigned fd's are valid
        assert!(serversockfd > 0);
        assert!(clientsockfd > 0);
        assert!(clientsockfd2 > 0);
        
        //binding to a socket
        let sockaddr = interface::SockaddrV4{ sin_family: AF_INET as u16, sin_port: port.to_be(), sin_addr: interface::V4Addr{ s_addr: u32::from_ne_bytes([127, 0, 0, 1]) }, padding: 0};
        let mut socket = interface::GenSockaddr::V4(sockaddr); //127.0.0.1
        assert_eq!(cage.bind_syscall(serversockfd, &socket), 0);
        assert_eq!(cage.listen_syscall(serversockfd, 1), 0); //we are only allowing for one client at a time
        
        //forking the cage to get another cage with the same information
        assert_eq!(cage.fork_syscall(2), 0);

        //creating a thread for the server so that the information can be sent between the two threads
        let sender = interface::helper_thread(move || {
            
            let cage2 = {CAGE_TABLE.read().unwrap().get(&2).unwrap().clone()};
            let mut socket2 = interface::GenSockaddr::V4(interface::SockaddrV4{ sin_family: AF_INET as u16, sin_port: port.to_be(), sin_addr: interface::V4Addr{ s_addr: 0 }, padding: 0}); //0.0.0.0

            interface::sleep(interface::RustDuration::from_millis(200)); 

            let mut sockfd = cage2.accept_syscall(serversockfd, &mut socket2); //really can only make sure that the fd is valid
            assert!(sockfd > 0);

            interface::sleep(interface::RustDuration::from_millis(100)); 
        
            //process the first test...
            //Writing 100, then peek 100, then read 100
            let mut buf = sizecbuf(100);
            assert_eq!(cage2.recvfrom_syscall(sockfd, buf.as_mut_ptr(), 100, MSG_PEEK, &mut Some(&mut socket2)), 100); //peeking at the input message
            assert_eq!(cbuf2str(&buf), &"A".repeat(100));
            buf = sizecbuf(100);
            assert_eq!(cage2.recvfrom_syscall(sockfd, buf.as_mut_ptr(), 100, 0, &mut Some(&mut socket2)), 100);        //reading the input message
            assert_eq!(cbuf2str(&buf), &"A".repeat(100));
            buf = sizecbuf(100);
            buf = sizecbuf(100);

            interface::sleep(interface::RustDuration::from_millis(200)); 

            //process the second test...
            //Writing 100, read 20, peek 20, read 80
            assert_eq!(cage2.recvfrom_syscall(sockfd, buf.as_mut_ptr(), 20, 0, &mut Some(&mut socket2)), 20);
            assert_eq!(cbuf2str(&buf), "A".repeat(20) + &"\0".repeat(80));
            buf = sizecbuf(100);
            assert_eq!(cage2.recvfrom_syscall(sockfd, buf.as_mut_ptr(), 20, MSG_PEEK, &mut Some(&mut socket2)), 20);
            assert_eq!(cbuf2str(&buf), "A".repeat(20) + &"\0".repeat(80));
            buf = sizecbuf(100);
            assert_eq!(cage2.recvfrom_syscall(sockfd, buf.as_mut_ptr(), 80, 0, &mut Some(&mut socket2)), 80);
            assert_eq!(cbuf2str(&buf), "A".repeat(80) + &"\0".repeat(20));
            buf = sizecbuf(100);

            interface::sleep(interface::RustDuration::from_millis(200)); 

            //process the third test...
            //Writing 100, peek several times, read 100
            for _ in 0..4 { 
                assert_eq!(cage2.recvfrom_syscall(sockfd, buf.as_mut_ptr(), 10, MSG_PEEK, &mut Some(&mut socket2)), 10); 
                assert_eq!(cbuf2str(&buf), "A".repeat(10) + &"\0".repeat(90));
                buf = sizecbuf(100);
            }
            for _ in 0..4 { 
                assert_eq!(cage2.recvfrom_syscall(sockfd, buf.as_mut_ptr(), 20, MSG_PEEK, &mut Some(&mut socket2)), 20); 
                assert_eq!(cbuf2str(&buf), "A".repeat(20) + &"\0".repeat(80));
                buf = sizecbuf(100);
            }
            for _ in 0..4 { 
                assert_eq!(cage2.recvfrom_syscall(sockfd, buf.as_mut_ptr(), 30, MSG_PEEK, &mut Some(&mut socket2)), 30); 
                assert_eq!(cbuf2str(&buf), "A".repeat(30) + &"\0".repeat(70));
                buf = sizecbuf(100);
            }
            for _ in 0..4 { 
                assert_eq!(cage2.recvfrom_syscall(sockfd, buf.as_mut_ptr(), 40, MSG_PEEK, &mut Some(&mut socket2)), 40); 
                assert_eq!(cbuf2str(&buf), "A".repeat(40) + &"\0".repeat(60));
                buf = sizecbuf(100);
            }
            assert_eq!(cage2.recvfrom_syscall(sockfd, buf.as_mut_ptr(), 100, 0, &mut Some(&mut socket2)), 100);
            assert_eq!(cbuf2str(&buf), &"A".repeat(100));
            buf = sizecbuf(100);

            interface::sleep(interface::RustDuration::from_millis(200)); 

            //process the fourth test...
            //Writing 50, peek 50
            assert_eq!(cage2.recvfrom_syscall(sockfd, buf.as_mut_ptr(), 50, MSG_PEEK, &mut Some(&mut socket2)), 50);
            assert_eq!(cbuf2str(&buf), "A".repeat(50) + &"\0".repeat(50));
            buf = sizecbuf(100);
            assert_eq!(cage2.close_syscall(sockfd), 0);


            socket2 = interface::GenSockaddr::V4(interface::SockaddrV4{ sin_family: AF_INET as u16, sin_port: port.to_be(), sin_addr: interface::V4Addr{ s_addr: 0 }, padding: 0}); //0.0.0.0
            interface::sleep(interface::RustDuration::from_millis(200)); 
            sockfd = cage2.accept_syscall(serversockfd, &mut socket2); //really can only make sure that the fd is valid
            assert!(sockfd > 0);

            //process the first test...
            //Writing 100, then peek 100, then read 100
            let mut buf = sizecbuf(100);
            assert_eq!(cage2.recvfrom_syscall(sockfd, buf.as_mut_ptr(), 100, MSG_PEEK, &mut Some(&mut socket2)), 100); //peeking at the input message
            assert_eq!(cbuf2str(&buf), &"A".repeat(100));
            buf = sizecbuf(100);
            assert_eq!(cage2.recvfrom_syscall(sockfd, buf.as_mut_ptr(), 100, 0, &mut Some(&mut socket2)), 100);        //reading the input message
            assert_eq!(cbuf2str(&buf), &"A".repeat(100));
            buf = sizecbuf(100);

            interface::sleep(interface::RustDuration::from_millis(200)); 

            //process the second test...
            //Writing 100, read 20, peek 20, read 80
            assert_eq!(cage2.recvfrom_syscall(sockfd, buf.as_mut_ptr(), 20, 0, &mut Some(&mut socket2)), 20);
            assert_eq!(cbuf2str(&buf), "A".repeat(20) + &"\0".repeat(80));
            buf = sizecbuf(100);
            assert_eq!(cage2.recvfrom_syscall(sockfd, buf.as_mut_ptr(), 20, MSG_PEEK, &mut Some(&mut socket2)), 20);
            assert_eq!(cbuf2str(&buf), "A".repeat(20) + &"\0".repeat(80));
            buf = sizecbuf(100);
            assert_eq!(cage2.recvfrom_syscall(sockfd, buf.as_mut_ptr(), 80, 0, &mut Some(&mut socket2)), 80);
            assert_eq!(cbuf2str(&buf), "A".repeat(80) + &"\0".repeat(20));

            interface::sleep(interface::RustDuration::from_millis(200)); 

            //process the third test...
            //Writing 100, peek several times, read 100
            for _ in 0..4 { 
                buf = sizecbuf(100);
                assert_eq!(cage2.recvfrom_syscall(sockfd, buf.as_mut_ptr(), 10, MSG_PEEK, &mut Some(&mut socket2)), 10); 
                assert_eq!(cbuf2str(&buf), "A".repeat(10) + &"\0".repeat(90));
            }
            for _ in 0..4 { 
                buf = sizecbuf(100);
                assert_eq!(cage2.recvfrom_syscall(sockfd, buf.as_mut_ptr(), 20, MSG_PEEK, &mut Some(&mut socket2)), 20); 
                assert_eq!(cbuf2str(&buf), "A".repeat(20) + &"\0".repeat(80));
            }
            for _ in 0..4 { 
                buf = sizecbuf(100);
                assert_eq!(cage2.recvfrom_syscall(sockfd, buf.as_mut_ptr(), 30, MSG_PEEK, &mut Some(&mut socket2)), 30); 
                assert_eq!(cbuf2str(&buf), "A".repeat(30) + &"\0".repeat(70));
            }
            for _ in 0..4 { 
                buf = sizecbuf(100);
                assert_eq!(cage2.recvfrom_syscall(sockfd, buf.as_mut_ptr(), 40, MSG_PEEK, &mut Some(&mut socket2)), 40); 
                assert_eq!(cbuf2str(&buf), "A".repeat(40) + &"\0".repeat(60));
            }
            buf = sizecbuf(100);
            assert_eq!(cage2.recvfrom_syscall(sockfd, buf.as_mut_ptr(), 100, 0, &mut Some(&mut socket2)), 100);
            assert_eq!(cbuf2str(&buf), &"A".repeat(100));
            buf = sizecbuf(100);

            interface::sleep(interface::RustDuration::from_millis(200)); 

            //process the fourth test...
            //Writing 50, peek 50
            assert_eq!(cage2.recvfrom_syscall(sockfd, buf.as_mut_ptr(), 50, MSG_PEEK, &mut Some(&mut socket2)), 50);
            assert_eq!(cbuf2str(&buf), "A".repeat(50) + &"\0".repeat(50));
            
            interface::sleep(interface::RustDuration::from_millis(100)); 
            
            assert_eq!(cage2.close_syscall(sockfd), 0);
            assert_eq!(cage2.close_syscall(serversockfd), 0);

            assert_eq!(cage2.exit_syscall(), 0);
        });

        //connect to the server
        assert_eq!(cage.connect_syscall(clientsockfd, &socket), 0);

        //send the data with delays so that the server can process the information cleanly
        assert_eq!(cage.send_syscall(clientsockfd, str2cbuf(&"A".repeat(100)), 100, 0), 100);
        interface::sleep(interface::RustDuration::from_millis(100));

        assert_eq!(cage.send_syscall(clientsockfd, str2cbuf(&"A".repeat(100)), 100, 0), 100);
        interface::sleep(interface::RustDuration::from_millis(100));

        assert_eq!(cage.send_syscall(clientsockfd, str2cbuf(&"A".repeat(100)), 100, 0), 100);
        interface::sleep(interface::RustDuration::from_millis(100));

        assert_eq!(cage.send_syscall(clientsockfd, str2cbuf(&"A".repeat(50)), 50, 0), 50);
        interface::sleep(interface::RustDuration::from_millis(100));

        assert_eq!(cage.close_syscall(clientsockfd), 0);

        //connect to the server with the other sockfd
        assert_eq!(cage.connect_syscall(clientsockfd2, &socket), 0);

        //send the data with delays so that the server can process the information cleanly
        assert_eq!(cage.send_syscall(clientsockfd2, str2cbuf(&"A".repeat(100)), 100, 0), 100);
        interface::sleep(interface::RustDuration::from_millis(100));

        assert_eq!(cage.send_syscall(clientsockfd2, str2cbuf(&"A".repeat(100)), 100, 0), 100);
        interface::sleep(interface::RustDuration::from_millis(100));

        assert_eq!(cage.send_syscall(clientsockfd2, str2cbuf(&"A".repeat(100)), 100, 0), 100);
        interface::sleep(interface::RustDuration::from_millis(100));

        assert_eq!(cage.send_syscall(clientsockfd2, str2cbuf(&"A".repeat(50)), 50, 0), 50);
        interface::sleep(interface::RustDuration::from_millis(100));

        assert_eq!(cage.close_syscall(clientsockfd2), 0);
        
        sender.join().unwrap();

        assert_eq!(cage.exit_syscall(), 0);
        lindrustfinalize();
    }



    pub fn ut_lind_net_bind_multiple() {
        lindrustinit();
        let cage = {CAGE_TABLE.read().unwrap().get(&1).unwrap().clone()};

        let mut sockfd = cage.socket_syscall(AF_INET, SOCK_STREAM, 0);
        let socket = interface::GenSockaddr::V4(interface::SockaddrV4{ sin_family: AF_INET as u16, sin_port: 50103u16.to_be(), sin_addr: interface::V4Addr{ s_addr: u32::from_ne_bytes([127, 0, 0, 1]) }, padding: 0}); //127.0.0.1
        assert_eq!(cage.bind_syscall(sockfd, &socket), 0);

        let sockfd2 = cage.socket_syscall(AF_INET, SOCK_STREAM, 0);

        //allowing port reuse
        assert_eq!(cage.setsockopt_syscall(sockfd, SOL_SOCKET, SO_REUSEPORT, 1), 0);
        assert_eq!(cage.setsockopt_syscall(sockfd2, SOL_SOCKET, SO_REUSEPORT, 1), 0);

        assert_eq!(cage.bind_syscall(sockfd2, &socket), 0);

        //double listen should be allowed
        assert_eq!(cage.listen_syscall(sockfd, 1), 0);
        assert_eq!(cage.listen_syscall(sockfd2, 1), 0);

        //UDP bind should be allowed
        sockfd = cage.socket_syscall(AF_INET, SOCK_DGRAM, 0);
        assert_eq!(cage.bind_syscall(sockfd, &socket), 0);

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
        
        assert_eq!(cage.bind_syscall(sockfd, &socket), 0);
        assert_eq!(cage.getsockname_syscall(sockfd, &mut retsocket), 0);
        assert_eq!(retsocket, socket);    

        //checking that we cannot rebind the socket
        assert_eq!(cage.bind_syscall(sockfd, &socket), -(Errno::EINVAL as i32)); //already bound so should fail
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
        let sockaddr = interface::SockaddrV4{ sin_family: AF_INET as u16, sin_port: 53000_u16.to_be(), sin_addr: interface::V4Addr{ s_addr: u32::from_ne_bytes([127, 0, 0, 1]) }, padding: 0};
        let mut socket = interface::GenSockaddr::V4(sockaddr); //127.0.0.1
        assert_eq!(cage.bind_syscall(serversockfd, &socket), 0);
        assert_eq!(cage.listen_syscall(serversockfd, 10), 0);
        
        //forking the cage to get another cage with the same information
        assert_eq!(cage.fork_syscall(2), 0);
        
        let sender = interface::helper_thread(move || {
            let cage2 = {CAGE_TABLE.read().unwrap().get(&2).unwrap().clone()};
            
            interface::sleep(interface::RustDuration::from_millis(100)); 

            let mut socket2 = interface::GenSockaddr::V4(interface::SockaddrV4{ sin_family: AF_INET as u16, sin_port: 53000_u16.to_be(), sin_addr: interface::V4Addr{ s_addr: u32::from_ne_bytes([127, 0, 0, 1]) }, padding: 0}); //127.0.0.1
            assert!(cage2.accept_syscall(serversockfd, &mut socket2) > 0); //really can only make sure that the fd is valid
            
            assert_eq!(cage2.close_syscall(serversockfd), 0);
            assert_eq!(cage2.exit_syscall(), 0);
        });

        assert_eq!(cage.connect_syscall(clientsockfd, &socket), 0); 
        
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

        let serversockfd = cage.socket_syscall(AF_INET, SOCK_STREAM, 0);
        let clientsockfd = cage.socket_syscall(AF_INET, SOCK_STREAM, 0);

        let port: u16 = 53001;
        
        //making sure that the assigned fd's are valid
        assert!(serversockfd > 0);
        assert!(clientsockfd > 0);
        
        //binding to a socket
        let sockaddr = interface::SockaddrV4{ sin_family: AF_INET as u16, sin_port: port.to_be(), sin_addr: interface::V4Addr{ s_addr: u32::from_ne_bytes([127, 0, 0, 1]) }, padding: 0};
        let mut socket = interface::GenSockaddr::V4(sockaddr); //127.0.0.1
        assert_eq!(cage.bind_syscall(serversockfd, &socket), 0);
        assert_eq!(cage.listen_syscall(serversockfd, 1), 0); //we are only allowing for one client at a time
        
        //forking the cage to get another cage with the same information
        assert_eq!(cage.fork_syscall(2), 0);

        //creating a thread for the server so that the information can be sent between the two threads
        let sender = interface::helper_thread(move || {
            
            let cage2 = {CAGE_TABLE.read().unwrap().get(&2).unwrap().clone()};
            interface::sleep(interface::RustDuration::from_millis(100)); 

            let mut socket2 = interface::GenSockaddr::V4(interface::SockaddrV4{ sin_family: AF_INET as u16, sin_port: port.to_be(), sin_addr: interface::V4Addr{ s_addr: u32::from_ne_bytes([127, 0, 0, 1]) }, padding: 0}); //127.0.0.1
            let sockfd = cage2.accept_syscall(serversockfd, &mut socket2); //really can only make sure that the fd is valid
            assert!(sockfd > 0);

            //process the first test...
            //Writing 100, then peek 100, then read 100
            let mut buf = sizecbuf(100);
            assert_eq!(cage2.recvfrom_syscall(sockfd, buf.as_mut_ptr(), 100, MSG_PEEK, &mut Some(&mut socket2)), 100); //peeking at the input message
            buf = sizecbuf(100);
            assert_eq!(cage2.recvfrom_syscall(sockfd, buf.as_mut_ptr(), 100, 0, &mut Some(&mut socket2)), 100);        //reading the input message
            buf = sizecbuf(100);

            interface::sleep(interface::RustDuration::from_millis(200)); 

            //process the second test...
            //Writing 100, read 20, peek 20, read 80
            assert_eq!(cage2.recvfrom_syscall(sockfd, buf.as_mut_ptr(), 20, 0, &mut Some(&mut socket2)), 20);
            buf = sizecbuf(100);
            assert_eq!(cage2.recvfrom_syscall(sockfd, buf.as_mut_ptr(), 20, MSG_PEEK, &mut Some(&mut socket2)), 20);
            buf = sizecbuf(100);
            assert_eq!(cage2.recvfrom_syscall(sockfd, buf.as_mut_ptr(), 80, 0, &mut Some(&mut socket2)), 80);
            buf = sizecbuf(100);

            interface::sleep(interface::RustDuration::from_millis(200)); 

            //process the third test...
            //Writing 100, peek several times, read 100
            for _ in 0..4 { 
                assert_eq!(cage2.recvfrom_syscall(sockfd, buf.as_mut_ptr(), 10, MSG_PEEK, &mut Some(&mut socket2)), 10); 
                buf = sizecbuf(100);
            }
            for _ in 0..4 { 
                assert_eq!(cage2.recvfrom_syscall(sockfd, buf.as_mut_ptr(), 20, MSG_PEEK, &mut Some(&mut socket2)), 20); 
                buf = sizecbuf(100);
            }
            for _ in 0..4 { 
                assert_eq!(cage2.recvfrom_syscall(sockfd, buf.as_mut_ptr(), 30, MSG_PEEK, &mut Some(&mut socket2)), 30); 
                buf = sizecbuf(100);
            }
            for _ in 0..4 { 
                assert_eq!(cage2.recvfrom_syscall(sockfd, buf.as_mut_ptr(), 40, MSG_PEEK, &mut Some(&mut socket2)), 40); 
                buf = sizecbuf(100);
            }
            assert_eq!(cage2.recvfrom_syscall(sockfd, buf.as_mut_ptr(), 100, 0, &mut Some(&mut socket2)), 100);
            buf = sizecbuf(100);

            interface::sleep(interface::RustDuration::from_millis(200)); 

            //process the fourth test...
            //Writing 50, peek 50
            assert_eq!(cage2.recvfrom_syscall(sockfd, buf.as_mut_ptr(), 50, MSG_PEEK, &mut Some(&mut socket2)), 50);
            buf = sizecbuf(100);
            
            interface::sleep(interface::RustDuration::from_millis(100)); 
            
            assert_eq!(cage2.close_syscall(sockfd), 0);
            assert_eq!(cage2.close_syscall(serversockfd), 0);
            assert_eq!(cage2.exit_syscall(), 0);
        });

        //connect to the server
        assert_eq!(cage.connect_syscall(clientsockfd, &socket), 0);

        //send the data with delays so that the server can process the information cleanly
        assert_eq!(cage.send_syscall(clientsockfd, str2cbuf(&"A".repeat(100)), 100, 0), 100);
        interface::sleep(interface::RustDuration::from_millis(100));

        assert_eq!(cage.send_syscall(clientsockfd, str2cbuf(&"A".repeat(100)), 100, 0), 100);
        interface::sleep(interface::RustDuration::from_millis(100));

        assert_eq!(cage.send_syscall(clientsockfd, str2cbuf(&"A".repeat(100)), 100, 0), 100);
        interface::sleep(interface::RustDuration::from_millis(100));

        assert_eq!(cage.send_syscall(clientsockfd, str2cbuf(&"A".repeat(50)), 50, 0), 50);
        interface::sleep(interface::RustDuration::from_millis(100));
        
        sender.join().unwrap();

        assert_eq!(cage.exit_syscall(), 0);
        lindrustfinalize();
    }
    
    

    pub fn ut_lind_net_shutdown() {
        lindrustinit();
        let cage = {CAGE_TABLE.read().unwrap().get(&1).unwrap().clone()};
        
        let serversockfd = cage.socket_syscall(AF_INET, SOCK_STREAM, 0);
        let clientsockfd = cage.socket_syscall(AF_INET, SOCK_STREAM, 0);
        
        assert!(serversockfd > 0);
        assert!(clientsockfd > 0);
        
        //binding to a socket
        let sockaddr = interface::SockaddrV4{ sin_family: AF_INET as u16, sin_port: 50431_u16.to_be(), sin_addr: interface::V4Addr{ s_addr: u32::from_ne_bytes([127, 0, 0, 1]) }, padding: 0};
        let mut socket = interface::GenSockaddr::V4(sockaddr); //127.0.0.1
        assert_eq!(cage.bind_syscall(serversockfd, &socket), 0);
        assert_eq!(cage.listen_syscall(serversockfd, 10), 0);
        
        //forking the cage to get another cage with the same information
        assert_eq!(cage.fork_syscall(2), 0);

        let sender = interface::helper_thread(move || {
            let cage2 = {CAGE_TABLE.read().unwrap().get(&2).unwrap().clone()};
            
            interface::sleep(interface::RustDuration::from_millis(100)); 

            let mut socket2 = interface::GenSockaddr::V4(interface::SockaddrV4{ sin_family: AF_INET as u16, sin_port: 50431_u16.to_be(), sin_addr: interface::V4Addr{ s_addr: u32::from_ne_bytes([127, 0, 0, 1]) }, padding: 0}); //127.0.0.1
            let fd = cage2.accept_syscall(serversockfd, &mut socket2); 
            assert!(fd > 0);
            
            assert_eq!(cage2.send_syscall(fd, str2cbuf("random string"), 13, 0), 13);
            assert_eq!(cage2.netshutdown_syscall(fd, SHUT_RDWR), 0);
            assert_ne!(cage2.netshutdown_syscall(fd, SHUT_RDWR), 0); //should fail

            assert_eq!(cage2.close_syscall(serversockfd), 0);
            assert_eq!(cage2.exit_syscall(), 0);
        });

        assert_eq!(cage.connect_syscall(clientsockfd, &socket), 0);
        
        let mut retsocket = interface::GenSockaddr::V4(interface::SockaddrV4::default());
        assert_eq!(cage.getsockname_syscall(clientsockfd, &mut retsocket), 0);
        assert_ne!(retsocket, socket);
        
        sender.join().unwrap();

        assert_eq!(cage.exit_syscall(), 0);
        lindrustfinalize();
    }



    pub fn ut_lind_net_socket() {
        lindrustinit();
        let cage = {CAGE_TABLE.read().unwrap().get(&1).unwrap().clone()};

        let mut sockfd = cage.socket_syscall(AF_INET, SOCK_STREAM, 0);
        let sockfd2 = cage.socket_syscall(AF_INET, SOCK_STREAM, IPPROTO_TCP);

        let sockfd3 = cage.socket_syscall(AF_INET, SOCK_DGRAM, 0);
        let sockfd4 = cage.socket_syscall(AF_INET, SOCK_DGRAM, IPPROTO_UDP);

        //checking that the fd's are correct
        assert!(sockfd > 0);
        assert!(sockfd2 > 0);
        assert!(sockfd3 > 0);
        assert!(sockfd4 > 0);

        //let's check an illegal operation...
        let sockfdillegal = cage.socket_syscall(AF_UNIX, SOCK_DGRAM, 0);
        assert_eq!(sockfdillegal, -(Errno::EOPNOTSUPP as i32));

        sockfd = cage.socket_syscall(AF_INET, SOCK_STREAM, 0);
        assert!(sockfd > 0);

        assert_eq!(cage.close_syscall(sockfd), 0);
        assert_eq!(cage.exit_syscall(), 0);
        lindrustfinalize();
    }



    pub fn ut_lind_net_socketoptions() {
        lindrustinit();
        let cage = {CAGE_TABLE.read().unwrap().get(&1).unwrap().clone()};

        let sockfd = cage.socket_syscall(AF_INET, SOCK_STREAM, 0);
        assert!(sockfd > 0);

        let sockaddr = interface::SockaddrV4{ sin_family: AF_INET as u16, sin_port: 50115_u16.to_be(), sin_addr: interface::V4Addr{ s_addr: u32::from_ne_bytes([127, 0, 0, 1]) }, padding: 0};
        let mut socket = interface::GenSockaddr::V4(sockaddr); //127.0.0.1
        assert_eq!(cage.bind_syscall(sockfd, &socket), 0);
        assert_eq!(cage.listen_syscall(sockfd, 4), 0);

        //set and get some options:
        assert_eq!(cage.getsockopt_syscall(sockfd, SOL_SOCKET, SO_REUSEPORT), 0);
        assert_eq!(cage.getsockopt_syscall(sockfd, SOL_SOCKET, SO_LINGER), 0);
        assert_eq!(cage.getsockopt_syscall(sockfd, SOL_SOCKET, SO_KEEPALIVE), 0);

        //linger...
        assert_eq!(cage.getsockopt_syscall(sockfd, SOL_SOCKET, SO_LINGER), 0);
        assert_eq!(cage.setsockopt_syscall(sockfd, SOL_SOCKET, SO_LINGER, 1), 0);
        assert_eq!(cage.getsockopt_syscall(sockfd, SOL_SOCKET, SO_LINGER), 1);

        //check the options
        assert_eq!(cage.getsockopt_syscall(sockfd, SOL_SOCKET, SO_REUSEPORT), 0);
        assert_eq!(cage.getsockopt_syscall(sockfd, SOL_SOCKET, SO_LINGER), 1);
        assert_eq!(cage.getsockopt_syscall(sockfd, SOL_SOCKET, SO_KEEPALIVE), 0);

        //reuseport...
        assert_eq!(cage.getsockopt_syscall(sockfd, SOL_SOCKET, SO_REUSEPORT), 0);
        assert_eq!(cage.setsockopt_syscall(sockfd, SOL_SOCKET, SO_REUSEPORT, 1), 0);
        assert_eq!(cage.getsockopt_syscall(sockfd, SOL_SOCKET, SO_REUSEPORT), 1);

        //check the options
        assert_eq!(cage.getsockopt_syscall(sockfd, SOL_SOCKET, SO_REUSEPORT), 1);
        assert_eq!(cage.getsockopt_syscall(sockfd, SOL_SOCKET, SO_LINGER), 1);
        assert_eq!(cage.getsockopt_syscall(sockfd, SOL_SOCKET, SO_KEEPALIVE), 0);

        //keep alive...
        assert_eq!(cage.getsockopt_syscall(sockfd, SOL_SOCKET, SO_KEEPALIVE), 0);
        assert_eq!(cage.setsockopt_syscall(sockfd, SOL_SOCKET, SO_KEEPALIVE, 1), 0);
        
        //check the options
        assert_eq!(cage.getsockopt_syscall(sockfd, SOL_SOCKET, SO_REUSEPORT), 1);
        assert_eq!(cage.getsockopt_syscall(sockfd, SOL_SOCKET, SO_LINGER), 1);
        assert_eq!(cage.getsockopt_syscall(sockfd, SOL_SOCKET, SO_KEEPALIVE), 1);
        
        assert_eq!(cage.setsockopt_syscall(sockfd, SOL_SOCKET, SO_SNDBUF, 1000), 0);
        assert_eq!(cage.getsockopt_syscall(sockfd, SOL_SOCKET, SO_SNDBUF), 1000);
        
        assert_eq!(cage.setsockopt_syscall(sockfd, SOL_SOCKET, SO_RCVBUF, 2000), 0);
        assert_eq!(cage.getsockopt_syscall(sockfd, SOL_SOCKET, SO_RCVBUF), 2000);
        
        //check the options
        assert_eq!(cage.getsockopt_syscall(sockfd, SOL_SOCKET, SO_REUSEPORT), 1);
        assert_eq!(cage.getsockopt_syscall(sockfd, SOL_SOCKET, SO_LINGER), 1);
        assert_eq!(cage.getsockopt_syscall(sockfd, SOL_SOCKET, SO_KEEPALIVE), 1);
        
        assert_eq!(cage.exit_syscall(), 0);
        lindrustfinalize();
    }



    pub fn ut_lind_net_udp_bad_bind() {
        lindrustinit();
        let cage = {CAGE_TABLE.read().unwrap().get(&1).unwrap().clone()};

        let sockfd = cage.socket_syscall(AF_INET, SOCK_DGRAM, 0);
        assert!(sockfd > 0); //checking that the sockfd is valid

        let sockaddr = interface::SockaddrV4{ sin_family: AF_INET as u16, sin_port: 50116_u16.to_be(), sin_addr: interface::V4Addr{ s_addr: u32::from_ne_bytes([127, 0, 0, 1]) }, padding: 0};
        let mut socket = interface::GenSockaddr::V4(sockaddr); //127.0.0.1

        let sockaddr2 = interface::SockaddrV4{ sin_family: AF_INET as u16, sin_port: 50303_u16.to_be(), sin_addr: interface::V4Addr{ s_addr: u32::from_ne_bytes([127, 0, 0, 1]) }, padding: 0};
        let mut socket2 = interface::GenSockaddr::V4(sockaddr); //127.0.0.1

        assert_eq!(cage.bind_syscall(sockfd, &socket), 0);
        assert_eq!(cage.connect_syscall(sockfd, &socket2), 0);

        //now the bind should fail...
        assert_ne!(cage.bind_syscall(sockfd, &socket), 0);

        assert_eq!(cage.exit_syscall(), 0);
        lindrustfinalize();
    }



    pub fn ut_lind_net_udp_simple() {
        lindrustinit();
        let cage = {CAGE_TABLE.read().unwrap().get(&1).unwrap().clone()};

        //just going to test the basic connect with UDP now...
        let serverfd = cage.socket_syscall(AF_INET, SOCK_DGRAM, 0);
        let clientfd = cage.socket_syscall(AF_INET, SOCK_DGRAM, 0);

        let mut sendsockaddr = interface::SockaddrV4{ sin_family: AF_INET as u16, sin_port: 50121_u16.to_be(), sin_addr: interface::V4Addr{ s_addr: u32::from_ne_bytes([127, 0, 0, 1]) }, padding: 0};
        let send_socket = interface::GenSockaddr::V4(sendsockaddr);
        let socket = send_socket.clone();

        assert!(serverfd > 0);
        assert!(clientfd > 0);

        //forking the cage to get another cage with the same information
        assert_eq!(cage.fork_syscall(2), 0);

        let sender = interface::helper_thread(move || {
            let cage2 = {CAGE_TABLE.read().unwrap().get(&2).unwrap().clone()};

            assert_eq!(cage2.bind_syscall(serverfd, &socket), 0);
            
            interface::sleep(interface::RustDuration::from_millis(50)); 
            let mut buf = sizecbuf(10);
            cage2.recv_syscall(serverfd, buf.as_mut_ptr(), 10, 0);
            assert_eq!(cbuf2str(&buf), "test".to_owned() + &"\0".repeat(6));

            interface::sleep(interface::RustDuration::from_millis(50)); 
            assert_eq!(cage2.recv_syscall(serverfd, buf.as_mut_ptr(), 10, 0), 0);
            assert_eq!(cbuf2str(&buf), "test2".to_owned() + &"\0".repeat(5));

            assert_eq!(cage2.close_syscall(serverfd), 0);
            assert_eq!(cage2.exit_syscall(), 0);
        });
        
        let mut buf2 = str2cbuf("test");
        assert_eq!(cage.sendto_syscall(clientfd, buf2, 10, 0, &send_socket), 10);
        panic!();
        let sendsockfd2 = cage.socket_syscall(AF_INET, SOCK_DGRAM, 0);
        assert!(sendsockfd2 > 0);

        let sockaddr2 = interface::SockaddrV4{ sin_family: AF_INET as u16, sin_port: 50992_u16.to_be(), sin_addr: interface::V4Addr{ s_addr: u32::from_ne_bytes([127, 0, 0, 1]) }, padding: 0};
        let mut socket2 = interface::GenSockaddr::V4(sockaddr2); //127.0.0.1

        interface::sleep(interface::RustDuration::from_millis(100)); 

        buf2 = str2cbuf("test2");
        assert_eq!(cage.bind_syscall(sendsockfd2, &socket2), 0);
        assert_eq!(cage.sendto_syscall(sendsockfd2, buf2, 10, 0, &send_socket), 10);

        interface::sleep(interface::RustDuration::from_millis(100)); 
        sender.join().unwrap();

        assert_eq!(cage.close_syscall(sendsockfd2), 0);
        assert_eq!(cage.close_syscall(clientfd), 0);
        assert_eq!(cage.exit_syscall(), 0);
        lindrustfinalize();
    }


    pub fn ut_lind_net_udp_connect() {
        lindrustinit();
        let cage = {CAGE_TABLE.read().unwrap().get(&1).unwrap().clone()};

        //getting the sockets set up...
        let listenfd = cage.socket_syscall(AF_INET, SOCK_DGRAM, 0);
        let sendfd = cage.socket_syscall(AF_INET, SOCK_DGRAM, 0);
        let sockaddr = interface::SockaddrV4{ sin_family: AF_INET as u16, sin_port: 50121_u16.to_be(), sin_addr: interface::V4Addr{ s_addr: u32::from_ne_bytes([127, 0, 0, 1]) }, padding: 0};
        let mut socket = interface::GenSockaddr::V4(sockaddr); //127.0.0.1
        let mut socket_clone = socket.clone();

        assert!(listenfd > 0);
        assert!(sendfd > 0);

        assert_eq!(cage.bind_syscall(listenfd, &socket), 0);

        //forking the cage to get another cage with the same information
        assert_eq!(cage.fork_syscall(2), 0);

        let sender = interface::helper_thread(move || {

            let cage2 = {CAGE_TABLE.read().unwrap().get(&2).unwrap().clone()};
            interface::sleep(interface::RustDuration::from_millis(100)); 

            let mut buf = sizecbuf(16);
            assert_eq!(cage2.recvfrom_syscall(listenfd, buf.as_mut_ptr(), 16, 0, &mut Some(&mut socket_clone)), 16);
            assert_eq!(cbuf2str(&buf), "UDP Connect Test");

            assert_eq!(cage2.close_syscall(listenfd), 0);
            assert_eq!(cage2.exit_syscall(), 0);
        });

        assert_eq!(cage.connect_syscall(sendfd, &socket), 0);
        assert_eq!(cage.send_syscall(sendfd, str2cbuf("UDP Connect Test"), 16, 0), 16); 

        sender.join().unwrap();

        assert_eq!(cage.close_syscall(sendfd), 0);
        assert_eq!(cage.exit_syscall(), 0);
        lindrustfinalize();
    }
}
