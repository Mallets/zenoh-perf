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
use zenoh::net::queryable::EVAL;
use zenoh::net::*;
use zenoh::Properties;

#[derive(Debug, StructOpt)]
#[structopt(name = "zn_eval")]
struct Opt {
    #[structopt(short = "l", long = "locator")]
    locator: Option<String>,
    #[structopt(short = "m", long = "mode")]
    mode: String,
    #[structopt(short = "u", long = "scout")]
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

    if opt.scout {
        config.insert("multicast_scouting".to_string(), "true".to_string());
    } else {
        config.insert("multicast_scouting".to_string(), "false".to_string());
        match opt.mode.as_str() {
            "peer" => config.insert("listener".to_string(), opt.locator.unwrap()),
            "client" => config.insert("peer".to_string(), opt.locator.unwrap()),
            _ => panic!("Unsupported mode: {}", opt.mode),
        };
    }

    let session = open(config.into()).await.unwrap();

    // The resource to read the data from
    let path = "/test/query".to_string();
    let reskey = ResKey::RName(path.clone());
    let mut queryable = session.declare_queryable(&reskey, EVAL).await.unwrap();
    while let Ok(query) = queryable.receiver().recv() {
        query.reply(Sample {
            res_name: path.clone(),
            payload: vec![0u8; opt.payload].into(),
            data_info: None,
        });
    }
}
