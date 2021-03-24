//
// Copyright (c) 2017, 2020 ADLINK Technology Inc.
//
// This program and the accompanying materials are made available under the
// terms of the Eclipse Public License 2.0 which is available at
// http://www.eclipse.org/legal/epl-2.0, or the Apache License, Version 2.0
// which is available at https://www.apache.org/licenses/LICENSE-2.0.
//
// SPDX-License-Identifier: EPL-2.0 OR Apache-2.0
//
// Contributors:
//   ADLINK zenoh team, <zenoh@adlink-labs.tech>
//
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include "MQTTAsync.h"
#include <assert.h>
#include <pthread.h>
#include <unistd.h>
#include <ctype.h>


#define CLIENTID    "mqtt_overhead"
#define TOPIC       "/test/overhead"
#define QOS         1

volatile int finished = 0;
volatile int ready = 0;
u_int64_t counter = 0;
const char* DEFAULT_BROKER = "tcp://127.0.0.1:1883";


struct send_struct {
	MQTTAsync client;
	void* data;
	size_t payload;
	u_int64_t msgs;
};


void onConnectFailure(void* context, MQTTAsync_failureData5* response)
{
	printf("Connect failed, rc %d\n", response ? response->code : 0);
	exit(EXIT_FAILURE);
}


void onConnect(void* context, MQTTAsync_successData5* response)
{
	ready = 1;
}



void *send_msgs(void *args) {

	struct send_struct *thr_args = (struct send_struct *)args;
	int rc;

	u_int64_t i = 0;
	while (i < thr_args->msgs) {
		MQTTAsync_message pubmsg = MQTTAsync_message_initializer;
		pubmsg.payload = thr_args->data;
		pubmsg.payloadlen = (int) thr_args->payload;
		pubmsg.qos = QOS;
		pubmsg.retained = 0;
		if ((rc = MQTTAsync_sendMessage(thr_args->client, TOPIC, &pubmsg, NULL)) != MQTTASYNC_SUCCESS)
		{
			printf("Failed to send message, return code %d\n", rc);

		} else {
			i++;
			__atomic_fetch_add(&counter, 1, __ATOMIC_RELAXED);
		}

	}
	finished = 1;
	return NULL;
}

int main(int argc, char* argv[])
{
	MQTTAsync client;
	MQTTAsync_connectOptions conn_opts = MQTTAsync_connectOptions_initializer5;
	MQTTAsync_createOptions create_opts = MQTTAsync_createOptions_initializer;
	int rc, c;
	int print_flag = 0;
	size_t payload = 8;
	u_int64_t total_size = 1048576; // 1MB in bytes
	u_int64_t bytes_in_mb = 1048576; // bytes
	char* broker = NULL;
	char* payload_value = NULL;
	char* total_size_value = NULL;
	void* data = NULL;
	pthread_t sending;


	// Parsing arguments
	while((c = getopt(argc, argv, ":tb:p:s:")) != -1 ){
		switch (c) {
			case 't':
				print_flag = 1;
				break;
			case 'p':
				payload_value = optarg;
				break;
			case 'b':
				broker = optarg;
				break;
			case 's':
				total_size_value = optarg;
				break;
			default:
				break;
		}
	}

	// Setting defaults

	if (broker == NULL) {
		// We should copy the default
		broker = (char*) calloc(sizeof(char),strlen(DEFAULT_BROKER));
		memcpy(broker, DEFAULT_BROKER, strlen(DEFAULT_BROKER));
	}

	if (payload_value != NULL) {
		payload = (size_t) atoi(payload_value);
	}

	if (total_size_value != NULL) {
		total_size = ((u_int64_t) atoi(total_size_value) * bytes_in_mb);
	}

	data = (void*) calloc(sizeof(u_int8_t),payload);

	printf("Print: %d Broker: %s Payload: %ld Total Size: %ld\n", print_flag, broker, payload, total_size);

	create_opts.MQTTVersion = MQTTVERSION_5;
	if ((rc = MQTTAsync_createWithOptions(&client, broker, CLIENTID, MQTTCLIENT_PERSISTENCE_NONE, NULL, &create_opts)) != MQTTASYNC_SUCCESS)
	{
		printf("Failed to create client object, return code %d\n", rc);
		exit(EXIT_FAILURE);
	}

	conn_opts.keepAliveInterval = 3;
	conn_opts.onSuccess5 = onConnect;
	conn_opts.onFailure5 = onConnectFailure;
	conn_opts.context = client;
	conn_opts.MQTTVersion = MQTTVERSION_5;
	conn_opts.cleanstart = 1;

	if ((rc = MQTTAsync_connect(client, &conn_opts)) != MQTTASYNC_SUCCESS)
	{
		printf("Failed to start connect, return code %d\n", rc);
		exit(EXIT_FAILURE);
	}


	while (!ready);

	if (print_flag) {

		struct send_struct args;
		args.client = client;
		args.data = data;
		args.payload = payload;
		args.msgs = total_size/(u_int64_t) payload;

		if ((rc = pthread_create(&sending, NULL,&send_msgs, (void*)&args)) != 0) {
			printf("Failed to start sending thread, return code %d\n", rc);
			exit(EXIT_FAILURE);
		}

		while (!finished) {
			sleep(1);
			u_int64_t n;
			u_int64_t zero = 0;
			__atomic_exchange(&counter, &zero, &n, __ATOMIC_RELAXED);
			printf("%ld msg/s\n", n);
		}
	} else {
		 u_int64_t max_msgs = total_size/(u_int64_t) payload;
		 u_int64_t i = 0;
		 while (i < max_msgs) {
			MQTTAsync_message pubmsg = MQTTAsync_message_initializer;
			pubmsg.payload = data;
			pubmsg.payloadlen = (int) payload;
			pubmsg.qos = QOS;
			pubmsg.retained = 0;
			if ((rc = MQTTAsync_sendMessage(client, TOPIC, &pubmsg, NULL)) != MQTTASYNC_SUCCESS)
			{
				printf("Failed to send message: %ld/%ld, return code %d\n",i,max_msgs, rc);
			} else {
				i++;
			}
		}
	}



	MQTTAsync_destroy(&client);
 	return rc;
}


