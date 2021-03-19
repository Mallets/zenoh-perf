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
use async_std::sync::{Arc, Barrier, Mutex};
use async_std::task;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};
use structopt::StructOpt;
use zenoh::net::protocol::core::{
    CongestionControl, PeerId, QueryConsolidation, QueryTarget, Reliability, ResKey, SubInfo, ZInt,
};
use zenoh::net::protocol::io::RBuf;
use zenoh::net::protocol::proto::{DataInfo, RoutingContext};
use zenoh::net::protocol::session::Primitives;
use zenoh::net::routing::OutSession;
use zenoh::net::runtime::Runtime;
use zenoh_util::properties::config::{
    ConfigProperties, ZN_MODE_KEY, ZN_MULTICAST_SCOUTING_KEY, ZN_PEER_KEY,
};

type Pending = Arc<Mutex<HashMap<u64, Arc<Barrier>>>>;

struct QueryPrimitives {
    pending: Pending,
}

impl QueryPrimitives {
    pub fn new(pending: Pending) -> QueryPrimitives {
        QueryPrimitives { pending }
    }
}

#[async_trait]
impl Primitives for QueryPrimitives {
    async fn decl_resource(&self, _rid: ZInt, _reskey: &ResKey) {}
    async fn forget_resource(&self, _rid: ZInt) {}
    async fn decl_publisher(&self, _reskey: &ResKey, _routing_context: Option<RoutingContext>) {}
    async fn forget_publisher(&self, _reskey: &ResKey, _routing_context: Option<RoutingContext>) {}
    async fn decl_subscriber(
        &self,
        _reskey: &ResKey,
        _sub_info: &SubInfo,
        _routing_context: Option<RoutingContext>,
    ) {
    }
    async fn forget_subscriber(&self, _reskey: &ResKey, _routing_context: Option<RoutingContext>) {}
    async fn decl_queryable(&self, _reskey: &ResKey, _routing_context: Option<RoutingContext>) {}
    async fn forget_queryable(&self, _reskey: &ResKey, _routing_context: Option<RoutingContext>) {}

    async fn send_data(
        &self,
        _reskey: &ResKey,
        _payload: RBuf,
        _reliability: Reliability,
        _congestion_control: CongestionControl,
        _data_info: Option<DataInfo>,
        _routing_context: Option<RoutingContext>,
    ) {
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
    }
    async fn send_reply_data(
        &self,
        qid: ZInt,
        _source_kind: ZInt,
        _replier_id: PeerId,
        _reskey: ResKey,
        _info: Option<DataInfo>,
        _payload: RBuf,
    ) {
        let barrier = self.pending.lock().await.remove(&qid).unwrap();
        barrier.wait().await;
    }
    async fn send_reply_final(&self, _qid: ZInt) {}
    async fn send_pull(
        &self,
        _is_final: bool,
        _reskey: &ResKey,
        _pull_id: ZInt,
        _max_samples: &Option<ZInt>,
    ) {
    }
    async fn send_close(&self) {}
}

#[derive(Debug, StructOpt)]
#[structopt(name = "r_query")]
struct Opt {
    #[structopt(short = "e", long = "peer")]
    peer: Option<String>,
    #[structopt(short = "m", long = "mode")]
    mode: String,
    #[structopt(short = "n", long = "name")]
    name: String,
    #[structopt(short = "s", long = "scenario")]
    scenario: String,
    #[structopt(short = "p", long = "payload")]
    payload: usize,
}

#[async_std::main]
async fn main() {
    // Enable logging
    env_logger::init();

    // Parse the args
    let opt = Opt::from_args();

    let mut config = ConfigProperties::default();
    config.insert(ZN_MODE_KEY, opt.mode.clone());

    if opt.peer.is_none() {
        config.insert(ZN_MULTICAST_SCOUTING_KEY, "true".to_string());
    } else {
        config.insert(ZN_MULTICAST_SCOUTING_KEY, "false".to_string());
        config.insert(ZN_PEER_KEY, opt.peer.clone().unwrap());
    }

    let counter = Arc::new(AtomicUsize::new(0));
    let pending: Pending = Arc::new(Mutex::new(HashMap::new()));

    let runtime = Runtime::new(0u8, config, None).await.unwrap();
    let rx_primitives = Arc::new(QueryPrimitives::new(pending.clone()));
    let tx_primitives = runtime
        .read()
        .await
        .router
        .new_primitives(OutSession::Primitives(rx_primitives))
        .await;

    let c_counter = counter.clone();
    task::spawn(async move {
        loop {
            let now = Instant::now();
            task::sleep(Duration::from_secs(1)).await;
            let elapsed = now.elapsed().as_micros() as f64;

            let c = c_counter.swap(0, Ordering::Relaxed);
            if c > 0 {
                let interval = 1_000_000.0 / elapsed;
                println!(
                    "router,{},query.throughput,{},{},{}",
                    opt.scenario,
                    opt.name,
                    opt.payload,
                    (c as f64 / interval).floor() as usize
                );
            }
        }
    });

    let mut count: u64 = 0;
    loop {
        let reskey = ResKey::RName("/test/query".to_string());
        let predicate = "";
        let qid = count;
        let target = QueryTarget::default();
        let consolidation = QueryConsolidation::default();
        let routing_context = None;

        // Insert the pending query
        let barrier = Arc::new(Barrier::new(2));
        pending.lock().await.insert(count, barrier.clone());
        tx_primitives
            .send_query(
                &reskey,
                &predicate,
                qid,
                target.clone(),
                consolidation.clone(),
                routing_context,
            )
            .await;
        // Wait for the reply to arrive
        barrier.wait().await;
        counter.fetch_add(1, Ordering::Relaxed);

        count += 1;
    }
}
