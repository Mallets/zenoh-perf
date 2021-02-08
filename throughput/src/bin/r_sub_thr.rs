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
use async_std::sync::{Arc, Mutex};
use async_trait::async_trait;
use std::time::Instant;
use structopt::StructOpt;
use zenoh_protocol::core::{
    CongestionControl, PeerId, QueryConsolidation, QueryTarget, Reliability, ResKey, SubInfo,
    SubMode, ZInt,
};
use zenoh_protocol::io::RBuf;
use zenoh_protocol::proto::{DataInfo, RoutingContext};
use zenoh_protocol::session::Primitives;
use zenoh_router::runtime::Runtime;
use zenoh_util::properties::config::{
    ConfigProperties, ZN_LISTENER_KEY, ZN_MODE_KEY, ZN_MULTICAST_SCOUTING_KEY, ZN_PEER_KEY,
};

const N: usize = 100_000;

struct Stats {
    count: usize,
    start: Instant,
}

impl Stats {
    pub fn print(&self) {
        let elapsed = self.start.elapsed().as_secs_f64();
        let thpt = N as f64 / elapsed;
        println!("{} msg/s", thpt);
    }
}

pub struct ThroughputPrimitives {
    stats: Mutex<Stats>,
}

impl ThroughputPrimitives {
    pub fn new() -> ThroughputPrimitives {
        ThroughputPrimitives {
            stats: Mutex::new(Stats {
                count: 0,
                start: Instant::now(),
            }),
        }
    }
}

impl Default for ThroughputPrimitives {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Primitives for ThroughputPrimitives {
    async fn resource(&self, _rid: ZInt, _reskey: &ResKey) {}
    async fn forget_resource(&self, _rid: ZInt) {}
    async fn publisher(&self, _reskey: &ResKey, _routing_context: Option<RoutingContext>) {}
    async fn forget_publisher(&self, _reskey: &ResKey, _routing_context: Option<RoutingContext>) {}
    async fn subscriber(
        &self,
        _reskey: &ResKey,
        _sub_info: &SubInfo,
        _routing_context: Option<RoutingContext>,
    ) {
    }
    async fn forget_subscriber(&self, _reskey: &ResKey, _routing_context: Option<RoutingContext>) {}
    async fn queryable(&self, _reskey: &ResKey, _routing_context: Option<RoutingContext>) {}
    async fn forget_queryable(&self, _reskey: &ResKey, _routing_context: Option<RoutingContext>) {}

    async fn data(
        &self,
        _reskey: &ResKey,
        _payload: RBuf,
        _reliability: Reliability,
        _congestion_control: CongestionControl,
        _data_info: Option<DataInfo>,
        _routing_context: Option<RoutingContext>,
    ) {
        let mut stats = self.stats.lock().await;
        if stats.count == 0 {
            stats.start = Instant::now();
            stats.count += 1;
        } else if stats.count < N {
            stats.count += 1;
        } else {
            stats.print();
            stats.count = 0;
        }
    }

    async fn query(
        &self,
        _reskey: &ResKey,
        _predicate: &str,
        _qid: ZInt,
        _target: QueryTarget,
        _consolidation: QueryConsolidation,
        _routing_context: Option<RoutingContext>,
    ) {
    }
    async fn reply_data(
        &self,
        _qid: ZInt,
        _source_kind: ZInt,
        _replier_id: PeerId,
        _reskey: ResKey,
        _info: Option<DataInfo>,
        _payload: RBuf,
    ) {
    }
    async fn reply_final(&self, _qid: ZInt) {}
    async fn pull(
        &self,
        _is_final: bool,
        _reskey: &ResKey,
        _pull_id: ZInt,
        _max_samples: &Option<ZInt>,
    ) {
    }
    async fn close(&self) {}
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
            "peer" => config.insert(ZN_LISTENER_KEY, opt.locator.unwrap()),
            "client" => config.insert(ZN_PEER_KEY, opt.locator.unwrap()),
            _ => panic!("Unsupported mode: {}", opt.mode),
        };
    }

    let my_primitives = Arc::new(ThroughputPrimitives::new());

    let runtime = Runtime::new(0u8, config, None).await.unwrap();
    // runtime
    //     .read()
    //     .await
    //     .orchestrator
    //     .manager
    //     .add_listener(&opt.locator)
    //     .await
    //     .unwrap();
    let primitives = runtime
        .read()
        .await
        .router
        .new_primitives(my_primitives)
        .await;

    primitives.resource(1, &"/tp".to_string().into()).await;

    let rid = ResKey::RId(1);
    let sub_info = SubInfo {
        reliability: Reliability::Reliable,
        mode: SubMode::Push,
        period: None,
    };
    primitives.subscriber(&rid, &sub_info, None).await;

    // Wait forever
    future::pending::<()>().await;
}
