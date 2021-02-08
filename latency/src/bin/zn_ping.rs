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
use async_std::stream::StreamExt;
use async_std::sync::{Arc, Barrier, Mutex};
use async_std::task;
use std::collections::HashMap;
use std::time::{Duration, Instant};
use structopt::StructOpt;
use zenoh::net::ResKey::*;
use zenoh::net::*;
use zenoh::Properties;

#[derive(Debug, StructOpt)]
#[structopt(name = "zn_ping")]
struct Opt {
    #[structopt(short = "e", long = "peer")]
    peer: Option<String>,
    #[structopt(short = "m", long = "mode")]
    mode: String,
    #[structopt(short = "s", long = "scout")]
    scout: bool,
    #[structopt(short = "p", long = "payload")]
    payload: usize,
    #[structopt(short = "i", long = "interval")]
    interval: f64,
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
        config.insert("peer".to_string(), opt.peer.unwrap().to_string());
    }

    let session = open(config.into()).await.unwrap();
    let session = Arc::new(session);

    // The hashmap with the pings
    let pending = Arc::new(Mutex::new(HashMap::<u64, Instant>::new()));
    let barrier = Arc::new(Barrier::new(2));

    let c_pending = pending.clone();
    let c_barrier = barrier.clone();
    let c_session = session.clone();
    task::spawn(async move {
        // The resource to wait the response back
        let reskey_pong = RId(c_session
            .declare_resource(&RName("/test/pong".to_string()))
            .await
            .unwrap());

        let sub_info = SubInfo {
            reliability: Reliability::Reliable,
            mode: SubMode::Push,
            period: None,
        };
        let mut sub = c_session
            .declare_subscriber(&reskey_pong, &sub_info)
            .await
            .unwrap();

        // Wait for the both publishers and subscribers to be declared
        c_barrier.wait().await;

        while let Some(mut sample) = sub.stream().next().await {
            let mut count_bytes = [0u8; 8];
            sample.payload.read_bytes(&mut count_bytes);
            let count = u64::from_le_bytes(count_bytes);
            let instant = c_pending.lock().await.remove(&count).unwrap();
            println!(
                "{} bytes: seq={} time={:?}",
                sample.payload.len(),
                count,
                instant.elapsed()
            );
        }
    });

    // The resource to publish data on
    let reskey_ping = RId(session
        .declare_resource(&RName("/test/ping".to_string()))
        .await
        .unwrap());
    let _publ = session.declare_publisher(&reskey_ping).await.unwrap();

    // Wait for the both publishers and subscribers to be declared
    barrier.wait().await;

    let payload = vec![0u8; opt.payload - 8];
    let mut count: u64 = 0;
    loop {
        let mut data: WBuf = WBuf::new(opt.payload, true);
        let count_bytes: [u8; 8] = count.to_le_bytes();
        data.write_bytes(&count_bytes);
        data.write_bytes(&payload);

        let data: RBuf = data.into();

        pending.lock().await.insert(count, Instant::now());
        session
            .write_ext(
                &reskey_ping,
                data,
                encoding::DEFAULT,
                data_kind::DEFAULT,
                CongestionControl::Block, // Make sure to not drop messages because of congestion control
            )
            .await
            .unwrap();

        task::sleep(Duration::from_secs_f64(opt.interval)).await;
        count += 1;
    }
}
