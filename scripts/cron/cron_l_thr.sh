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
SWD=$ZENOH_PERF/stats

mkdir $ZENOH_PERF/stats &>/dev/null

$ZENOH_PERF/scripts/bash/build_zenoh.sh $BWD
ZENOH_ROOT=$BWD/zenoh

PUB=$ZENOH_ROOT/target/release/examples/zn_pub_thr
SUB=$ZENOH_ROOT/target/release/examples/zn_sub_thr

$ZENOH_PERF/scripts/bash/run_l_thr.sh $DWD $PUB $SUB $N
Rscript $ZENOH_PERF/scripts/R/gen_stats.R $DWD $SWD $TS

