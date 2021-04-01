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
#[structopt(name = "zn_overhead")]
struct Opt {
    #[structopt(short = "e", long = "peer")]
    peer: Option<String>,
    #[structopt(short = "m", long = "mode")]
    mode: String,
    #[structopt(short = "s", long = "scout")]
    scout: bool,
    #[structopt(short = "p", long = "payload")]
    payload: usize,
    #[structopt(short = "v", long = "verbose")]
    print: bool,
    #[structopt(short = "t", long = "total", default_value = "1048576")] //1MB in bytes
    total: u64,
    #[structopt(short = "i", long = "interval", default_value = "0")]
    interval: f64,
}

#[async_std::main]
async fn main() {
    // initiate logging
    env_logger::init();

    // Parse the args
    let opt = Opt::from_args();

    let bytes_in_mb: u64 = 1048576;

    let mut config = Properties::default();
    config.insert("mode".to_string(), opt.mode.clone());
    config.insert("add_timestamp".to_string(), "false".to_string());

    if opt.scout {
        config.insert("multicast_scouting".to_string(), "true".to_string());
    } else {
        config.insert("multicast_scouting".to_string(), "false".to_string());
        config.insert("peer".to_string(), opt.peer.unwrap());
    }

    let session = open(config.into()).await.unwrap();

    let reskey = RId(session
        .declare_resource(&RName("/test/overhead".to_string()))
        .await
        .unwrap());
    let _publ = session.declare_publisher(&reskey).await.unwrap();

    let data: RBuf = (0usize..opt.payload)
        .map(|i| (i % 10) as u8)
        .collect::<Vec<u8>>()
        .into();

    let mut i: u64 = 0;
    let tot: u64 = (opt.total * bytes_in_mb) / (opt.payload as u64);

    if opt.print {
        let count = Arc::new(AtomicUsize::new(0));
        let c_count = count.clone();
        task::spawn(async move {
            loop {
                task::sleep(Duration::from_secs(1)).await;
                let c = count.swap(0, Ordering::Relaxed);
                if c > 0 {
                    println!("{} msg/s", c);
                }
            }
        });

        while i < tot {
            session
                .write_ext(
                    &reskey,
                    data.clone(),
                    encoding::DEFAULT,
                    data_kind::DEFAULT,
                    CongestionControl::Block, // Make sure to not drop messages because of congestion control
                )
                .await
                .unwrap();
            c_count.fetch_add(1, Ordering::Relaxed);
            i += 1;
            task::sleep(Duration::from_secs_f64(opt.interval)).await;
        }
    } else {
        while i < tot {
            session
                .write_ext(
                    &reskey,
                    data.clone(),
                    encoding::DEFAULT,
                    data_kind::DEFAULT,
                    CongestionControl::Block, // Make sure to not drop messages because of congestion control
                )
                .await
                .unwrap();
            i += 1;
            task::sleep(Duration::from_secs_f64(opt.interval)).await;
        }
    }
}
