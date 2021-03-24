/*
    Copyright (c) 2007-2016 Contributors as noted in the AUTHORS file

    This file is part of libzmq, the ZeroMQ core engine in C++.

    libzmq is free software; you can redistribute it and/or modify it under
    the terms of the GNU Lesser General Public License (LGPL) as published
    by the Free Software Foundation; either version 3 of the License, or
    (at your option) any later version.

    As a special exception, the Contributors give you permission to link
    this library with independent modules to produce an executable,
    regardless of the license terms of these independent modules, and to
    copy and distribute the resulting executable under terms of your choice,
    provided that you also meet, for each linked independent module, the
    terms and conditions of the license of that module. An independent
    module is a module which is not derived from or based on this library.
    If you modify this library, you must extend this exception to your
    version of the library.

    libzmq is distributed in the hope that it will be useful, but WITHOUT
    ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or
    FITNESS FOR A PARTICULAR PURPOSE. See the GNU Lesser General Public
    License for more details.

    You should have received a copy of the GNU Lesser General Public License
    along with this program.  If not, see <http://www.gnu.org/licenses/>.
*/

#include <zmq.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <pthread.h>
#include <unistd.h>
#include <sys/time.h>

int payload;
char *name = NULL;
char *scenario = NULL;
u_int64_t counter = 0;

void *counter_thread(void *arg)
{
    u_int64_t n;
    u_int64_t zero = 0;
    while (1)
    {
        sleep(1);
        __atomic_exchange(&counter, &zero, &n, __ATOMIC_RELAXED);
        if (n > 0)
        {
            printf("zeromq,%s,throughput,%s,%d,%llu\n", scenario, name, payload, n);
            fflush(stdout);
        }
    }
}

int main(int argc, char *argv[])
{
    char *payload_value = NULL;
    const char *locator;
    void *ctx;
    void *s;
    int rc, c;
    zmq_msg_t msg;

    // Parsing arguments
    while ((c = getopt(argc, argv, ":l:p:n:s:")) != -1)
    {
        switch (c)
        {
        case 'n':
            name = optarg;
            break;
        case 's':
            scenario = optarg;
            break;
        case 'p':
            payload_value = optarg;
            break;
        case 'l':
            locator = optarg;
            break;
        default:
            break;
        }
    }

    if (locator == NULL || name == NULL || scenario == NULL || payload_value == NULL)
    {
        printf("Usage: zmq_pub_thr -l tcp://127.0.0.1:4505 -p 8 -n name -s scenario\n");
        exit(EXIT_FAILURE);
    }
    payload = (size_t)atoi(payload_value);

    ctx = zmq_init(1);
    if (!ctx)
    {
        printf("error in zmq_init: %s\n", zmq_strerror(errno));
        return -1;
    }

    s = zmq_socket(ctx, ZMQ_PULL);
    if (!s)
    {
        printf("error in zmq_socket: %s\n", zmq_strerror(errno));
        return -1;
    }

    rc = zmq_bind(s, locator);
    if (rc != 0)
    {
        printf("error in zmq_bind: %s\n", zmq_strerror(errno));
        return -1;
    }

    rc = zmq_msg_init(&msg);
    if (rc != 0)
    {
        printf("error in zmq_msg_init: %s\n", zmq_strerror(errno));
        return -1;
    }

    // Spawn the counter task
    pthread_t *thread = (pthread_t *)malloc(sizeof(pthread_t));
    memset(thread, 0, sizeof(pthread_t));
    rc = pthread_create(thread, NULL, counter_thread, NULL);
    if (rc != 0)
    {
        printf("error in pthread_create: %d\n", rc);
        return -1;
    }

    while (1)
    {
        rc = zmq_recvmsg(s, &msg, 0);
        if (rc < 0)
        {
            printf("error in zmq_recvmsg: %s\n", zmq_strerror(errno));
            return -1;
        }
        __atomic_fetch_add(&counter, 1, __ATOMIC_RELAXED);
    }

    return 0;
}