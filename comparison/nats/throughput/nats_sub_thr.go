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

package main

import (
	"flag"
	"log"
	"os"
	"sync/atomic"
	"time"
	"github.com/nats-io/nats.go"
	"fmt"
)

const resourceName = "test.thr"

func usage() {
	log.Printf("Usage: nats-pub [-s server] [-p payload size] [-n test name] [-c scenario]\n")
	flag.PrintDefaults()
}

func showUsageAndExit(exitcode int) {
	usage()
	os.Exit(exitcode)
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
	opts := []nats.Option{nats.Name("NATS Throughput Subscriber")}

	// Connect to NATS
	nc, err := nats.Connect(*server, opts...)
	if err != nil {
		panic(err)
	}
	defer nc.Close()

	atomic.StoreUint64(&counter,0)

	_, err = nc.Subscribe(resourceName, func(msg *nats.Msg) {
		atomic.AddUint64(&counter, 1)
	})
	if  err != nil {
		panic(err)
	}

	err = nc.Flush()
	if  err != nil {
		panic(err)
	}

	for {
		//start := time.Now()
		time.Sleep(1*time.Second)
		//t := time.Now()
		//elapsed := t.Sub(start)

		c := atomic.SwapUint64(&counter,0)
		// if c > 0 {
		fmt.Printf("nats,%s,throughout,%s,%d,%d\n", *scenario, *name, *payloadSize, c)
		// }
	}


}