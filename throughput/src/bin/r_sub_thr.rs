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
use std::time::{Duration, Instant};
use structopt::StructOpt;
use zenoh::net::protocol::core::{
    Channel, PeerId, QueryConsolidation, QueryTarget, Reliability, ResKey, SubInfo, SubMode, ZInt,
};
use zenoh::net::protocol::io::ZBuf;
use zenoh::net::protocol::proto::{DataInfo, RoutingContext};
use zenoh::net::protocol::session::Primitives;
use zenoh::net::runtime::Runtime;
use zenoh_util::properties::config::{
    ConfigProperties, ZN_LISTENER_KEY, ZN_MODE_KEY, ZN_MULTICAST_SCOUTING_KEY, ZN_PEER_KEY,
};
use zenoh_util::properties::{IntKeyProperties, Properties};

struct ThroughputPrimitives {
    count: Arc<AtomicUsize>,
}

impl ThroughputPrimitives {
    pub fn new(count: Arc<AtomicUsize>) -> ThroughputPrimitives {
        ThroughputPrimitives { count }
    }
}

impl Primitives for ThroughputPrimitives {
    fn decl_resource(&self, _rid: ZInt, _reskey: &ResKey) {
        self.count.fetch_add(1, Ordering::Relaxed);
    }

    fn forget_resource(&self, _rid: ZInt) {
        self.count.fetch_add(1, Ordering::Relaxed);
    }

    fn decl_publisher(&self, _reskey: &ResKey, _routing_context: Option<RoutingContext>) {
        self.count.fetch_add(1, Ordering::Relaxed);
    }

    fn forget_publisher(&self, _reskey: &ResKey, _routing_context: Option<RoutingContext>) {
        self.count.fetch_add(1, Ordering::Relaxed);
    }

    fn decl_subscriber(
        &self,
        _reskey: &ResKey,
        _sub_info: &SubInfo,
        _routing_context: Option<RoutingContext>,
    ) {
        self.count.fetch_add(1, Ordering::Relaxed);
    }

    fn forget_subscriber(&self, _reskey: &ResKey, _routing_context: Option<RoutingContext>) {
        self.count.fetch_add(1, Ordering::Relaxed);
    }

    fn decl_queryable(
        &self,
        _reskey: &ResKey,
        _kind: ZInt,
        _routing_context: Option<RoutingContext>,
    ) {
        self.count.fetch_add(1, Ordering::Relaxed);
    }

    fn forget_queryable(&self, _reskey: &ResKey, _routing_context: Option<RoutingContext>) {
        self.count.fetch_add(1, Ordering::Relaxed);
    }

    fn send_data(
        &self,
        _reskey: &ResKey,
        _payload: ZBuf,
        _channel: Channel,
        _data_info: Option<DataInfo>,
        _routing_context: Option<RoutingContext>,
    ) {
        self.count.fetch_add(1, Ordering::Relaxed);
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
        self.count.fetch_add(1, Ordering::Relaxed);
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
        self.count.fetch_add(1, Ordering::Relaxed);
    }

    fn send_reply_final(&self, _qid: ZInt) {
        self.count.fetch_add(1, Ordering::Relaxed);
    }

    fn send_pull(
        &self,
        _is_final: bool,
        _reskey: &ResKey,
        _pull_id: ZInt,
        _max_samples: &Option<ZInt>,
    ) {
        self.count.fetch_add(1, Ordering::Relaxed);
    }

    fn send_close(&self) {
        self.count.fetch_add(1, Ordering::Relaxed);
    }
}

#[derive(Debug, StructOpt)]
#[structopt(name = "r_sub_thr")]
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

    config.insert(ZN_MULTICAST_SCOUTING_KEY, "false".to_string());
    match opt.mode.as_str() {
        "peer" | "router" => {
            config.insert(ZN_LISTENER_KEY, opt.locator);
        }
        "client" => {
            config.insert(ZN_PEER_KEY, opt.locator);
        }
        _ => {
            panic!("Unsupported mode: {}", opt.mode);
        }
    }

    let count = Arc::new(AtomicUsize::new(0));
    let my_primitives = Arc::new(ThroughputPrimitives::new(count.clone()));

    let runtime = Runtime::new(0u8, config, None).await.unwrap();
    let primitives = runtime.router.new_primitives(my_primitives);

    primitives.decl_resource(1, &"/test/thr".to_string().into());

    let rid = ResKey::RId(1);
    let sub_info = SubInfo {
        reliability: Reliability::Reliable,
        mode: SubMode::Push,
        period: None,
    };
    primitives.decl_subscriber(&rid, &sub_info, None);

    loop {
        let now = Instant::now();
        task::sleep(Duration::from_secs(1)).await;
        let elapsed = now.elapsed().as_micros() as f64;

        let c = count.swap(0, Ordering::Relaxed);
        if c > 0 {
            let interval = 1_000_000.0 / elapsed;
            println!(
                "router,{},throughput,{},{},{}",
                opt.scenario,
                opt.name,
                opt.payload,
                (c as f64 / interval).floor() as usize
            );
        }
    }
}
