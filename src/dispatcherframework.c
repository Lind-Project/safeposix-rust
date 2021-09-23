#include <stddef.h>
#include <poll.h>
#include <sys/types.h>
#include <sys/socket.h>
#include <sys/epoll.h>
#include "dispatcherframework.h"

#define BLANKARGS \
    union RustArg arg1, arg2, arg3, arg4, arg5, arg6; \
    arg1.dispatch_ulong = 0; \
    arg2.dispatch_ulong = 0; \
    arg3.dispatch_ulong = 0; \
    arg4.dispatch_ulong = 0; \
    arg5.dispatch_ulong = 0; \
    arg6.dispatch_ulong = 0//no semicolon here to force macro caller to place one for neatness

#define DISPATCH_SYSCALL_0_inner(callnum) \
    return dispatcher(cageid, callnum, arg1, arg2, arg3, arg4, arg5, arg6)//no semicolon here to force macro caller to place one for neatness
#define DISPATCH_SYSCALL_1_inner(callnum, arg1type, arg1val) \
    arg1.dispatch_ ## arg1type = arg1val; \
    DISPATCH_SYSCALL_0_inner(callnum)
#define DISPATCH_SYSCALL_2_inner(callnum, arg1type, arg1val, arg2type, arg2val) \
    arg2.dispatch_ ## arg2type = arg2val; \
    DISPATCH_SYSCALL_1_inner(callnum, arg1type, arg1val)
#define DISPATCH_SYSCALL_3_inner(callnum, arg1type, arg1val, arg2type, arg2val, arg3type, arg3val) \
    arg3.dispatch_ ## arg3type = arg3val; \
    DISPATCH_SYSCALL_2_inner(callnum, arg1type, arg1val, arg2type, arg2val)
#define DISPATCH_SYSCALL_4_inner(callnum, arg1type, arg1val, arg2type, arg2val, arg3type, arg3val, \
                                 arg4type, arg4val) \
    arg4.dispatch_ ## arg4type = arg4val; \
    DISPATCH_SYSCALL_3_inner(callnum, arg1type, arg1val, arg2type, arg2val, arg3type, arg3val)
#define DISPATCH_SYSCALL_5_inner(callnum, arg1type, arg1val, arg2type, arg2val, arg3type, arg3val, \
                                 arg4type, arg4val, arg5type, arg5val) \
    arg5.dispatch_ ## arg5type = arg5val; \
    DISPATCH_SYSCALL_4_inner(callnum, arg1type, arg1val, arg2type, arg2val, arg3type, arg3val, arg4type, arg4val)
#define DISPATCH_SYSCALL_6_inner(callnum, arg1type, arg1val, arg2type, arg2val, arg3type, arg3val, \
                                 arg4type, arg4val, arg5type, arg5val, arg6type, arg6val) \
    arg6.dispatch_ ## arg6type = arg6val; \
    DISPATCH_SYSCALL_5_inner(callnum, arg1type, arg1val, arg2type, arg2val, arg3type, arg3val, arg4type, arg4val, arg5type, arg5val)


#define DISPATCH_SYSCALL_6(callnum, arg1type, arg1val, arg2type, arg2val, arg3type, arg3val, \
                           arg4type, arg4val, arg5type, arg5val, arg6type, arg6val) \
    BLANKARGS; \
    DISPATCH_SYSCALL_6_inner(callnum, arg1type, arg1val, arg2type, arg2val, arg3type, arg3val, \
                             arg4type, arg4val, arg5type, arg5val, arg6type, arg6val)
#define DISPATCH_SYSCALL_5(callnum, arg1type, arg1val, arg2type, arg2val, arg3type, arg3val, arg4type, arg4val, arg5type, arg5val) \
    BLANKARGS; \
    DISPATCH_SYSCALL_5_inner(callnum, arg1type, arg1val, arg2type, arg2val, arg3type, arg3val, arg4type, arg4val, arg5type, arg5val)
#define DISPATCH_SYSCALL_4(callnum, arg1type, arg1val, arg2type, arg2val, arg3type, arg3val, arg4type, arg4val) \
    BLANKARGS; \
    DISPATCH_SYSCALL_4_inner(callnum, arg1type, arg1val, arg2type, arg2val, arg3type, arg3val, arg4type, arg4val)
