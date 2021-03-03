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
use zenoh_protocol::core::{CongestionControl, Reliability, ResKey};
use zenoh_protocol::io::RBuf;
use zenoh_protocol::session::{DummySessionEventHandler, Mux};
use zenoh_router::runtime::Runtime;
use zenoh_util::properties::config::{
    ConfigProperties, ZN_ADD_TIMESTAMP_KEY, ZN_MODE_KEY, ZN_MULTICAST_SCOUTING_KEY, ZN_PEER_KEY,
};

#[derive(Debug, StructOpt)]
#[structopt(name = "r_pub_thr")]
struct Opt {
    #[structopt(short = "e", long = "peer")]
    peer: Option<String>,
    #[structopt(short = "m", long = "mode")]
    mode: String,
    #[structopt(short = "s", long = "scout")]
    scout: bool,
    #[structopt(short = "p", long = "payload")]
    payload: usize,
    #[structopt(short = "t", long = "print")]
    print: bool,
}

#[async_std::main]
async fn main() {
    // Enable logging
    env_logger::init();

    // Parse the args
    let opt = Opt::from_args();

    let mut config = ConfigProperties::default();
    config.insert(ZN_MODE_KEY, opt.mode.clone());
    config.insert(ZN_ADD_TIMESTAMP_KEY, "false".to_string());

    if opt.scout {
        config.insert(ZN_MULTICAST_SCOUTING_KEY, "true".to_string());
    } else {
        config.insert(ZN_MULTICAST_SCOUTING_KEY, "false".to_string());
        config.insert(ZN_PEER_KEY, opt.peer.unwrap());
    }

    let my_primitives = Arc::new(Mux::new(Arc::new(DummySessionEventHandler::new())));

    let runtime = Runtime::new(0u8, config, None).await.unwrap();
    let primitives = runtime
        .read()
        .await
        .router
        .new_primitives(my_primitives)
        .await;

    primitives.resource(1, &"/tp".to_string().into()).await;
    let rid = ResKey::RId(1);
    primitives.publisher(&rid, None).await;

    // @TODO: Fix writer starvation in the RwLock and remove this sleep
    // Wait for the declare to arrive
    task::sleep(Duration::from_millis(1_000)).await;

    let payload = RBuf::from(vec![0u8; opt.payload]);
    if opt.print {
        let count = Arc::new(AtomicUsize::new(0));
        let c_count = count.clone();
        task::spawn(async move {
            loop {
                task::sleep(Duration::from_secs(1)).await;
                let c = count.swap(0, Ordering::AcqRel);
                if c > 0 {
                    println!("{} msg/s", c);
                }
            }
        });

        loop {
            primitives
                .data(
                    &rid,
                    payload.clone(),
                    Reliability::Reliable,
                    CongestionControl::Block,
                    None,
                    None,
                )
                .await;
            c_count.fetch_add(1, Ordering::AcqRel);
        }
    } else {
        loop {
            primitives
                .data(
                    &rid,
                    payload.clone(),
                    Reliability::Reliable,
                    CongestionControl::Block,
                    None,
                    None,
                )
                .await;
        }
    }
}
