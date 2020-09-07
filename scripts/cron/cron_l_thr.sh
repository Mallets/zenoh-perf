#!/bin/bash

if test "$#" -ne 1; then
    echo "Please provide the zenoh-perf path as argument"
fi    

export ZENOH_PERF=$1

N=50
TS=`eval date "+%F-%T"`
TS=`eval echo $TS | tr : _`
BWD=$ZENOH_PERF/$TS-build
DWD=$ZENOH_PERF/$TS-data

$ZENOH_PERF/scripts/bash/build_zenoh.sh $BWD
ZENOH_ROOT=$BWD/zenoh

PUB=$ZENOH_ROOT/target/release/examples/zn_pub_thr
SUB=$ZENOH_ROOT/target/release/examples/zn_sub_thr

$ZENOH_PERF/scripts/bash/run_l_thr.sh $DWD $PUB $SUB $N
