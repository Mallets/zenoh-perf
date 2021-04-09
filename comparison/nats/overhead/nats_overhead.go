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
)

const resourceName = "test.overhead"

func usage() {
	log.Printf("Usage: nats-pub [-s server] [-p payload size] [-t total size] [-v ]\n")
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
	// var verbose = flag.Bool("v", false, "Verbose")
	var totalSize = flag.Int("t", 1, "Total size")
	var interveal = flag.Float64("i", 0.0, "Interveal")
	var bytesInMB uint64 = 1_048_576;



	log.SetFlags(0)
	flag.Usage = usage
	flag.Parse()

	if *showHelp {
		showUsageAndExit(0)
	}


	// Connect Options.
	opts := []nats.Option{nats.Name("NATS Overhead")}

	// Connect to NATS
	nc, err := nats.Connect(*server, opts...)
	if err != nil {
		log.Fatal(err)
	}
	defer nc.Close()

    var payload = make([]byte, *payloadSize)

    for i := range payload {
        payload[i] = 0
    }

	totalMessages := (uint64(*totalSize) * bytesInMB) / uint64(*payloadSize)

	for i := uint64(0); i < totalMessages; i++ {
        if err := nc.Publish(resourceName, payload); err != nil {
			panic(err)
        }
		time.Sleep(time.Duration(*interveal)*time.Second)
    }

	nc.Flush()

}