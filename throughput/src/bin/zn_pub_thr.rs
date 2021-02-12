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
use structopt::StructOpt;
use zenoh::net::ResKey::*;
use zenoh::net::*;
use zenoh::Properties;

#[derive(Debug, StructOpt)]
#[structopt(name = "zn_pub_thr")]
struct Opt {
    #[structopt(short = "e", long = "peer")]
    peer: Option<String>,
    #[structopt(short = "m", long = "mode")]
    mode: String,
    #[structopt(short = "s", long = "scout")]
    scout: bool,
    #[structopt(short = "p", long = "payload")]
    payload: usize,
}

#[async_std::main]
async fn main() {
    // initiate logging
    env_logger::init();

    // Parse the args
    let opt = Opt::from_args();

    let mut config = Properties::default();
    config.insert("mode".to_string(), opt.mode.clone());
    config.insert("add_timestamp".to_string(), "false".to_string());

    if opt.scout {
        config.insert("multicast_scouting".to_string(), "true".to_string());
    } else {
        config.insert("multicast_scouting".to_string(), "false".to_string());
        config.insert("peer".to_string(), opt.peer.unwrap().to_string());
    }

    let session = open(config.into()).await.unwrap();

    let reskey = RId(session
        .declare_resource(&RName("/test/thr".to_string()))
        .await
        .unwrap());
    let _publ = session.declare_publisher(&reskey).await.unwrap();

    let data: RBuf = (0usize..opt.payload)
        .map(|i| (i % 10) as u8)
        .collect::<Vec<u8>>()
        .into();
    loop {
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
    }
}
