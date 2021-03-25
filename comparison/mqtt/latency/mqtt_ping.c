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
#include <sys/time.h>


#define CLIENTID    "mqtt_ping"
#define PING_TOPIC       "/test/ping"
#define PONG_TOPIC       "/test/pong"
#define QOS         1

volatile int finished = 0;
volatile int ready = 0;
volatile int subscribed = 0;
volatile int received = 0;
u_int64_t counter = 0;
const char* DEFAULT_BROKER = "tcp://127.0.0.1:1883";
const char* DEFAULT_PONG_TOPIC = "/test/pong";





void onConnectFailure(void* context, MQTTAsync_failureData5* response)
{
	printf("Connect failed, rc %d\n", response ? response->code : 0);
	exit(EXIT_FAILURE);
}


void onConnect(void* context, MQTTAsync_successData5* response)
{
	ready = 1;
}

void onSubscribe(void* context, MQTTAsync_successData5* response)
{
	subscribed = 1;
}

void onSubscribeFailure(void* context, MQTTAsync_failureData5* response)
{
	printf("Subscribe failed, rc %d\n", response->code);
	exit(EXIT_FAILURE);
}


int msgarrvd(void *context, char *topicName, int topicLen, MQTTAsync_message *message)
{
	if (strncmp(topicName, DEFAULT_PONG_TOPIC, topicLen) == 0 ) {
		received = 1;
	}
	MQTTAsync_freeMessage(&message);
	MQTTAsync_free(topicName);
	return 1;
}


int main(int argc, char* argv[])
{
	MQTTAsync client;
	MQTTAsync_connectOptions conn_opts = MQTTAsync_connectOptions_initializer5;
	MQTTAsync_createOptions create_opts = MQTTAsync_createOptions_initializer;
	int rc, c;
	float interveal = 1; //s
	size_t payload = 8;
	size_t seq_num = 0;
	char* broker = NULL;
	char* payload_value = NULL;
	void* data = NULL;
	char* name = NULL;
	char* scenario = NULL;
	char* interveal_value = NULL;
	struct timespec start, end;



	 // Parsing arguments
	while((c = getopt(argc, argv, ":b:p:n:s:i:")) != -1 ){
		switch (c) {
			case 'n':
				name = optarg;
				break;
			case 's':
				scenario = optarg;
				break;
			case 'p':
				payload_value = optarg;
				break;
			case 'b':
				broker = optarg;
				break;
			case 'i':
				interveal_value = optarg;
				break;
			default:
				break;
		}
	}

	if (name == NULL) {
		printf("Missing -n parameter, exiting!");
		exit(EXIT_FAILURE);
	}

	if (scenario == NULL) {
		printf("Missing -s parameter, exiting!");
		exit(EXIT_FAILURE);
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

	if (interveal_value != NULL) {
		interveal = (size_t) atof(interveal_value);
	}

	data = (void*) calloc(sizeof(u_int8_t),payload);

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


	MQTTAsync_responseOptions opts = MQTTAsync_responseOptions_initializer;

	opts.onSuccess5 = onSubscribe;
	opts.onFailure5 = onSubscribeFailure;
	opts.context = client;
	if ((rc = MQTTAsync_subscribe(client, PONG_TOPIC, QOS, &opts)) != MQTTASYNC_SUCCESS)
	{
		printf("Failed to start subscribe, return code %d\n", rc);
		finished = 1;
	}

	if ((rc = MQTTAsync_setCallbacks(client, client, NULL, msgarrvd, NULL)) != MQTTASYNC_SUCCESS)
	{
		printf("Failed to set callbacks, return code %d\n", rc);
		MQTTAsync_destroy(&client);
		exit(EXIT_FAILURE);
	}

	while (!subscribed) ;


	while (1) {
		usleep((useconds_t)interveal * 1000000);
		MQTTAsync_message pubmsg = MQTTAsync_message_initializer;
		memcpy(data, seq_num);
		pubmsg.payload = data;
		pubmsg.payloadlen = (int) payload;
		pubmsg.qos = QOS;
		pubmsg.retained = 0;
		if ((rc = MQTTAsync_sendMessage(client, PING_TOPIC, &pubmsg, NULL)) == MQTTASYNC_SUCCESS)
		{
				clock_gettime(CLOCK_MONOTONIC_RAW, &start);
				// The message was sent, we should wait for the reply
				while (!received) usleep((useconds_t)1);
				clock_gettime(CLOCK_MONOTONIC_RAW, &end);
				received = 0;
				u_int64_t elapsed = (end.tv_sec - start.tv_sec) * 1000000 + (end.tv_nsec - start.tv_nsec) / 1000;

				printf("mqtt,%s,latency,%s,%ld,%ld\n", scenario, name, payload,elapsed);
				fflush(stdout);

		}
	}



	MQTTAsync_destroy(&client);
 	return rc;
}


