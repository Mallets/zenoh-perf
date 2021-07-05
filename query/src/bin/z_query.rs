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
use std::convert::TryInto;
use std::time::Instant;
use structopt::StructOpt;
use zenoh::*;

#[derive(Debug, StructOpt)]
#[structopt(name = "z_query")]
struct Opt {
    #[structopt(short = "l", long = "locator")]
    locator: String,
    #[structopt(short = "m", long = "mode")]
    mode: String,
    #[structopt(short = "n", long = "name")]
    name: String,
    #[structopt(short = "s", long = "scenario")]
    scenario: String,
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
    config.insert("peer".to_string(), opt.locator);

    let zenoh = Zenoh::new(config.into()).await.unwrap();
    let workspace = zenoh.workspace(None).await.unwrap();

    let mut count: u64 = 0;
    loop {
        let selector = "/test/query".to_string();
        let now = Instant::now();
        let mut data_stream = workspace.get(&selector.try_into().unwrap()).await.unwrap();

        let mut payload: usize = 0;
        while let Some(data) = data_stream.next().await {
            let len = match data.value {
                Value::Raw(_, payload) => payload.len(),
                Value::Custom {
                    encoding_descr: _,
                    data: payload,
                } => payload.len(),
                Value::StringUtf8(payload) => payload.as_bytes().len(),
                Value::Properties(ps) => {
                    let mut len: usize = 0;
                    for p in ps.iter() {
                        let (a, b) = (p.0, p.1);
                        len += a.as_bytes().len() + b.as_bytes().len();
                    }
                    len
                }
                Value::Json(payload) => payload.as_bytes().len(),
                Value::Integer(_) => std::mem::size_of::<i64>(),
                Value::Float(_) => std::mem::size_of::<f64>(),
            };
            payload += len;
        }

        println!(
            "zenoh,{},query.latency,{},{},{},{}",
            opt.scenario,
            opt.name,
            payload,
            count,
            now.elapsed().as_micros()
        );
        count += 1;
    }
}
