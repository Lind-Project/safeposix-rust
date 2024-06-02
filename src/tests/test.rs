pub fn ut_lind_net_select() {
    lindrustinit(0);
    let cage = interface::cagetable_getref(1);

    let filefd = cage.open_syscall("/netselecttest.txt", O_CREAT | O_EXCL | O_RDWR, S_IRWXA);
    assert!(filefd > 0);

    let serversockfd = cage.socket_syscall(AF_INET, SOCK_STREAM, 0);
    let clientsockfd1 = cage.socket_syscall(AF_INET, SOCK_STREAM, 0);
    let clientsockfd2 = cage.socket_syscall(AF_INET, SOCK_STREAM, 0);

    let port: u16 = generate_random_port();
    let sockaddr = interface::SockaddrV4 {
        sin_family: AF_INET as u16,
        sin_port: port.to_be(),
        sin_addr: interface::V4Addr {
            s_addr: u32::from_ne_bytes([127, 0, 0, 1]),
        },
        padding: 0,
    };
    let socket = interface::GenSockaddr::V4(sockaddr); //127.0.0.1 from bytes above
    assert_eq!(cage.bind_syscall(serversockfd, &socket), 0);
    assert_eq!(cage.listen_syscall(serversockfd, 4), 0);

    let master_inputs = &mut interface::FdSet::new();
    let master_outputs = &mut interface::FdSet::new();

    // allocate spaces for fd_set bitmaps
    let inputs = &mut interface::FdSet::new();
    let outputs = &mut interface::FdSet::new();

    master_inputs.set(serversockfd);
    master_inputs.set(filefd);
    master_outputs.set(filefd);
    
    inputs.copy_from(master_inputs);
    outputs.copy_from(master_outputs);

    assert_eq!(inputs.is_set(serversockfd), true);
    assert_eq!(inputs.is_set(filefd), true);
    assert_eq!(outputs.is_set(filefd), true);

    assert_eq!(cage.fork_syscall(2), 0);
    assert_eq!(cage.fork_syscall(3), 0);

    assert_eq!(cage.close_syscall(clientsockfd1), 0);
    assert_eq!(cage.close_syscall(clientsockfd2), 0);

    // these barriers ensures that the clients finish the connect before we do the select
    let barrier = Arc::new(Barrier::new(3));
    let barrier_clone1 = barrier.clone();
    let barrier_clone2 = barrier.clone();

    //client 1 connects to the server to send and recv data...
    let threadclient1 = interface::helper_thread(move || {
        let cage2 = interface::cagetable_getref(2);
        assert_eq!(cage2.close_syscall(serversockfd), 0);

        assert_eq!(cage2.connect_syscall(clientsockfd1, &socket), 0);
        barrier_clone1.wait();
        assert_eq!(cage2.send_syscall(clientsockfd1, str2cbuf("test"), 4, 0), 4);

        interface::sleep(interface::RustDuration::from_millis(1));

        let mut buf = sizecbuf(4);
        assert_eq!(cage2.recv_syscall(clientsockfd1, buf.as_mut_ptr(), 4, 0), 4);
        assert_eq!(cbuf2str(&buf), "test");

        assert_eq!(cage2.close_syscall(clientsockfd1), 0);
        cage2.exit_syscall(EXIT_SUCCESS);
    });

    //client 2 connects to the server to send and recv data...
    let threadclient2 = interface::helper_thread(move || {
        let cage3 = interface::cagetable_getref(3);

        assert_eq!(cage3.close_syscall(serversockfd), 0);

        assert_eq!(cage3.connect_syscall(clientsockfd2, &socket), 0);
        barrier_clone2.wait();
        assert_eq!(cage3.send_syscall(clientsockfd2, str2cbuf("test"), 4, 0), 4);

        interface::sleep(interface::RustDuration::from_millis(1));

        let mut buf = sizecbuf(4);
        let mut result: i32;
        loop {
            result = cage3.recv_syscall(clientsockfd2, buf.as_mut_ptr(), 4, 0);
            if result != -libc::EINTR {
                break; // if the error was EINTR, retry the syscall
            }
        }
        assert_eq!(result, 4);
        assert_eq!(cbuf2str(&buf), "test");

        assert_eq!(cage3.close_syscall(clientsockfd2), 0);
        cage3.exit_syscall(EXIT_SUCCESS);
    });
    barrier.wait();
    //acting as the server and processing the request
    for _counter in 0..600 {
        inputs.copy_from(master_inputs);
        outputs.copy_from(master_outputs);
        let select_result = cage.select_syscall(
            11,
            Some(inputs),
            Some(outputs),
            None,
            Some(interface::RustDuration::ZERO),
        );
        assert!(select_result >= 0);

        //Check for any activity in any of the Input sockets...
        //for sock in binputs {
        for sock in 0..FD_SET_MAX_FD {
            if !inputs.is_set(sock) {
                continue;
            }

            //If the socket returned was listerner socket, then there's a new conn., so we accept it, and put the client socket in the list of Inputs.
            if sock == serversockfd {
                let mut sockgarbage =
                    interface::GenSockaddr::V4(interface::SockaddrV4::default());
                let sockfd = cage.accept_syscall(sock as i32, &mut sockgarbage); //really can only make sure that the fd is valid
                assert!(sockfd > 0);
                master_inputs.set(sockfd);
                master_outputs.set(sockfd);
            } else if sock == filefd {
                //Write to a file...
                assert_eq!(cage.write_syscall(sock as i32, str2cbuf("test"), 4), 4);
                assert_eq!(cage.lseek_syscall(sock as i32, 0, SEEK_SET), 0);
                master_inputs.clear(sock);
            } else {
                //If the socket is in established conn., then we recv the data. If there's no data, then close the client socket.
                let mut buf = sizecbuf(4);
                let mut recvresult: i32;
                loop {
                    recvresult = cage.recv_syscall(sock as i32, buf.as_mut_ptr(), 4, 0);
                    if recvresult != -libc::EINTR {
                        break; // if the error was EINTR, retry the syscall
                    }
                }
                if recvresult == 4 {
                    if cbuf2str(&buf) == "test" {
                        master_outputs.set(sock);
                        continue;
                    }
                } else {
                    assert_eq!(recvresult, 0);
                }
                assert_eq!(cage.close_syscall(sock as i32), 0);
                master_inputs.clear(sock);
            }
        }

        //for sock in boutputs {
        for sock in 0..FD_SET_MAX_FD {
            if !outputs.is_set(sock) {
                continue;
            }
            if sock == filefd {
                let mut buf = sizecbuf(4);
                assert_eq!(cage.read_syscall(sock as i32, buf.as_mut_ptr(), 4), 4);
                assert_eq!(cbuf2str(&buf), "test");
                master_outputs.clear(sock);
            } else {
                //Data is sent out this socket, it's no longer ready for writing remove this socket from writefd's.
                assert_eq!(cage.send_syscall(sock as i32, str2cbuf("test"), 4, 0), 4);
                master_outputs.clear(sock);
            }
        }
    }
    assert_eq!(cage.close_syscall(serversockfd), 0);

    threadclient1.join().unwrap();
    threadclient2.join().unwrap();

    assert_eq!(cage.exit_syscall(EXIT_SUCCESS), EXIT_SUCCESS);
    lindrustfinalize();
}