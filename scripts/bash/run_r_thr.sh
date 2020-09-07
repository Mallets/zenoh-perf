#!/bin/bash

if [$# -ne 4]; then
    echo "USAGE:\n\t run_thr.sh <pub> <sub> <remote-host> <N>"

PUB=$1
SUB=$2
RH=$3
N=$4

SIZE="8 16 32 64 128 256 512 1024 2048 4096 8192 16384 32768  65536  131072 262144 524288 1048576 2097152 4194304 8388608 16777216"

DATE=`eval date "+%F-%T"`
DATA_PATH="zenoh-perf-data/thr/local/$DATE"
mkdir -p  $DATA_PATH

for s in $SIZE; do
    {
        TS=`eval date "+%F-%T"`
        $SUB -s $N > $DATA_PATH/$s.txt &
        echo "[$TS]: Testing thropughput for $s bytes"
        export S_PID=$!
        TS=`eval date "+%F-%T"`
        echo "[$TS]: Started Subscriber (PID = $S_PID)"
        sleep 1
        ssh ato@$RH -t $PUB $s &
        P_PID=$!
        TS=`eval date "+%F-%T"`
        echo "[$TS]: Started Publisher (PID = $P_PID) - $s"
        TS=`eval date "+%F-%T"`
        echo "[$TS]: Waiting for Subscriber ($S_PID)"
    }
    wait $S_PID
    TS=`eval date "+%F-%T"`
    echo "[$TS]: Subscriber completed, terminating publisher!"
    kill -9 $P_PID &> /dev/null
    wait $P_PID &> /dev/null
    sleep 1
done
