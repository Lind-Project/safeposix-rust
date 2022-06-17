#include <arpa/inet.h>
#include <sys/socket.h>
#include <ifaddrs.h>
#include <stdio.h>
#include <sys/ioctl.h>
#include <net/if.h>

int main ()
{
    struct ifaddrs *ifap, *ifa;
    struct sockaddr_in *sa, *ba, *na, *da;
    char *addr, *baddr, *naddr, *daddr;

    getifaddrs (&ifap);
    for (ifa = ifap; ifa; ifa = ifa->ifa_next) {
        if (ifa->ifa_addr && ifa->ifa_addr->sa_family==AF_INET) {
            sa = (struct sockaddr_in *) ifa->ifa_addr;
            addr = inet_ntoa(sa->sin_addr);
            na = (struct sockaddr_in *) ifa->ifa_netmask;
            naddr = inet_ntoa(na->sin_addr);
            ba = (struct sockaddr_in *) ifa->ifa_broadaddr;
            baddr = inet_ntoa(ba->sin_addr);
            da = (struct sockaddr_in *) ifa->ifa_dstaddr;
            daddr = inet_ntoa(da->sin_addr);
            printf("%s ", ifa->ifa_name);
            printf("%d ", ifa->ifa_flags);
            printf("%s ", addr);
            printf("%s", naddr);
            if (ifa->ifa_flags & IFF_BROADCAST) printf(" %s\n", baddr);
            else if (ifa->ifa_flags & IFF_POINTOPOINT) printf(" %s\n", daddr);
            else printf(",none\n");
        }
    }

    freeifaddrs(ifap);
    return 0;
}
