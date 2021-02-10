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
use async_std::task;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;
use structopt::StructOpt;
use zenoh::net::ResKey::*;
use zenoh::net::*;
use zenoh::Properties;

#[derive(Debug, StructOpt)]
#[structopt(name = "zn_sub_thr")]
struct Opt {
    #[structopt(short = "l", long = "locator")]
    locator: Option<String>,
    #[structopt(short = "m", long = "mode")]
    mode: String,
    #[structopt(short = "s", long = "scout")]
    scout: bool,
    #[structopt(short = "p", long = "payload")]
    payload: usize,
    #[structopt(short = "n", long = "name")]
    name: String,
}

#[async_std::main]
async fn main() {
    // initiate logging
    env_logger::init();

    // Parse the args
    let opt = Opt::from_args();

    let mut config = Properties::default();
    config.insert("mode".to_string(), opt.mode.clone());

    if opt.scout {
        config.insert("multicast_scouting".to_string(), "true".to_string());
    } else {
        config.insert("multicast_scouting".to_string(), "false".to_string());
        match opt.mode.as_str() {
            "peer" => config.insert("listener".to_string(), opt.locator.unwrap().to_string()),
            "client" => config.insert("peer".to_string(), opt.locator.unwrap().to_string()),
            _ => panic!("Unsupported mode: {}", opt.mode),
        };
    }

    let session = open(config.into()).await.unwrap();

    let reskey = RId(session
        .declare_resource(&RName("/test/thr".to_string()))
        .await
        .unwrap());

    let messages = Arc::new(AtomicUsize::new(0));
    let c_messages = messages.clone();

    let sub_info = SubInfo {
        reliability: Reliability::Reliable,
        mode: SubMode::Push,
        period: None,
    };
    session
        .declare_callback_subscriber(&reskey, &sub_info, move |_sample| {
            c_messages.fetch_add(1, Ordering::AcqRel);
        })
        .await
        .unwrap();

    loop {
        task::sleep(Duration::from_secs(1)).await;
        let messages = messages.swap(0, Ordering::AcqRel);
        println!(
            "zenoh-net,sub,throughput,{},{},{}",
            opt.name, opt.payload, messages
        );
    }
}
