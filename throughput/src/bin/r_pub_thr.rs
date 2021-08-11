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
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;
use structopt::StructOpt;
use zenoh::net::protocol::core::{Channel, Priority, Reliability, ResKey};
use zenoh::net::protocol::io::ZBuf;
use zenoh::net::protocol::session::DummyPrimitives;
use zenoh::net::protocol::session::Primitives;
use zenoh::net::runtime::Runtime;
use zenoh_util::properties::config::{
    ConfigProperties, ZN_ADD_TIMESTAMP_KEY, ZN_MODE_KEY, ZN_MULTICAST_SCOUTING_KEY, ZN_PEER_KEY,
};
use zenoh_util::properties::{IntKeyProperties, Properties};

#[derive(Debug, StructOpt)]
#[structopt(name = "r_pub_thr")]
struct Opt {
    #[structopt(short = "l", long = "locator")]
    locator: String,
    #[structopt(short = "m", long = "mode")]
    mode: String,
    #[structopt(short = "p", long = "payload")]
    payload: usize,
    #[structopt(short = "t", long = "print")]
    print: bool,
    #[structopt(short = "c", long = "conf", parse(from_os_str))]
    config: Option<PathBuf>,
}

#[async_std::main]
async fn main() {
    // Enable logging
    env_logger::init();

    // Parse the args
    let opt = Opt::from_args();

    let mut config = match opt.config.as_ref() {
        Some(f) => {
            let config = async_std::fs::read_to_string(f).await.unwrap();
            let properties = Properties::from(config);
            IntKeyProperties::from(properties)
        }
        None => ConfigProperties::default(),
    };
    config.insert(ZN_MODE_KEY, opt.mode.clone());
    config.insert(ZN_ADD_TIMESTAMP_KEY, "false".to_string());

    config.insert(ZN_MULTICAST_SCOUTING_KEY, "false".to_string());
    config.insert(ZN_PEER_KEY, opt.locator);

    let my_primitives = Arc::new(DummyPrimitives::new());

    let runtime = Runtime::new(0u8, config, None).await.unwrap();
    let primitives = runtime.router.new_primitives(my_primitives);

    primitives.decl_resource(1, &"/test/thr".to_string().into());
    let rid = ResKey::RId(1);
    primitives.decl_publisher(&rid, None);

    // @TODO: Fix writer starvation in the RwLock and remove this sleep
    // Wait for the declare to arrive
    task::sleep(Duration::from_millis(1_000)).await;

    let channel = Channel {
        priority: Priority::Data,
        reliability: Reliability::Reliable,
    };
    let payload = ZBuf::from(vec![0u8; opt.payload]);
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

        loop {
            primitives.send_data(&rid, payload.clone(), channel, None, None);
            c_count.fetch_add(1, Ordering::Relaxed);
        }
    } else {
        loop {
            primitives.send_data(&rid, payload.clone(), channel, None, None);
        }
    }
}
