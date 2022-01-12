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
        ut_lind_net_poll(); //WIP -- done, just need to check it on something that's not a Mac
        ut_lind_net_recvfrom();
        ut_lind_net_select(); //WIP
        ut_lind_net_shutdown();
        ut_lind_net_socket();
        ut_lind_net_socketoptions();
        ut_lind_net_socketpair(); //WIP -- done, just need to check it on something that's not a Mac
        ut_lind_net_udp_bad_bind();
        ut_lind_net_udp_simple();
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
        let thread = interface::helper_thread(move || {
            
            let cage2 = {CAGE_TABLE.read().unwrap().get(&2).unwrap().clone()};
            let mut socket2 = interface::GenSockaddr::V4(interface::SockaddrV4{ sin_family: AF_INET as u16, sin_port: port.to_be(), sin_addr: interface::V4Addr{ s_addr: 0 }, padding: 0}); //0.0.0.0

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
        interface::sleep(interface::RustDuration::from_millis(20));

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
        
        thread.join().unwrap();

        assert_eq!(cage.exit_syscall(), 0);
        lindrustfinalize();
    }



    pub fn ut_lind_net_bind_multiple() {
        lindrustinit();
        let cage = {CAGE_TABLE.read().unwrap().get(&1).unwrap().clone()};

        let mut sockfd = cage.socket_syscall(AF_INET, SOCK_STREAM, 0);
        let socket = interface::GenSockaddr::V4(interface::SockaddrV4{ sin_family: AF_INET as u16, sin_port: 50103u16.to_be(), sin_addr: interface::V4Addr{ s_addr: u32::from_ne_bytes([127, 0, 0, 1]) }, padding: 0}); //127.0.0.1
        assert_eq!(cage.setsockopt_syscall(sockfd, SOL_SOCKET, SO_REUSEPORT, 1), 0);
        assert_eq!(cage.bind_syscall(sockfd, &socket), 0);

        let sockfd2 = cage.socket_syscall(AF_INET, SOCK_STREAM, 0);

        //allowing port reuse
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
        let sockaddr = interface::SockaddrV4{ sin_family: AF_INET as u16, sin_port: 53003_u16.to_be(), sin_addr: interface::V4Addr{ s_addr: u32::from_ne_bytes([127, 0, 0, 1]) }, padding: 0};
        let mut socket = interface::GenSockaddr::V4(sockaddr); //127.0.0.1
        assert_eq!(cage.bind_syscall(serversockfd, &socket), 0);
        assert_eq!(cage.listen_syscall(serversockfd, 10), 0);
        
        //forking the cage to get another cage with the same information
        assert_eq!(cage.fork_syscall(2), 0);
        
        let thread = interface::helper_thread(move || {
            let cage2 = {CAGE_TABLE.read().unwrap().get(&2).unwrap().clone()};
            let mut socket2 = interface::GenSockaddr::V4(interface::SockaddrV4::default());
            assert!(cage2.accept_syscall(serversockfd, &mut socket2) > 0); //really can only make sure that the fd is valid
            
            assert_eq!(cage2.close_syscall(serversockfd), 0);
            assert_eq!(cage2.exit_syscall(), 0);
        });

        interface::sleep(interface::RustDuration::from_millis(100));
        assert_eq!(cage.connect_syscall(clientsockfd, &socket), 0); 
        
        let mut retsocket = interface::GenSockaddr::V4(interface::SockaddrV4::default());
        assert_eq!(cage.getsockname_syscall(clientsockfd, &mut retsocket), 0);
        assert_ne!(retsocket, socket);

        assert_eq!(cage.close_syscall(serversockfd), 0);
        
        thread.join().unwrap();
        
        assert_eq!(cage.exit_syscall(), 0);
        lindrustfinalize();
    }



    pub fn ut_lind_net_poll() {
        lindrustinit();
        let cage = {CAGE_TABLE.read().unwrap().get(&1).unwrap().clone()};

        let filefd = cage.open_syscall("/netpolltest.txt", O_CREAT | O_EXCL | O_RDWR, S_IRWXA);
        assert!(filefd > 0);

        let serversockfd = cage.socket_syscall(AF_INET, SOCK_STREAM, 0);
        let clientsockfd1 = cage.socket_syscall(AF_INET, SOCK_STREAM, 0);
        let clientsockfd2 = cage.socket_syscall(AF_INET, SOCK_STREAM, 0);

        let port: u16 = 53001;
        let sockaddr = interface::SockaddrV4{ sin_family: AF_INET as u16, sin_port: port.to_be(), sin_addr: interface::V4Addr{ s_addr: u32::from_ne_bytes([127, 0, 0, 1]) }, padding: 0};
        let mut socket = interface::GenSockaddr::V4(sockaddr); //127.0.0.1 from bytes above
        assert_eq!(cage.bind_syscall(serversockfd, &socket), 0);
        assert_eq!(cage.listen_syscall(serversockfd, 4), 0); 

        let mut serverpoll = interface::PollStruct{fd: serversockfd, events: POLLIN, revents: 0};
        let mut filepoll = interface::PollStruct{fd: filefd, events: POLLIN, revents: 0};
        let mut polled = vec![serverpoll, filepoll];

        //client 1 connects to the server to send and recv data...
        let thread1 = interface::helper_thread(move || {
            interface::sleep(interface::RustDuration::from_millis(100));
            let cage1 = {CAGE_TABLE.read().unwrap().get(&2).unwrap().clone()};

            assert_eq!(cage1.connect_syscall(clientsockfd1, &socket), 0);
            assert_eq!(cage1.send_syscall(clientsockfd1, str2cbuf(&"test"), 4, 0), 4);
            //giving it a longer pause time to that it can process all of the data that it is recieving
            interface::sleep(interface::RustDuration::from_millis(1000));

            assert_eq!(cage1.close_syscall(serversockfd), 0);
            assert_eq!(cage1.exit_syscall(), 0);
        });

        //client 2 connects to the server to send and recv data...
        let thread2 = interface::helper_thread(move || {
            //give it a longer time so that it can sufficiently process all of the data
            interface::sleep(interface::RustDuration::from_millis(200));
            let cage2 = {CAGE_TABLE.read().unwrap().get(&2).unwrap().clone()};

            assert_eq!(cage2.connect_syscall(clientsockfd2, &socket), 0);
            assert_eq!(cage2.send_syscall(clientsockfd2, str2cbuf(&"test"), 4, 0), 4);

            interface::sleep(interface::RustDuration::from_millis(1000));

            assert_eq!(cage2.close_syscall(serversockfd), 0);
            assert_eq!(cage2.exit_syscall(), 0);
        });
            
        //acting as the server and processing the request
        let thread3 = interface::helper_thread(move || {
            let mut infds: Vec<i32>;
            let mut outfds: Vec<i32>;
            let mut counter = 0; // this is to make sure that the loop doesn't go on forever
            loop {
                counter = counter + 1;
                if counter == 600 { break; }

                //start a while true loop for processing requests
                let pollretvalue = cage.poll_syscall(&mut polled.as_mut_slice(), Some(interface::RustDuration::ZERO));
                assert!(pollretvalue > 0);
                
                infds = vec![];
                outfds = vec![];

                for polledfile in &mut *polled {
                    if polledfile.revents & POLLIN == 0 {
                        infds.push(polledfile.fd);
                    } 
                    if polledfile.revents & POLLOUT == 0 {
                        outfds.push(polledfile.fd);
                    } 
                }
                
                //check for any activity in the input sockets
                for sockfd in infds {
                    //If the socket returned was listerner socket, then there's a new connection
                    //so we accept it, and put the client socket in the list of inputs.
                    if sockfd == serversockfd {
                        let port: u16 = 53001;
                        let sockaddr = interface::SockaddrV4{ sin_family: AF_INET as u16, sin_port: port.to_be(), sin_addr: interface::V4Addr{ s_addr: u32::from_ne_bytes([127, 0, 0, 1]) }, padding: 0};
                        let mut addr = interface::GenSockaddr::V4(sockaddr); //127.0.0.1 from bytes above
                        
                        let newsockfd = cage.accept_syscall(sockfd, &mut addr);
                        polled.push(interface::PollStruct{fd: newsockfd, events: POLLIN, revents: 0})
                    } else if sockfd == filefd { //Write to a file...
                        assert_eq!(cage.write_syscall(sockfd, str2cbuf("test"), 4), 4);
                        assert_eq!(cage.lseek_syscall(sockfd, 0, SEEK_SET), 0);
                        //Once the write is successful into a file, modify the file descriptor so that its ready for reading out of the file.
                        for polledfile in &mut *polled {
                            if polledfile.fd == sockfd {
                                polledfile.events = POLLOUT;
                            }
                        }
                    } else { //If the socket is in established conn., then we recv the data. If there's no data, then close the client socket.
                    let mut buf = sizecbuf(4);
                    assert_eq!(cage.recv_syscall(sockfd, buf.as_mut_ptr(), 4, 0), 4);
                        if cbuf2str(&buf) == "test" {
                            //This socket is ready for writing, modify the socket descriptor to be in read-write mode. This socket can write data out to network 
                            for polledfile in &mut *polled {
                                if polledfile.fd == sockfd {
                                    polledfile.events = POLLIN | POLLOUT;
                                }
                            }
                        } else { //No data means remote socket closed, hence close the client socket in server, also remove this socket from polling. 
                            assert_eq!(cage.close_syscall(sockfd), 0);
                            polled.retain(|x| x.fd != sockfd);
                        }
                    }
                }

                for sockfd in outfds {
                    if sockfd == filefd {
                        let mut read_buf1 = sizecbuf(4);
                        assert_eq!(cage.read_syscall(sockfd, read_buf1.as_mut_ptr(), 4), 4);
                        assert_eq!(cbuf2str(&read_buf1), "test");
                        //test for file finished, remove from polling.
                        polled.retain(|x| x.fd != sockfd);
                    } else { //Data is sent out of this socket, it's no longer ready for writing, modify it only read mode.
                        assert_eq!(cage.send_syscall(sockfd, str2cbuf(&"test"), 4, 0), 4);
                        for polledfile in &mut *polled {
                            if polledfile.fd == sockfd {
                                polledfile.events = POLLIN;
                            }
                        }
                    }
                }
            }
            assert_eq!(cage.close_syscall(serversockfd), 0);
            assert_eq!(cage.exit_syscall(), 0);
        });  

        thread1.join().unwrap();
        thread2.join().unwrap();
        thread3.join().unwrap();

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
        let thread = interface::helper_thread(move || {
            
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
        
        thread.join().unwrap();

        assert_eq!(cage.exit_syscall(), 0);
        lindrustfinalize();
    }



    pub fn ut_lind_net_select () {
        lindrustinit();
        let cage = {CAGE_TABLE.read().unwrap().get(&1).unwrap().clone()};

        let filefd = cage.open_syscall("/netselecttest.txt", O_CREAT | O_EXCL | O_RDWR, S_IRWXA);
        assert!(filefd > 0);

        let serversockfd = cage.socket_syscall(AF_INET, SOCK_STREAM, 0);
        let clientsockfd1 = cage.socket_syscall(AF_INET, SOCK_STREAM, 0);
        let clientsockfd2 = cage.socket_syscall(AF_INET, SOCK_STREAM, 0);

        let port: u16 = 53001;
        let sockaddr = interface::SockaddrV4{ sin_family: AF_INET as u16, sin_port: port.to_be(), sin_addr: interface::V4Addr{ s_addr: u32::from_ne_bytes([127, 0, 0, 1]) }, padding: 0};
        let mut socket = interface::GenSockaddr::V4(sockaddr); //127.0.0.1 from bytes above
        assert_eq!(cage.bind_syscall(serversockfd, &socket), 0);
        assert_eq!(cage.listen_syscall(serversockfd, 4), 0); 

        let mut inputs = interface::RustHashSet::<i32>::new();
        let mut outputs = interface::RustHashSet::<i32>::new();
        let mut excepts = interface::RustHashSet::<i32>::new();

        inputs.insert(serversockfd);
        inputs.insert(filefd);
        outputs.insert(filefd);

        //client 1 connects to the server to send and recv data...
        let threadclient1 = interface::helper_thread(move || {
            interface::sleep(interface::RustDuration::from_millis(100));
            let cage1 = {CAGE_TABLE.read().unwrap().get(&2).unwrap().clone()};

            assert_eq!(cage1.connect_syscall(clientsockfd1, &socket), 0);
            assert_eq!(cage1.send_syscall(clientsockfd1, str2cbuf(&"test"), 4, 0), 4);
            
            //giving it a longer pause time to that it can process all of the data that it is recieving
            interface::sleep(interface::RustDuration::from_millis(1000));

            let mut buf = sizecbuf(4);
            assert_eq!(cage1.recv_syscall(clientsockfd1, buf.as_mut_ptr(), 4, 0), 4);
            assert_eq!(cbuf2str(&buf), "test");

            assert_eq!(cage1.close_syscall(clientsockfd1), 0);
            assert_eq!(cage1.exit_syscall(), 0);
        });

        //client 2 connects to the server to send and recv data...
        let threadclient2 = interface::helper_thread(move || {
            interface::sleep(interface::RustDuration::from_millis(100));
            let cage2 = {CAGE_TABLE.read().unwrap().get(&2).unwrap().clone()};

            assert_eq!(cage2.connect_syscall(clientsockfd2, &socket), 0);
            assert_eq!(cage2.send_syscall(clientsockfd2, str2cbuf(&"test"), 4, 0), 4);
            
            //giving it a longer pause time to that it can process all of the data that it is recieving
            interface::sleep(interface::RustDuration::from_millis(1000));

            let mut buf = sizecbuf(4);
            assert_eq!(cage2.recv_syscall(clientsockfd2, buf.as_mut_ptr(), 4, 0), 4);
            assert_eq!(cbuf2str(&buf), "test");

            assert_eq!(cage2.close_syscall(clientsockfd2), 0);
            assert_eq!(cage2.exit_syscall(), 0);
        });

        //acting as the server and processing the request
        let mut counter = 0;
        loop {
            if counter > 600 { break; }
            counter += 1;
            assert!(cage.select_syscall(11, &mut inputs, &mut outputs, &mut excepts, Some(interface::RustDuration::ZERO)) > 0);
            
            //Check for any activity in any of the Input sockets...
            let mut insocks_add: Vec<i32> = vec![];
            let mut insocks_remove: Vec<i32> = vec![];
            for sock in &inputs {
                //If the socket returned was listerner socket, then there's a new conn., so we accept it, and put the client socket in the list of Inputs.
                if sock == &serversockfd {
                    let mut sockfd = cage.accept_syscall(*sock, &mut socket); //really can only make sure that the fd is valid
                    assert!(sockfd > 0);
                    insocks_add.push(sockfd);
                } else if sock == &filefd {
                    //Write to a file... 
                    assert_eq!(cage.write_syscall(*sock, str2cbuf("test"), 4), 4);
                    assert_eq!(cage.lseek_syscall(*sock, 0, SEEK_SET), 0);
                    insocks_remove.push(sock.clone());
                } else { //If the socket is in established conn., then we recv the data. If there's no data, then close the client socket.
                    let mut buf = sizecbuf(4);
                    assert_eq!(cage.recv_syscall(*sock, buf.as_mut_ptr(), 4, 0), 4);
                    if cbuf2str(&buf) == "test" { //We make the ouput ready, so that it sends out data... 
                        outputs.insert(*sock);
                    } else { //No data means remote socket closed, hence close the client socket in server, also remove this socket from readfd's.
                        assert_eq!(cage.close_syscall(*sock), 0);
                        insocks_remove.push(sock.clone());
                    }
                }
            }
            for sock in insocks_add {
                inputs.insert(sock);
            }
            for sock in insocks_remove {
                inputs.remove(&sock);
            }

            let mut remsocks: Vec<i32> = vec![];
            for sock in &outputs {
                if sock == &filefd {
                    let mut buf = sizecbuf(4);
                    assert_eq!(cage.read_syscall(*sock, buf.as_mut_ptr(), 4), 4);
                    assert_eq!(cbuf2str(&buf), "test");
                    remsocks.push(sock.clone()); //test for file finished, remove from monitoring.
                } else { //Data is sent out this socket, it's no longer ready for writing remove this socket from writefd's. 
                    interface::sleep(interface::RustDuration::from_millis(30));
                    assert_eq!(cage.send_syscall(*sock, str2cbuf("test"), 4, 0), 4);
                    remsocks.push(sock.clone());
                }
            }

            for sock in remsocks {
                outputs.remove(&sock);
            }
        }
        assert_eq!(cage.close_syscall(serversockfd), 0);

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

        let thread = interface::helper_thread(move || {
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
        
        thread.join().unwrap();

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
        let mut optstore = -12;
        assert_eq!(cage.getsockopt_syscall(sockfd, SOL_SOCKET, SO_REUSEPORT, &mut optstore), 0);
        assert_eq!(optstore, 0);
        assert_eq!(cage.getsockopt_syscall(sockfd, SOL_SOCKET, SO_LINGER, &mut optstore), 0);
        assert_eq!(optstore, 0);
        assert_eq!(cage.getsockopt_syscall(sockfd, SOL_SOCKET, SO_KEEPALIVE, &mut optstore), 0);
        assert_eq!(optstore, 0);

        //linger...
        assert_eq!(cage.getsockopt_syscall(sockfd, SOL_SOCKET, SO_LINGER, &mut optstore), 0);
        assert_eq!(optstore, 0);
        assert_eq!(cage.setsockopt_syscall(sockfd, SOL_SOCKET, SO_LINGER, 1), 0);
        assert_eq!(cage.getsockopt_syscall(sockfd, SOL_SOCKET, SO_LINGER, &mut optstore), 0);
        assert_eq!(optstore, 1);

        //check the options
        assert_eq!(cage.getsockopt_syscall(sockfd, SOL_SOCKET, SO_REUSEPORT, &mut optstore), 0);
        assert_eq!(optstore, 0);
        assert_eq!(cage.getsockopt_syscall(sockfd, SOL_SOCKET, SO_LINGER, &mut optstore), 0);
        assert_eq!(optstore, 1);
        assert_eq!(cage.getsockopt_syscall(sockfd, SOL_SOCKET, SO_KEEPALIVE, &mut optstore), 0);
        assert_eq!(optstore, 0);

        //reuseport...
        assert_eq!(cage.getsockopt_syscall(sockfd, SOL_SOCKET, SO_REUSEPORT, &mut optstore), 0);
        assert_eq!(optstore, 0);
        assert_eq!(cage.setsockopt_syscall(sockfd, SOL_SOCKET, SO_REUSEPORT, 1), 0);
        assert_eq!(cage.getsockopt_syscall(sockfd, SOL_SOCKET, SO_REUSEPORT, &mut optstore), 0);
        assert_eq!(optstore, 1);

        //check the options
        assert_eq!(cage.getsockopt_syscall(sockfd, SOL_SOCKET, SO_REUSEPORT, &mut optstore), 0);
        assert_eq!(optstore, 1);
        assert_eq!(cage.getsockopt_syscall(sockfd, SOL_SOCKET, SO_LINGER, &mut optstore), 0);
        assert_eq!(optstore, 1);
        assert_eq!(cage.getsockopt_syscall(sockfd, SOL_SOCKET, SO_KEEPALIVE, &mut optstore), 0);
        assert_eq!(optstore, 0);

        //keep alive...
        assert_eq!(cage.getsockopt_syscall(sockfd, SOL_SOCKET, SO_KEEPALIVE, &mut optstore), 0);
        assert_eq!(optstore, 0);
        assert_eq!(cage.setsockopt_syscall(sockfd, SOL_SOCKET, SO_KEEPALIVE, 1), 0);
        
        //check the options
        assert_eq!(cage.getsockopt_syscall(sockfd, SOL_SOCKET, SO_REUSEPORT, &mut optstore), 0);
        assert_eq!(optstore, 1);
        assert_eq!(cage.getsockopt_syscall(sockfd, SOL_SOCKET, SO_LINGER, &mut optstore), 0);
        assert_eq!(optstore, 1);
        assert_eq!(cage.getsockopt_syscall(sockfd, SOL_SOCKET, SO_KEEPALIVE, &mut optstore), 0);
        assert_eq!(optstore, 1);
        
        assert_eq!(cage.setsockopt_syscall(sockfd, SOL_SOCKET, SO_SNDBUF, 1000), 0);
        assert_eq!(cage.getsockopt_syscall(sockfd, SOL_SOCKET, SO_SNDBUF, &mut optstore), 0);
        assert_eq!(optstore, 1000);
        
        assert_eq!(cage.setsockopt_syscall(sockfd, SOL_SOCKET, SO_RCVBUF, 2000), 0);
        assert_eq!(cage.getsockopt_syscall(sockfd, SOL_SOCKET, SO_RCVBUF, &mut optstore), 0);
        assert_eq!(optstore, 2000);
        
        //check the options
        assert_eq!(cage.getsockopt_syscall(sockfd, SOL_SOCKET, SO_REUSEPORT, &mut optstore), 0);
        assert_eq!(optstore, 1);
        assert_eq!(cage.getsockopt_syscall(sockfd, SOL_SOCKET, SO_LINGER, &mut optstore), 0);
        assert_eq!(optstore, 1);
        assert_eq!(cage.getsockopt_syscall(sockfd, SOL_SOCKET, SO_KEEPALIVE, &mut optstore), 0);
        assert_eq!(optstore, 1);
        
        assert_eq!(cage.exit_syscall(), 0);
        lindrustfinalize();
    }



    pub fn ut_lind_net_socketpair() {
        lindrustinit();
        let cage = {CAGE_TABLE.read().unwrap().get(&1).unwrap().clone()};
        let mut socketpair = interface::SockPair::default();
        assert_eq!(Cage::socketpair_syscall(cage.clone(), AF_INET, SOCK_STREAM, 0, &mut socketpair), 0);
        let arc_cage = interface::RustRfc::new(cage.clone());

        let thread = interface::helper_thread(move || {
            let cage2 = {CAGE_TABLE.read().unwrap().get(&2).unwrap().clone()};
            
            let mut buf = sizecbuf(10);
            cage2.recv_syscall(socketpair.sock2, buf.as_mut_ptr(), 10, 0);
            assert_eq!(cbuf2str(&buf), "test\0\0\0\0\0\0");
            
            interface::sleep(interface::RustDuration::from_millis(30));
            assert_eq!(cage2.send_syscall(socketpair.sock2, str2cbuf("Socketpair Test"), 15, 0), 15); 

            assert_eq!(cage2.exit_syscall(), 0);
        });

        assert_eq!((&*arc_cage).send_syscall(socketpair.sock1, str2cbuf("test"), 4, 0), 4);

        let mut buf2 = sizecbuf(15);
        (&*arc_cage).recv_syscall(socketpair.sock2, buf2.as_mut_ptr(), 15, 0);
        assert_eq!(cbuf2str(&buf2), "Socketpair Test");
    
        thread.join().unwrap();

        assert_eq!((&*arc_cage).close_syscall(socketpair.sock1), 0);
        assert_eq!((&*arc_cage).close_syscall(socketpair.sock2), 0);

        // end of the TCP test

        socketpair = interface::SockPair::default();
        assert_eq!(Cage::socketpair_syscall(cage.clone(), AF_INET, SOCK_DGRAM, 0, &mut socketpair), 0);

        let thread = interface::helper_thread(move || {
            let cage2 = {CAGE_TABLE.read().unwrap().get(&2).unwrap().clone()};
            
            let mut buf = sizecbuf(10);
            cage2.recv_syscall(socketpair.sock2, buf.as_mut_ptr(), 10, 0);
            assert_eq!(cbuf2str(&buf), "test\0\0\0\0\0\0");
            
            interface::sleep(interface::RustDuration::from_millis(30));
            assert_eq!(cage2.send_syscall(socketpair.sock2, str2cbuf("Socketpair Test"), 15, 0), 15); 

            assert_eq!(cage2.exit_syscall(), 0);
        });

        assert_eq!((&*arc_cage).send_syscall(socketpair.sock2, str2cbuf("test"), 4, 0), 4);

        let mut buf2 = sizecbuf(15);
        (&*arc_cage).recv_syscall(socketpair.sock2, buf2.as_mut_ptr(), 15, 0);
        assert_eq!(cbuf2str(&buf2), "Socketpair Test");
    
        thread.join().unwrap();

        assert_eq!((&*arc_cage).close_syscall(socketpair.sock1), 0);
        assert_eq!((&*arc_cage).close_syscall(socketpair.sock2), 0);

        //end of the UDP test

        assert_eq!((&*arc_cage).exit_syscall(), 0);
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

        let socket = interface::GenSockaddr::V4(interface::SockaddrV4{ sin_family: AF_INET as u16, sin_port: 50121_u16.to_be(), sin_addr: interface::V4Addr{ s_addr: u32::from_ne_bytes([127, 0, 0, 1]) }, padding: 0});

        assert!(serverfd > 0);
        assert!(clientfd > 0);
        
        //forking the cage to get another cage with the same information
        assert_eq!(cage.fork_syscall(2), 0);
        let thread = interface::helper_thread(move || {
            let cage2 = {CAGE_TABLE.read().unwrap().get(&2).unwrap().clone()};
            assert_eq!(cage2.bind_syscall(serverfd, &socket), 0);

            interface::sleep(interface::RustDuration::from_millis(30));
            
            let mut buf = sizecbuf(10);
            cage2.recv_syscall(serverfd, buf.as_mut_ptr(), 10, 0);
            assert_eq!(cbuf2str(&buf), "test\0\0\0\0\0\0");
            
            interface::sleep(interface::RustDuration::from_millis(30));
            assert_eq!(cage2.recv_syscall(serverfd, buf.as_mut_ptr(), 10, 0), 5);
            assert_eq!(cbuf2str(&buf), "test2\0\0\0\0\0");

            assert_eq!(cage2.close_syscall(serverfd), 0);
            assert_eq!(cage2.exit_syscall(), 0);
        });
        
        interface::sleep(interface::RustDuration::from_millis(50));
        let mut buf2 = str2cbuf("test");
        assert_eq!(cage.sendto_syscall(clientfd, buf2, 4, 0, &socket), 4);
        let sendsockfd2 = cage.socket_syscall(AF_INET, SOCK_DGRAM, 0);
        assert!(sendsockfd2 > 0);

        let sockaddr2 = interface::SockaddrV4{ sin_family: AF_INET as u16, sin_port: 50992_u16.to_be(), sin_addr: interface::V4Addr{ s_addr: u32::from_ne_bytes([127, 0, 0, 1]) }, padding: 0};
        let mut socket2 = interface::GenSockaddr::V4(sockaddr2); //127.0.0.1

        interface::sleep(interface::RustDuration::from_millis(50));

        buf2 = str2cbuf("test2");
        assert_eq!(cage.bind_syscall(sendsockfd2, &socket2), 0);
        assert_eq!(cage.sendto_syscall(sendsockfd2, buf2, 5, 0, &socket), 5);

        thread.join().unwrap();

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
        let sockaddr = interface::SockaddrV4{ sin_family: AF_INET as u16, sin_port: 51111_u16.to_be(), sin_addr: interface::V4Addr{ s_addr: u32::from_ne_bytes([127, 0, 0, 1]) }, padding: 0};
        let mut socket = interface::GenSockaddr::V4(sockaddr); //127.0.0.1
        let mut socket_clone = socket.clone();

        assert!(listenfd > 0);
        assert!(sendfd > 0);

        assert_eq!(cage.bind_syscall(listenfd, &socket), 0);

        //forking the cage to get another cage with the same information
        assert_eq!(cage.fork_syscall(2), 0);

        let thread = interface::helper_thread(move || {

            let cage2 = {CAGE_TABLE.read().unwrap().get(&2).unwrap().clone()};
            
            interface::sleep(interface::RustDuration::from_millis(20));
            let mut buf = sizecbuf(16);
            assert_eq!(cage2.recv_syscall(listenfd, buf.as_mut_ptr(), 16, 0), 16);
            assert_ne!(buf, sizecbuf(16));
            assert_eq!(cbuf2str(&buf), "UDP Connect Test");

            assert_eq!(cage2.close_syscall(listenfd), 0);
            assert_eq!(cage2.exit_syscall(), 0);
        });
        
        assert_eq!(cage.connect_syscall(sendfd, &socket), 0);
        interface::sleep(interface::RustDuration::from_millis(50));
        assert_eq!(cage.send_syscall(sendfd, str2cbuf("UDP Connect Test"), 16, 0), 16); 
        thread.join().unwrap();

        assert_eq!(cage.exit_syscall(), 0);
        lindrustfinalize();
    }
}
