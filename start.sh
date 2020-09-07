#!/bin/bash

function cleanup() {
    TS=`eval date "+%F-%T"`
    echo "[$TS]: Cleaning up environment"

    for i in $(seq 1 10); do
        killall -9 zn_sub_thr &>/dev/null
        killall -9 zn_pub_thr &>/dev/null
        sleep 3
    done
}

while true;
do
    cleanup
    git pull
    $ZENOH_PERF/scripts/cron/cron_l_thr.sh $ZENOH_PERF
    sleep 5h
    cleanup
    sleep 1h
done
