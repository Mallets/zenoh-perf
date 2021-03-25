#!/usr/bin/env bash

tshark -nlr $1 -w mqtt.pcap "mqtt"
capinfos mqtt.pcap -d