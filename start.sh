#!/bin/bash

while true;
do
    $ZENOH_PERF/scripts/cron/cron_l_thr.sh $ZENOH_PERF
    sleep 6h
done
