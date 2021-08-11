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
use std::time::Duration;
use std::time::Instant;
use structopt::StructOpt;
use zenoh::net::protocol::core::{
    Channel, PeerId, Priority, QueryConsolidation, QueryTarget, Reliability, ResKey, SubInfo,
    SubMode, ZInt,
};
use zenoh::net::protocol::io::{WBuf, ZBuf};
use zenoh::net::protocol::proto::{DataInfo, RoutingContext};
use zenoh::net::protocol::session::Primitives;
use zenoh::net::runtime::Runtime;
use zenoh_util::properties::config::{
    ConfigProperties, ZN_MODE_KEY, ZN_MULTICAST_SCOUTING_KEY, ZN_PEER_KEY,
};

// Primitives for the non-blocking locator
struct LatencyPrimitivesParallel {
    scenario: String,
    name: String,
    interval: f64,
    pending: Arc<Mutex<HashMap<u64, Instant>>>,
}

impl LatencyPrimitivesParallel {
    pub fn new(
        scenario: String,
        name: String,
        interval: f64,
        pending: Arc<Mutex<HashMap<u64, Instant>>>,
    ) -> Self {
        Self {
            scenario,
            name,
            interval,
            pending,
        }
    }
}

impl Primitives for LatencyPrimitivesParallel {
    fn decl_resource(&self, _rid: ZInt, _reskey: &ResKey) {}
    fn forget_resource(&self, _rid: ZInt) {}
    fn decl_publisher(&self, _reskey: &ResKey, _routing_context: Option<RoutingContext>) {}
    fn forget_publisher(&self, _reskey: &ResKey, _routing_context: Option<RoutingContext>) {}
    fn decl_subscriber(
        &self,
        _reskey: &ResKey,
        _sub_info: &SubInfo,
        _routing_context: Option<RoutingContext>,
    ) {
    }
    fn forget_subscriber(&self, _reskey: &ResKey, _routing_context: Option<RoutingContext>) {}
    fn decl_queryable(
        &self,
        _reskey: &ResKey,
        _kind: ZInt,
        _routing_context: Option<RoutingContext>,
    ) {
    }
    fn forget_queryable(&self, _reskey: &ResKey, _routing_context: Option<RoutingContext>) {}

    fn send_data(
        &self,
        _reskey: &ResKey,
        mut payload: ZBuf,
        _channel: Channel,
        _data_info: Option<DataInfo>,
        _routing_context: Option<RoutingContext>,
    ) {
        let mut count_bytes = [0u8; 8];
        payload.read_bytes(&mut count_bytes);
        let count = u64::from_le_bytes(count_bytes);
        let instant = self.pending.lock().unwrap().remove(&count).unwrap();
        println!(
            "router,{},latency.parallel,{},{},{},{},{}",
            self.scenario,
            self.name,
            payload.len(),
            self.interval,
            count,
            instant.elapsed().as_micros()
        );
    }

    fn send_query(
        &self,
        _reskey: &ResKey,
        _predicate: &str,
        _qid: ZInt,
        _target: QueryTarget,
        _consolidation: QueryConsolidation,
        _routing_context: Option<RoutingContext>,
    ) {
    }
    fn send_reply_data(
        &self,
        _qid: ZInt,
        _source_kind: ZInt,
        _replier_id: PeerId,
        _reskey: ResKey,
        _info: Option<DataInfo>,
        _payload: ZBuf,
    ) {
    }
    fn send_reply_final(&self, _qid: ZInt) {}
    fn send_pull(
        &self,
        _is_final: bool,
        _reskey: &ResKey,
        _pull_id: ZInt,
        _max_samples: &Option<ZInt>,
    ) {
    }
    fn send_close(&self) {}
}

// Primitives for the blocking locator
struct LatencyPrimitivesSequential {
    pending: Arc<Mutex<HashMap<u64, Arc<Barrier>>>>,
}

impl LatencyPrimitivesSequential {
    pub fn new(pending: Arc<Mutex<HashMap<u64, Arc<Barrier>>>>) -> Self {
        Self { pending }
    }
}

impl Primitives for LatencyPrimitivesSequential {
    fn decl_resource(&self, _rid: ZInt, _reskey: &ResKey) {}
    fn forget_resource(&self, _rid: ZInt) {}
    fn decl_publisher(&self, _reskey: &ResKey, _routing_context: Option<RoutingContext>) {}
    fn forget_publisher(&self, _reskey: &ResKey, _routing_context: Option<RoutingContext>) {}
    fn decl_subscriber(
        &self,
        _reskey: &ResKey,
        _sub_info: &SubInfo,
        _routing_context: Option<RoutingContext>,
    ) {
    }
    fn forget_subscriber(&self, _reskey: &ResKey, _routing_context: Option<RoutingContext>) {}
    fn decl_queryable(
        &self,
        _reskey: &ResKey,
        _kind: ZInt,
        _routing_context: Option<RoutingContext>,
    ) {
    }
    fn forget_queryable(&self, _reskey: &ResKey, _routing_context: Option<RoutingContext>) {}

