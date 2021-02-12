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
use std::convert::TryFrom;
use structopt::StructOpt;
use zenoh::net::RBuf;
use zenoh::Properties;
use zenoh::*;

#[derive(Debug, StructOpt)]
#[structopt(name = "z_put_thr")]
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
    // Initiate logging
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

    let data: RBuf = (0usize..opt.payload)
        .map(|i| (i % 10) as u8)
        .collect::<Vec<u8>>()
        .into();

    let zenoh = Zenoh::new(config.into()).await.unwrap();
    let workspace = zenoh.workspace(None).await.unwrap();

    let path: Path = Path::try_from("/test/thr").unwrap();
    let value = Value::from(data);
    loop {
        workspace.put(&path, value.clone()).await.unwrap();
    }
}