#define DISPATCH_SYSCALL_3(callnum, arg1type, arg1val, arg2type, arg2val, arg3type, arg3val) \
    BLANKARGS; \
    DISPATCH_SYSCALL_3_inner(callnum, arg1type, arg1val, arg2type, arg2val, arg3type, arg3val)
#define DISPATCH_SYSCALL_2(callnum, arg1type, arg1val, arg2type, arg2val) \
    BLANKARGS; \
    DISPATCH_SYSCALL_2_inner(callnum, arg1type, arg1val, arg2type, arg2val)
#define DISPATCH_SYSCALL_1(callnum, arg1type, arg1val) \
    BLANKARGS; \
    DISPATCH_SYSCALL_1_inner(callnum, arg1type, arg1val)
#define DISPATCH_SYSCALL_0(callnum) \
    BLANKARGS; \
    DISPATCH_SYSCALL_0_inner(callnum)


int lind_pread(int fd, void *buf, size_t count, off_t offset, int cageid) {
    DISPATCH_SYSCALL_4(LIND_safe_fs_pread, int, fd, mutcbuf, buf, size_t, count, off_t, offset);
} 

int lind_pwrite(int fd, const void *buf, size_t count, off_t offset, int cageid) {
    DISPATCH_SYSCALL_4(LIND_safe_fs_pwrite, int, fd, cbuf, buf, size_t, count, off_t, offset);
}

int lind_unlink (const char *name, int cageid) {
    DISPATCH_SYSCALL_1(LIND_safe_fs_unlink, cstr, name);
}

int lind_link (const char *from, const char *to, int cageid) {
    DISPATCH_SYSCALL_2(LIND_safe_fs_link, cstr, from, cbuf, to);
}

int lind_access (const char *file, int mode, int cageid) {
    DISPATCH_SYSCALL_2(LIND_safe_fs_access, cstr, file, int, mode);
}

int lind_chdir (const char *name, int cageid) {
    DISPATCH_SYSCALL_1(LIND_safe_fs_chdir, cstr, name);
}

int lind_mkdir (const char *path, int mode, int cageid) {
    DISPATCH_SYSCALL_2(LIND_safe_fs_mkdir, cstr, path, int, mode);
}

int lind_rmdir (const char *path, int cageid) {
    DISPATCH_SYSCALL_1(LIND_safe_fs_rmdir, cstr, path);
}

int lind_xstat (const char *path, struct stat *buf, int cageid) {
    DISPATCH_SYSCALL_2(LIND_safe_fs_xstat, cstr, path, statstruct, buf);
}

int lind_open (const char *path, int flags, int mode, int cageid) {
    DISPATCH_SYSCALL_3(LIND_safe_fs_open, cstr, path, int, flags, int, mode);
}

int lind_close (int fd, int cageid) {
    DISPATCH_SYSCALL_1(LIND_safe_fs_close, int, fd);
}

int lind_read (int fd, void *buf, int size, int cageid) { 
    DISPATCH_SYSCALL_3(LIND_safe_fs_read, int, fd, mutcbuf, buf, int, size);
}

int lind_write (int fd, const void *buf, size_t count, int cageid) { 
    DISPATCH_SYSCALL_3(LIND_safe_fs_write, int, fd, cbuf, buf, int, count);
}

int lind_lseek (int fd, off_t offset, int whence, int cageid) {
    DISPATCH_SYSCALL_3(LIND_safe_fs_lseek, int, fd, off_t, offset, int, whence);
}

int lind_fxstat (int fd, struct stat *buf, int cageid) {
    DISPATCH_SYSCALL_2(LIND_safe_fs_fxstat, int, fd, statstruct, buf);
}

int lind_fstatfs (int fd, struct statfs *buf, int cageid) {
    DISPATCH_SYSCALL_2(LIND_safe_fs_fstatfs, int, fd, statfsstruct, buf);
}

int lind_statfs (const char *path, struct statfs *buf, int cageid) {
    DISPATCH_SYSCALL_2(LIND_safe_fs_statfs, cbuf, path, statfsstruct, buf);
}

