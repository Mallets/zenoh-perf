#!/usr/bin/env bash

sudo tcpdump -i lo 'port 4222' -w nats_overhead.pcap