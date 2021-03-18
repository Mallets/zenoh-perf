//
// Copyright (c) 2017, 2020 ADLINK Technology Inc.
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
use async_std::sync::Arc;
use futures::prelude::*;
use std::time::Instant;
use structopt::StructOpt;
use zenoh::net::ResKey;
use zenoh::net::*;
use zenoh::Properties;

#[derive(Debug, StructOpt)]
#[structopt(name = "zn_ping")]
struct Opt {
    #[structopt(short = "e", long = "peer")]
    peer: Option<String>,
    #[structopt(short = "m", long = "mode")]
    mode: String,
    #[structopt(short = "n", long = "name")]
    name: String,
    #[structopt(short = "s", long = "scenario")]
    scenario: String,
}

#[async_std::main]
async fn main() {
    // initiate logging
    env_logger::init();

    // Parse the args
    let opt = Opt::from_args();

    let mut config = Properties::default();
    config.insert("mode".to_string(), opt.mode.clone());

    if opt.peer.is_none() {
        config.insert("multicast_scouting".to_string(), "true".to_string());
    } else {
        config.insert("multicast_scouting".to_string(), "false".to_string());
        config.insert("peer".to_string(), opt.peer.unwrap());
    }

    let session = open(config.into()).await.unwrap();
    let session = Arc::new(session);

    let mut count: u64 = 0;
    loop {
        let reskey = ResKey::RName("/test/query".to_string());
        let predicate = "";
        let target = QueryTarget::default();
        let consolidation = QueryConsolidation::default();

        let now = Instant::now();
        let mut replies = session
            .query(&reskey, predicate, target, consolidation)
            .await
            .unwrap();

        let mut payload: usize = 0;
        while let Some(reply) = replies.next().await {
            payload += reply.data.payload.len();
        }
        println!(
            "zenoh-net,{},query,{},{},{},{}",
            opt.scenario,
            opt.name,
            payload,
            count,
            now.elapsed().as_micros()
        );

        count += 1;
    }
}
