#!/usr/bin/env bash

sudo tcpdump -i lo 'port 7447' -w zenoh_overhead.pcap