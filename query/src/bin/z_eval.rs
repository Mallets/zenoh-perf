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
use async_std::future;
use async_std::stream::StreamExt;
use std::convert::TryInto;
use structopt::StructOpt;
use zenoh::*;

#[derive(Debug, StructOpt)]
#[structopt(name = "z_pong")]
struct Opt {
    #[structopt(short = "l", long = "locator")]
    locator: Option<String>,
    #[structopt(short = "m", long = "mode")]
    mode: String,
    #[structopt(short = "s", long = "scout")]
    scout: bool,
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

    let zenoh = Zenoh::new(config.into()).await.unwrap();
    let workspace = zenoh.workspace(None).await.unwrap();
    let mut sub = workspace
        .subscribe(&"/test/ping/".to_string().try_into().unwrap())
        .await
        .unwrap();

    while let Some(change) = sub.next().await {
        match change.value.unwrap() {
            Value::Raw(_, payload) => {
                workspace
                    .put(&"/test/pong".try_into().unwrap(), payload.into())
                    .await
                    .unwrap();
            }
            _ => panic!("Invalid value"),
        }
    }

    // Stop forever
    future::pending::<()>().await;
}
