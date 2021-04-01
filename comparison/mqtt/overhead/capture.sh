#!/usr/bin/env bash

sudo tcpdump -i lo 'port 1883' -w mqtt_overhead.pcap