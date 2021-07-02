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
use std::convert::TryFrom;
use structopt::StructOpt;
use zenoh::*;

#[derive(Debug, StructOpt)]
#[structopt(name = "z_pong")]
struct Opt {
    #[structopt(short = "l", long = "locator")]
    locator: String,
    #[structopt(short = "m", long = "mode")]
    mode: String,
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

    config.insert("multicast_scouting".to_string(), "false".to_string());
    match opt.mode.as_str() {
        "peer" => config.insert("listener".to_string(), opt.locator),
        "client" => config.insert("peer".to_string(), opt.locator),
        _ => panic!("Unsupported mode: {}", opt.mode),
    };

    let zenoh = Zenoh::new(config.into()).await.unwrap();
    let workspace = zenoh.workspace(None).await.unwrap();
    let path = &Path::try_from("/test/query").unwrap();
    let mut get_stream = workspace.register_eval(&path.into()).await.unwrap();
    while let Some(get_request) = get_stream.next().await {
        let data = vec![0u8; opt.payload];
        get_request.reply(path.clone(), data.into());
    }

    get_stream.close().await.unwrap();
    zenoh.close().await.unwrap();
}
