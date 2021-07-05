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
use async_std::task;
use std::collections::HashMap;
use std::sync::{Arc, Barrier, Mutex};
use std::time::{Duration, Instant};
use structopt::StructOpt;
use zenoh::net::ResKey::*;
use zenoh::net::*;
use zenoh::Properties;

#[derive(Debug, StructOpt)]
#[structopt(name = "zn_ping")]
struct Opt {
    #[structopt(short = "l", long = "locator")]
    locator: String,
    #[structopt(short = "m", long = "mode")]
    mode: String,
    #[structopt(short = "p", long = "payload")]
    payload: usize,
    #[structopt(short = "n", long = "name")]
    name: String,
    #[structopt(short = "s", long = "scenario")]
    scenario: String,
    #[structopt(short = "i", long = "interval")]
    interval: f64,
    #[structopt(long = "parallel")]
    parallel: bool,
    #[structopt(short = "d", long = "samples")]
    samples: Option<usize>,
}

async fn single(opt: Opt, config: Properties) {
    let session = open(config.into()).await.unwrap();

    // The resource to wait the response back
    let reskey_pong = RId(session
        .declare_resource(&RName("/test/pong".to_string()))
        .await
        .unwrap());
    let sub_info = SubInfo {
        reliability: Reliability::Reliable,
        mode: SubMode::Push,
        period: None,
    };
    let mut sub = session
        .declare_subscriber(&reskey_pong, &sub_info)
        .await
        .unwrap();

    // The resource to publish data on
    let reskey_ping = RId(session
        .declare_resource(&RName("/test/ping".to_string()))
        .await
        .unwrap());
    let _publ = session.declare_publisher(&reskey_ping).await.unwrap();

    let sleep = Duration::from_secs_f64(opt.interval);
    let payload = vec![0u8; opt.payload - 8];
    let mut count: u64 = 0;
    loop {
        let mut data: WBuf = WBuf::new(opt.payload, true);
        let count_bytes: [u8; 8] = count.to_le_bytes();
        data.write_bytes(&count_bytes);
        data.write_bytes(&payload);
        let data: ZBuf = data.into();

        let now = Instant::now();
        session
            .write_ext(
                &reskey_ping,
                data,
                encoding::DEFAULT,
                data_kind::DEFAULT,
                CongestionControl::Block, // Make sure to not drop messages because of congestion control
            )
            .wait()
            .unwrap();

        let mut sample = sub.receiver().recv().unwrap();
        let mut count_bytes = [0u8; 8];
        sample.payload.read_bytes(&mut count_bytes);
        let s_count = u64::from_le_bytes(count_bytes);
        println!(
            "zenoh-net,{},latency.sequential,{},{},{},{},{}",
            opt.scenario,
            opt.name,
            sample.payload.len(),
            opt.interval,
            s_count,
            now.elapsed().as_micros()
        );

        task::sleep(sleep).await;
        count += 1;
    }
}

async fn parallel(opt: Opt, config: Properties) {
    let session = open(config.into()).await.unwrap();
    let session = Arc::new(session);

    // The hashmap with the pings
    let pending = Arc::new(Mutex::new(HashMap::<u64, Instant>::new()));
    let barrier = Arc::new(Barrier::new(2));

    let c_pending = pending.clone();
    let c_barrier = barrier.clone();
    let c_session = session.clone();
    let scenario = opt.scenario;
    let name = opt.name;
    let interval = opt.interval;
    task::spawn(async move {
        // The resource to wait the response back
        let reskey_pong = RId(c_session
            .declare_resource(&RName("/test/pong".to_string()))
            .await
            .unwrap());

        let sub_info = SubInfo {
            reliability: Reliability::Reliable,
            mode: SubMode::Push,
            period: None,
        };
        let mut sub = c_session
            .declare_subscriber(&reskey_pong, &sub_info)
            .await
            .unwrap();

        // Wait for the both publishers and subscribers to be declared
        c_barrier.wait();

        while let Ok(mut sample) = sub.receiver().recv() {
            let mut count_bytes = [0u8; 8];
            sample.payload.read_bytes(&mut count_bytes);
            let count = u64::from_le_bytes(count_bytes);
            let instant = c_pending.lock().unwrap().remove(&count).unwrap();
            println!(
                "zenoh-net,{},latency.parallel,{},{},{},{},{}",
                scenario,
                name,
                sample.payload.len(),
                interval,
                count,
                instant.elapsed().as_micros()
            );
        }
    });

    // The resource to publish data on
    let reskey_ping = RId(session
        .declare_resource(&RName("/test/ping".to_string()))
        .await
        .unwrap());
    let _publ = session.declare_publisher(&reskey_ping).await.unwrap();

    // Wait for the both publishers and subscribers to be declared
    barrier.wait();

    let sleep = Duration::from_secs_f64(opt.interval);
    let payload = vec![0u8; opt.payload - 8];
    let mut count: u64 = 0;
    loop {
        let mut data: WBuf = WBuf::new(opt.payload, true);
        let count_bytes: [u8; 8] = count.to_le_bytes();
        data.write_bytes(&count_bytes);
        data.write_bytes(&payload);

        let data: ZBuf = data.into();

        pending.lock().unwrap().insert(count, Instant::now());
        session
            .write_ext(
                &reskey_ping,
                data,
                encoding::DEFAULT,
                data_kind::DEFAULT,
                CongestionControl::Block, // Make sure to not drop messages because of congestion control
            )
            .wait()
            .unwrap();

        task::sleep(sleep).await;
        count += 1;
    }
}

async fn samples(opt: Opt, config: Properties) {
    let session = open(config.into()).await.unwrap();

    // The resource to wait the response back
    let reskey_pong = RId(session
        .declare_resource(&RName("/test/pong".to_string()))
        .await
        .unwrap());
    let sub_info = SubInfo {
        reliability: Reliability::Reliable,
        mode: SubMode::Push,
        period: None,
    };

    let barrier = Arc::new(Barrier::new(2));
    let c_barrier = barrier.clone();
    let _sub = session
        .declare_callback_subscriber(&reskey_pong, &sub_info, move |_sample| {
            c_barrier.wait();
        })
        .await
        .unwrap();

    // The resource to publish data on
    let reskey_ping = RId(session
        .declare_resource(&RName("/test/ping".to_string()))
        .await
        .unwrap());
    let _publ = session.declare_publisher(&reskey_ping).await.unwrap();

    let mut samples = vec![0u128; opt.samples.unwrap()];

    let sleep = Duration::from_secs_f64(opt.interval);
    let data: ZBuf = vec![0u8; opt.payload].into();

    for i in 0..opt.samples.unwrap() {
        let now = Instant::now();
        session
            .write_ext(
                &reskey_ping,
                data.clone(),
                encoding::DEFAULT,
                data_kind::DEFAULT,
                CongestionControl::Block, // Make sure to not drop messages because of congestion control
            )
            .wait()
            .unwrap();

        barrier.wait();
        samples[i] = now.elapsed().as_micros();
        task::sleep(sleep).await;
    }

    let num = opt.samples.unwrap();
    for (i, s) in samples.iter().enumerate().take(num) {
        println!(
            "zenoh-net,{},latency.sequential.samples,{},{},{},{},{}",
            opt.scenario, opt.name, opt.payload, opt.interval, i, s
        );
    }
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
    config.insert("peer".to_string(), opt.locator.clone());

    if opt.parallel {
        parallel(opt, config).await;
        return;
    }
    if opt.samples.is_some() {
        samples(opt, config).await;
        return;
    }

    single(opt, config).await;
}
