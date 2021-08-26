#include <stdio.h>
#include <string.h>
#include <time.h>
#include <stdlib.h>
#include <assert.h>
#include <fcntl.h>
#include <unistd.h>
#include "dispatcherframework.h"
const int hexlen = 128 * 1024;
void printimediff(char* c, struct timespec* t1, struct timespec* t2) {
    long int i = t2->tv_sec - t1->tv_sec;
    long int j = t2->tv_nsec - t1->tv_nsec;
    printf("%s took %ld ns\n", c, i * 1000 * 1000 * 1000 + j);
}

void contribtimediff(long int* num, struct timespec* t1, struct timespec* t2) {
    long int i = t2->tv_sec - t1->tv_sec;
    long int j = t2->tv_nsec - t1->tv_nsec;
    *num += (i * 1000 * 1000 * 1000 + j);
}

int main() {
    long int opentime = 0;
    long int readtime = 0;
    long int writetime = 0;
    long int closetime = 0;
    lindrustinit();
    char* hexstr = malloc(hexlen);
    for(int i = 0; i < hexlen/16; i++) {
        int offset = i * 16;
        memcpy(hexstr + offset, "1234567890ABCDEF", 16);
    }
    for(int q = 0; q < 1000; q++) {
        struct timespec starttime, endtime;
        clock_gettime(CLOCK_REALTIME, &starttime);
        int fd = lind_open("16MBhex", O_CREAT | O_RDWR, S_IRWXU, 1);
        clock_gettime(CLOCK_REALTIME, &endtime);
        contribtimediff(&opentime, &starttime, &endtime);
        printimediff("open", &starttime, &endtime);

        clock_gettime(CLOCK_REALTIME, &starttime);
        lind_write(fd, hexstr, hexlen, 1);
        clock_gettime(CLOCK_REALTIME, &endtime);
        contribtimediff(&writetime, &starttime, &endtime);
        printimediff("write", &starttime, &endtime);

        clock_gettime(CLOCK_REALTIME, &starttime);
        lind_lseek(fd, 0, SEEK_SET, 1);
        clock_gettime(CLOCK_REALTIME, &endtime);
        printimediff("lseek", &starttime, &endtime);
        //printimediff("lseek", &starttime, &endtime);

        clock_gettime(CLOCK_REALTIME, &starttime);
        assert(lind_read(fd, hexstr, hexlen, 1) == hexlen);
        clock_gettime(CLOCK_REALTIME, &endtime);
        contribtimediff(&readtime, &starttime, &endtime);
        printimediff("read", &starttime, &endtime);

        clock_gettime(CLOCK_REALTIME, &starttime);
        lind_close(fd, 1);
        clock_gettime(CLOCK_REALTIME, &endtime);
        contribtimediff(&closetime, &starttime, &endtime);
        printimediff("close", &starttime, &endtime);
    }

    printf("read: %ldns, write %ldns\n", readtime / 1000, writetime / 1000);
    lindrustfinalize();
}

