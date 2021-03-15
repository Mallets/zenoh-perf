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
use async_trait::async_trait;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;
use structopt::StructOpt;
use zenoh::net::protocol::core::{
    CongestionControl, PeerId, QueryConsolidation, QueryTarget, Reliability, ResKey, SubInfo,
    SubMode, ZInt,
};
use zenoh::net::protocol::io::RBuf;
use zenoh::net::protocol::proto::{DataInfo, RoutingContext};
use zenoh::net::protocol::session::Primitives;
use zenoh::net::routing::OutSession;
use zenoh::net::runtime::Runtime;
use zenoh_util::properties::config::{
    ConfigProperties, ZN_LISTENER_KEY, ZN_MODE_KEY, ZN_MULTICAST_SCOUTING_KEY, ZN_PEER_KEY,
};

struct ThroughputPrimitives {
    count: Arc<AtomicUsize>,
}

impl ThroughputPrimitives {
    pub fn new(count: Arc<AtomicUsize>) -> ThroughputPrimitives {
        ThroughputPrimitives { count }
    }
}

#[async_trait]
impl Primitives for ThroughputPrimitives {
    async fn decl_resource(&self, _rid: ZInt, _reskey: &ResKey) {
        self.count.fetch_add(1, Ordering::Relaxed);
    }

    async fn forget_resource(&self, _rid: ZInt) {
        self.count.fetch_add(1, Ordering::Relaxed);
    }

    async fn decl_publisher(&self, _reskey: &ResKey, _routing_context: Option<RoutingContext>) {
        self.count.fetch_add(1, Ordering::Relaxed);
    }

    async fn forget_publisher(&self, _reskey: &ResKey, _routing_context: Option<RoutingContext>) {
        self.count.fetch_add(1, Ordering::Relaxed);
    }

    async fn decl_subscriber(
        &self,
        _reskey: &ResKey,
        _sub_info: &SubInfo,
        _routing_context: Option<RoutingContext>,
    ) {
        self.count.fetch_add(1, Ordering::Relaxed);
    }

    async fn forget_subscriber(&self, _reskey: &ResKey, _routing_context: Option<RoutingContext>) {
        self.count.fetch_add(1, Ordering::Relaxed);
    }

    async fn decl_queryable(&self, _reskey: &ResKey, _routing_context: Option<RoutingContext>) {
        self.count.fetch_add(1, Ordering::Relaxed);
    }

    async fn forget_queryable(&self, _reskey: &ResKey, _routing_context: Option<RoutingContext>) {
        self.count.fetch_add(1, Ordering::Relaxed);
    }

    async fn send_data(
        &self,
        _reskey: &ResKey,
        _payload: RBuf,
        _reliability: Reliability,
        _congestion_control: CongestionControl,
        _data_info: Option<DataInfo>,
        _routing_context: Option<RoutingContext>,
    ) {
        self.count.fetch_add(1, Ordering::Relaxed);
    }

    async fn send_query(
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

    async fn send_reply_data(
        &self,
        _qid: ZInt,
        _source_kind: ZInt,
        _replier_id: PeerId,
        _reskey: ResKey,
        _info: Option<DataInfo>,
        _payload: RBuf,
    ) {
        self.count.fetch_add(1, Ordering::Relaxed);
    }

    async fn send_reply_final(&self, _qid: ZInt) {
        self.count.fetch_add(1, Ordering::Relaxed);
    }

    async fn send_pull(
        &self,
        _is_final: bool,
        _reskey: &ResKey,
        _pull_id: ZInt,
        _max_samples: &Option<ZInt>,
    ) {
        self.count.fetch_add(1, Ordering::Relaxed);
    }

    async fn send_close(&self) {
        self.count.fetch_add(1, Ordering::Relaxed);
    }
}

#[derive(Debug, StructOpt)]
#[structopt(name = "r_sub_thr")]
struct Opt {
    #[structopt(short = "l", long = "locator")]
    locator: Option<String>,
    #[structopt(short = "m", long = "mode")]
    mode: String,
    #[structopt(short = "s", long = "scout")]
    scout: bool,
    #[structopt(short = "p", long = "payload")]
    payload: usize,
    #[structopt(short = "n", long = "name")]
    name: String,
    #[structopt(short = "s", long = "scenario")]
    scenario: String,
}

#[async_std::main]
async fn main() {
    // Enable logging
    env_logger::init();

    // Parse the args
    let opt = Opt::from_args();

    let mut config = ConfigProperties::default();
    config.insert(ZN_MODE_KEY, opt.mode.clone());

    if opt.scout {
        config.insert(ZN_MULTICAST_SCOUTING_KEY, "true".to_string());
    } else {
        config.insert(ZN_MULTICAST_SCOUTING_KEY, "false".to_string());
        match opt.mode.as_str() {
            "peer" | "router" => config.insert(ZN_LISTENER_KEY, opt.locator.unwrap()),
            "client" => config.insert(ZN_PEER_KEY, opt.locator.unwrap()),
            _ => panic!("Unsupported mode: {}", opt.mode),
        };
    }

    let count = Arc::new(AtomicUsize::new(0));
    let my_primitives = Arc::new(ThroughputPrimitives::new(count.clone()));

    let runtime = Runtime::new(0u8, config, None).await.unwrap();
    let primitives = runtime
        .read()
        .await
        .router
        .new_primitives(OutSession::Primitives(my_primitives))
        .await;

    primitives.decl_resource(1, &"/tp".to_string().into()).await;

    let rid = ResKey::RId(1);
    let sub_info = SubInfo {
        reliability: Reliability::Reliable,
        mode: SubMode::Push,
        period: None,
    };
    primitives.decl_subscriber(&rid, &sub_info, None).await;

    loop {
        task::sleep(Duration::from_secs(1)).await;
        let c = count.swap(0, Ordering::Relaxed);
        if c > 0 {
            println!(
                "router,{},throughput,{},{},{}",
                opt.scenario, opt.name, opt.payload, c
            );
        }
    }
}