    fn send_data(
        &self,
        _reskey: &ResKey,
        mut payload: ZBuf,
        _channel: Channel,
        _data_info: Option<DataInfo>,
        _routing_context: Option<RoutingContext>,
    ) {
        let mut count_bytes = [0u8; 8];
        payload.read_bytes(&mut count_bytes);
        let count = u64::from_le_bytes(count_bytes);
        let barrier = self.pending.lock().unwrap().remove(&count).unwrap();
        barrier.wait();
    }

    fn send_query(
        &self,
        _reskey: &ResKey,
        _predicate: &str,
        _qid: ZInt,
        _target: QueryTarget,
        _consolidation: QueryConsolidation,
        _routing_context: Option<RoutingContext>,
    ) {
    }
    fn send_reply_data(
        &self,
        _qid: ZInt,
        _source_kind: ZInt,
        _replier_id: PeerId,
        _reskey: ResKey,
        _info: Option<DataInfo>,
        _payload: ZBuf,
    ) {
    }
    fn send_reply_final(&self, _qid: ZInt) {}
    fn send_pull(
        &self,
        _is_final: bool,
        _reskey: &ResKey,
        _pull_id: ZInt,
        _max_samples: &Option<ZInt>,
    ) {
    }
    fn send_close(&self) {}
}

#[derive(Debug, StructOpt)]
#[structopt(name = "r_pub_thr")]
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
}

async fn parallel(opt: Opt, config: ConfigProperties) {
    let pending: Arc<Mutex<HashMap<u64, Instant>>> = Arc::new(Mutex::new(HashMap::new()));

    let runtime = Runtime::new(0u8, config, None).await.unwrap();
    let rx_primitives = Arc::new(LatencyPrimitivesParallel::new(
        opt.scenario,
        opt.name,
        opt.interval,
        pending.clone(),
    ));
    let tx_primitives = runtime.router.new_primitives(rx_primitives);

    let rid = ResKey::RName("/test/pong".to_string());
    let sub_info = SubInfo {
        reliability: Reliability::Reliable,
        mode: SubMode::Push,
        period: None,
    };
    tx_primitives.decl_subscriber(&rid, &sub_info, None);

    let channel = Channel {
        priority: Priority::Data,
        reliability: Reliability::Reliable,
    };
    let payload = vec![0u8; opt.payload - 8];
    let mut count: u64 = 0;
    let reskey = ResKey::RName("/test/ping".to_string());
    loop {
        // Create and send the message
        let mut data: WBuf = WBuf::new(opt.payload, true);
        let count_bytes: [u8; 8] = count.to_le_bytes();
        data.write_bytes(&count_bytes);
        data.write_bytes(&payload);
        let data: ZBuf = data.into();

        // Insert the pending ping
        pending.lock().unwrap().insert(count, Instant::now());

        tx_primitives.send_data(&reskey, data, channel, None, None);

        task::sleep(Duration::from_secs_f64(opt.interval)).await;
        count += 1;
    }
}

async fn single(opt: Opt, config: ConfigProperties) {
    let pending: Arc<Mutex<HashMap<u64, Arc<Barrier>>>> = Arc::new(Mutex::new(HashMap::new()));

    let runtime = Runtime::new(0u8, config, None).await.unwrap();
    let rx_primitives = Arc::new(LatencyPrimitivesSequential::new(pending.clone()));
    let tx_primitives = runtime.router.new_primitives(rx_primitives);

    let rid = ResKey::RName("/test/pong".to_string());
    let sub_info = SubInfo {
        reliability: Reliability::Reliable,
        mode: SubMode::Push,
        period: None,
    };
    tx_primitives.decl_subscriber(&rid, &sub_info, None);

    let channel = Channel {
        priority: Priority::Data,
        reliability: Reliability::Reliable,
    };
    let payload = vec![0u8; opt.payload - 8];
    let mut count: u64 = 0;
    let reskey = ResKey::RName("/test/ping".to_string());
    loop {
        // Create and send the message
        let mut data: WBuf = WBuf::new(opt.payload, true);
        let count_bytes: [u8; 8] = count.to_le_bytes();
        data.write_bytes(&count_bytes);
        data.write_bytes(&payload);
        let data: ZBuf = data.into();

        // Insert the pending ping
        let barrier = Arc::new(Barrier::new(2));
        pending.lock().unwrap().insert(count, barrier.clone());

        let now = Instant::now();
        tx_primitives.send_data(&reskey, data, channel, None, None);
        barrier.wait();
        println!(
            "router,{},latency.sequential,{},{},{},{},{}",
            opt.scenario,
            opt.name,
            payload.len(),
            opt.interval,
            count,
            now.elapsed().as_micros()
        );

        task::sleep(Duration::from_secs_f64(opt.interval)).await;
        count += 1;
    }
}

#[async_std::main]
async fn main() {
    // Enable logging
    env_logger::init();

    // Parse the args
    let opt = Opt::from_args();

    let mut config = ConfigProperties::default();
    config.insert(ZN_MODE_KEY, opt.mode.clone());

    config.insert(ZN_MULTICAST_SCOUTING_KEY, "false".to_string());
    config.insert(ZN_PEER_KEY, opt.locator.clone());

    if opt.parallel {
        parallel(opt, config).await;
    } else {
        single(opt, config).await;
    }
}
