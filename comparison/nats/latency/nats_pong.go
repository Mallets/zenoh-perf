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
	"time"
	"github.com/nats-io/nats.go"
)

const resourceNamePing = "test.ping"
const resourceNamePong = "test.pong"


func usage() {
	log.Printf("Usage: nats-pong [-s server] [-p payload size] [-n test name] [-c scenario]\n")
	flag.PrintDefaults()
}

func showUsageAndExit(exitcode int) {
	usage()
	os.Exit(exitcode)
}




func main() {
	var server = flag.String("s", "127.0.0.1:4222", "The nats server URL")
	var showHelp = flag.Bool("h", false, "Show help message")

	log.SetFlags(0)
	flag.Usage = usage
	flag.Parse()

	if *showHelp {
		showUsageAndExit(0)
	}

	// Connect Options.
	opts := []nats.Option{nats.Name("NATS Pong")}

	// Connect to NATS
	nc, err := nats.Connect(*server, opts...)
	if err != nil {
		panic(err)
	}
	defer nc.Close()


	_, err = nc.Subscribe(resourceNamePing, func(msg *nats.Msg) {
		nc.Publish(resourceNamePong, msg.Data)
	})
	if  err != nil {
		panic(err)
	}

	err = nc.Flush()
	if  err != nil {
		panic(err)
	}

	for {
		time.Sleep(1000*time.Second)
	}


}