int lind_dup (int oldfd, int cageid) {
    DISPATCH_SYSCALL_1(LIND_safe_fs_dup, int, oldfd);
}

int lind_dup2 (int oldfd, int newfd, int cageid) {
    DISPATCH_SYSCALL_2(LIND_safe_fs_dup2, int, oldfd, int, newfd);
}

int lind_getdents (int fd, char *buf, size_t nbytes, int cageid) {
    DISPATCH_SYSCALL_3(LIND_safe_fs_getdents, int, fd, mutcbuf, buf, size_t, nbytes);
}

int lind_fcntl_get (int fd, int cmd, int cageid) {
    DISPATCH_SYSCALL_2(LIND_safe_fs_fcntl, int, fd, int, cmd);
}

int lind_fcntl_set (int fd, int cmd, long set_op, int cageid) {
    DISPATCH_SYSCALL_3(LIND_safe_fs_fcntl, int, fd, int, cmd, long, set_op);
}

int lind_bind (int sockfd, const struct sockaddr *addr, socklen_t addrlen, int cageid) {
    DISPATCH_SYSCALL_3(LIND_safe_net_bind, int, sockfd, constsockaddrstruct, addr, socklen_t, addrlen);
}

int lind_send (int sockfd, const void *buf, size_t len, int flags, int cageid) {
    DISPATCH_SYSCALL_4(LIND_safe_net_send, int, sockfd, cbuf, buf, size_t, len, int, flags);
}

int lind_recv (int sockfd, void *buf, size_t len, int flags, int cageid) {
    DISPATCH_SYSCALL_4(LIND_safe_net_recv, int, sockfd, mutcbuf, buf, size_t, len, int, flags);
}

int lind_sendto (int sockfd, const void *buf, size_t len, int flags, const struct sockaddr *dest_addr, socklen_t addrlen, int cageid) {
    DISPATCH_SYSCALL_6(LIND_safe_net_sendto, int, sockfd, cbuf, buf, size_t, len, int, flags, constsockaddrstruct, dest_addr, socklen_t, addrlen);
}

int lind_recvfrom (int sockfd, const void *buf, size_t len, int flags, struct sockaddr *src_addr, socklen_t *addrlen, int cageid) {
    DISPATCH_SYSCALL_6(LIND_safe_net_recvfrom, int, sockfd, cbuf, buf, size_t, len, int, flags, sockaddrstruct, src_addr, socklen_t_ptr, addrlen);
}

int lind_accept(int sockfd, struct sockaddr *sockaddr, socklen_t *addrlen, int cageid) {
    DISPATCH_SYSCALL_3(LIND_safe_net_recvfrom, int, sockfd, sockaddrstruct, sockaddr, socklen_t_ptr, addrlen);
}

int lind_connect (int sockfd, const struct sockaddr *src_addr, socklen_t addrlen, int cageid) {
    DISPATCH_SYSCALL_3(LIND_safe_net_connect, int, sockfd, constsockaddrstruct, src_addr, socklen_t, addrlen);
}

int lind_listen (int sockfd, int backlog, int cageid) {
    DISPATCH_SYSCALL_2(LIND_safe_net_listen, int, sockfd, int, backlog);
}

int lind_getpeername (int sockfd, struct sockaddr *addr, socklen_t *addrlen, int cageid) {
    DISPATCH_SYSCALL_3(LIND_safe_net_listen, int, sockfd, sockaddrstruct, addr, socklen_t_ptr, addrlen);
}

int lind_getsockopt (int sockfd, int level, int optname, void *optval, socklen_t *optlen, int cageid) {
    DISPATCH_SYSCALL_5(LIND_safe_net_getsockopt, int, sockfd, int, level, int, optname, mutcbuf, optval, socklen_t_ptr, optlen);
}

int lind_setsockopt (int sockfd, int level, int optname, const void *optval, socklen_t optlen, int cageid) {
    DISPATCH_SYSCALL_5(LIND_safe_net_setsockopt, int, sockfd, int, level, int, optname, cbuf, optval, socklen_t, optlen);
}

