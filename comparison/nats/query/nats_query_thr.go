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
	"sync/atomic"
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


func sendRequests(nc *nats.Conn, counter *uint64, payload *[]byte) {

	for {
		_, err := nc.Request(resourceName, *payload, 60*time.Second) //Timeout is required
		if err != nil {
			panic(err)
		}
		atomic.AddUint64(counter, 1)
	}


}

func main() {
	var server = flag.String("s", "127.0.0.1:4222", "The nats server URL")
	var payloadSize = flag.Int("p", 8, "Payload size")
	var showHelp = flag.Bool("h", false, "Show help message")
	var name = flag.String("n", "test", "The test name")
	var scenario = flag.String("c", "brokered", "The test scenario")


	var counter uint64

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

    for i := range payload {
        payload[i] = 0
    }

	atomic.StoreUint64(&counter,0)

	go sendRequests(nc, &counter, &payload)


    for {
		//start := time.Now()
		time.Sleep(1*time.Second)
		//t := time.Now()
		//elapsed := t.Sub(start)

		c := atomic.SwapUint64(&counter,0)
		// if c > 0 {
		fmt.Printf("nats,%s,query.throughout,%s,%d,%d\n", *scenario, *name, *payloadSize, c)
		// }
	}
}