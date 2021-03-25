#!/usr/bin/env bash

tshark -nlr $1 -w mqtt.pcap "mqtt"
tshark -nlr mqtt.pcap -w mqtt-data.pcap "mqtt.msgtype == 3 || mqtt.msgtype == 4"
capinfos mqtt.pcap -d
capinfos mqtt-data.pcap -d

tshark -r test.pcap -z io,stat,0,"SUM(tcp.pdu.size)tcp.pdu.size"
