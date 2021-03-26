#!/usr/bin/env bash

tshark -r $1 -l -n -T json -e frame.len -e ip.len -e tcp.dstport -e tcp.srcport -e tcp.payload "not tcp.analysis.retransmission and not tcp.analysis.fast_retransmission" > zenoh_data.json
../target/release/zn_analyze -j zenoh_data.json