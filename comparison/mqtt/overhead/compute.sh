#!/usr/bin/env bash

tshark -nlr $1 -w mqtt.pcap "mqtt"
tshark -nlr mqtt.pcap -w mqtt-data.pcap "mqtt.msgtype == 3 || mqtt.msgtype == 4"
capinfos mqtt.pcap -d
capinfos mqtt-data.pcap -d

#tshark -r test.pcap -z io,stat,0,"SUM(tcp.pdu.size)tcp.pdu.size"

tshark -r $1 -l -n -T json -e frame.len -e ip.len -e tcp.pdu.size -e mqtt.msgtype -e mqtt.len -e mqtt.msg "mqtt" > data.json
python3 analyze.py data.json