int lind_select (int nfds, fd_set * readfds, fd_set * writefds, fd_set * exceptfds, struct timeval *timeout, int cageid) {
    DISPATCH_SYSCALL_5(LIND_safe_net_select, int, nfds, cbuf, readfds, cbuf, writefds, cbuf, exceptfds, timevalstruct, timeout);
}

int lind_poll (struct pollfd *fds, unsigned long int nfds, int timeout, int cageid) {
    DISPATCH_SYSCALL_3(LIND_safe_net_poll, cbuf, fds, ulong, nfds, int, timeout);
}

int lind_epoll_create(int size, int cageid) {
    DISPATCH_SYSCALL_1(LIND_safe_net_epoll_create, int, size);
}
int lind_epoll_ctl(int epfd, int op, int fd, struct epoll_event *event, int cageid) {
    DISPATCH_SYSCALL_4(LIND_safe_net_epoll_ctl, int, epfd, int, op, int, fd, epolleventstruct, event);
}
int lind_epoll_wait(int epfd, struct epoll_event *events, int maxevents, int timeout, int cageid) {
    DISPATCH_SYSCALL_4(LIND_safe_net_epoll_ctl, int, epfd, epolleventstruct, events, int, maxevents, int, timeout);
}

int lind_socketpair (int domain, int type, int protocol, int* sv, int cageid) {
    DISPATCH_SYSCALL_4(LIND_safe_net_socketpair, int, domain, int, type, int, protocol, pipearray, sv);
}

int lind_gethostname (char *name, size_t len, int cageid) {
    DISPATCH_SYSCALL_2(LIND_safe_net_gethostname, mutcbuf, name, size_t, len);
}

int lind_socket (int domain, int type, int protocol, int cageid) {
    DISPATCH_SYSCALL_3(LIND_safe_net_socket, int, domain, int, type, int, protocol);
}

int lind_shutdown (int sockfd, int how, int cageid) {
    DISPATCH_SYSCALL_2(LIND_safe_net_shutdown, int, sockfd, int, how);
}

int lind_getuid (int cageid) {
    DISPATCH_SYSCALL_0(LIND_safe_sys_getuid);
}

int lind_geteuid (int cageid) {
    DISPATCH_SYSCALL_0(LIND_safe_sys_geteuid);
}

int lind_getgid (int cageid) {
    DISPATCH_SYSCALL_0(LIND_safe_sys_getgid);
}

int lind_getegid (int cageid) {
    DISPATCH_SYSCALL_0(LIND_safe_sys_getegid);
}

int lind_flock (int fd, int operation, int cageid) {
    DISPATCH_SYSCALL_0(LIND_safe_fs_flock);
}

int lind_pipe(int* pipefds, int cageid) {
    DISPATCH_SYSCALL_1(LIND_safe_fs_pipe, pipearray, pipefds);
}

/* pipe2 currently unimplemented */
int lind_pipe2(int* pipefds, int flags, int cageid) {
    DISPATCH_SYSCALL_2(LIND_safe_fs_pipe2, pipearray, pipefds, int, flags);
}

int lind_fork(int newcageid, int cageid) {
    DISPATCH_SYSCALL_1(LIND_safe_fs_fork, int, newcageid);
}

int lind_mmap(void *addr, size_t length, int prot, int flags, int fd, off_t offset, int cageid) {
    DISPATCH_SYSCALL_6(LIND_safe_fs_mmap, cbuf, addr, size_t, length, int, prot, int, flags, int, fd, off_t, offset);
}

int lind_munmap(void *addr, size_t length, int cageid) {
    DISPATCH_SYSCALL_2(LIND_safe_fs_munmap, cbuf, addr, size_t, length);
}

int lind_getpid(int cageid) {
    DISPATCH_SYSCALL_0(LIND_safe_sys_getpid);
}

int lind_getppid(int cageid) {
    DISPATCH_SYSCALL_0(LIND_safe_sys_getppid);
}

int lind_exec(int newcageid, int cageid) {
    DISPATCH_SYSCALL_1(LIND_safe_fs_exec, int, newcageid);
}

int lind_exit(int status, int cageid) {
    DISPATCH_SYSCALL_1(LIND_safe_sys_exit, int, status);
}
