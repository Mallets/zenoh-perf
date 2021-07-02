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
use std::convert::TryFrom;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;
use structopt::StructOpt;
use zenoh::net::ZBuf;
use zenoh::Properties;
use zenoh::*;

#[derive(Debug, StructOpt)]
#[structopt(name = "z_put_thr")]
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
    // Initiate logging
    env_logger::init();

    // Parse the args
    let opt = Opt::from_args();

    let mut config = match opt.config.as_ref() {
        Some(f) => {
            let config = async_std::fs::read_to_string(f).await.unwrap();
            Properties::from(config)
        }
        None => Properties::default(),
    };
    config.insert("mode".to_string(), opt.mode.clone());
    config.insert("add_timestamp".to_string(), "false".to_string());

    config.insert("multicast_scouting".to_string(), "false".to_string());
    config.insert("locator".to_string(), opt.locator);

    let data: ZBuf = (0usize..opt.payload)
        .map(|i| (i % 10) as u8)
        .collect::<Vec<u8>>()
        .into();

    let zenoh = Zenoh::new(config.into()).await.unwrap();
    let workspace = zenoh.workspace(None).await.unwrap();

    let path: Path = Path::try_from("/test/thr").unwrap();
    let value = Value::from(data);

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
            workspace.put(&path, value.clone()).await.unwrap();
            c_count.fetch_add(1, Ordering::Relaxed);
        }
    } else {
        loop {
            workspace.put(&path, value.clone()).await.unwrap();
        }
    }
}
