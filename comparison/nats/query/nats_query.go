//
// Copyright (c) 2017, 2021 ADLINK Technology Inc.
//
// This program and the accompanying materials are made available under the
// terms of the Eclipse Public License 2.0 which is available at
// http://www.eclipse.org/legal/epl-2.0, or the Apache License, Version 2.0
// which is available at https://www.apache.org/licenses/LICENSE-2.0.
//
// SPDX-License-Identifier: EPL-2.0 OR Apache-2.0
//
// Contributors:
//   ADLINK zenoh team, <zenoh@adlink-labs.tech>
//

// NATS v2.2.0
// nats-go v1.10

package main

import (
	"flag"
	"log"
	"os"
	"github.com/nats-io/nats.go"
	"time"
	"fmt"
)

const resourceName = "test.query"

func usage() {
	log.Printf("Usage: nats-query-thr [-s server] [-p payload size] [-v ]\n")
	flag.PrintDefaults()
}

func showUsageAndExit(exitcode int) {
	usage()
	os.Exit(exitcode)
}

func serializeSequenceNumber(seqNumber uint64, payload *[]byte) {
	(*payload)[0] = byte(0xff & seqNumber)
	(*payload)[1] = byte(0xff & (seqNumber >> 8))
    (*payload)[2] = byte(0xff & (seqNumber >> 16))
    (*payload)[3] = byte(0xff & (seqNumber >> 24))
	(*payload)[4] = byte(0xff & (seqNumber >> 32))
	(*payload)[5] = byte(0xff & (seqNumber >> 40))
	(*payload)[6] = byte(0xff & (seqNumber >> 48))
	(*payload)[7] = byte(0xff & (seqNumber >> 56))
}

func deserializeSequenceNumber(payload *[]byte) uint64 {
	var seqNumber uint64 = 0
	seqNumber = uint64((*payload)[0]) |
				uint64((*payload)[1]) << 8 |
				uint64((*payload)[2]) << 16 |
				uint64((*payload)[3]) << 24 |
				uint64((*payload)[4]) << 32 |
				uint64((*payload)[5]) << 40 |
				uint64((*payload)[6]) << 48 |
				uint64((*payload)[7]) << 56

	return seqNumber
}


func main() {
	var server = flag.String("s", "127.0.0.1:4222", "The nats server URL")
	var payloadSize = flag.Int("p", 8, "Payload size")
	var showHelp = flag.Bool("h", false, "Show help message")
	var name = flag.String("n", "test", "The test name")
	var scenario = flag.String("c", "brokered", "The test scenario")
	var interveal = flag.Float64("i", 1.0, "Interveal")

	log.SetFlags(0)
	flag.Usage = usage
	flag.Parse()

	if *showHelp {
		showUsageAndExit(0)
	}


	// Connect Options.
	opts := []nats.Option{nats.Name("NATS Query Throughput")}


	nc, err := nats.Connect(*server, opts...)
	if err != nil {
		log.Fatal(err)
	}
	defer nc.Close()

    var payload = make([]byte, *payloadSize)
	var seqNumber uint64 = 0

    for i := range payload {
        payload[i] = 0
    }


    for {
		serializeSequenceNumber(seqNumber, &payload)

		start := time.Now()
		m, err := nc.Request(resourceName, payload, 60*time.Second) //Timeout is required
		t := time.Now()
		if err != nil {
			panic(err)
		}
		seq := deserializeSequenceNumber(&m.Data)
		elapsed := t.Sub(start)

		//t := time.Now()
		fmt.Printf("nats,%s,query.latency,%s,%d,%d,%d\n", *scenario, *name, *payloadSize, seq,elapsed.Microseconds())
		seqNumber += 1
		time.Sleep(time.Duration(*interveal)*time.Second)

	}
}